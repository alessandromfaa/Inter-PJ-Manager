use super::{Confirmation, Scope};
use crate::{App, infrastructure::banking::inter::BankError};
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row, transaction::Transaction};
use sqlx_postgres::Postgres;
use std::time::Duration;
use uuid::Uuid;

async fn mock_confirmation(
    tx: &mut Transaction<'_, Postgres>,
    scope: Scope,
    family: &str,
    reference: &str,
) -> Result<(Confirmation, &'static str), BankError> {
    if family == "PIX" {
        let row=query("SELECT r.txid,r.cents,c.original_cents FROM mock_pix_receipts r JOIN mock_pix_charges c USING(company_id,txid)
            WHERE r.company_id=$1 AND r.e2e=$2").bind(scope.company).bind(reference).fetch_optional(&mut **tx).await?.ok_or(BankError::PixNotFound)?;
        let received: i64 = row.get("cents");
        let gross: i64 = row.get("original_cents");
        return Ok((
            Confirmation {
                txid: Some(row.get("txid")),
                status: "RECEBIDO".into(),
                gross,
                received,
                channel: Some("PIX".into()),
            },
            if gross == received {
                "CONCILIADO"
            } else {
                "DIVERGENTE"
            },
        ));
    }
    let code = Uuid::parse_str(reference).map_err(|_| BankError::Contract)?;
    let row=query("UPDATE mock_billing_requests SET status=CASE WHEN status='EM_PROCESSAMENTO' THEN 'A_RECEBER' ELSE status END
        WHERE company_id=$1 AND code=$2 RETURNING cents,status,channel").bind(scope.company).bind(code).fetch_optional(&mut **tx).await?.ok_or(BankError::Contract)?;
    let status: String = row.get("status");
    let gross: i64 = row.get("cents");
    let paid = status == "RECEBIDO";
    Ok((
        Confirmation {
            txid: None,
            status,
            gross,
            received: if paid { gross } else { 0 },
            channel: row.get("channel"),
        },
        if paid { "CONCILIADO" } else { "PENDENTE" },
    ))
}

pub async fn process_one(app: &App) -> Result<bool, BankError> {
    let mut tx = app.db.begin().await?;
    let mode = app.bank.environment();
    let binding = app.bank.collection_binding();
    let row = query(
        "SELECT id,company_id,account_id,family,reference FROM collection_events
        WHERE environment=$1::text::integration_environment AND account_id IS NOT DISTINCT FROM $2
        AND ($3::uuid IS NULL OR company_id=$3) AND state='AGUARDANDO' AND next_at<=now()
        ORDER BY next_at,id LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .bind(mode)
    .bind(binding.map(|b| b.1))
    .bind(binding.map(|b| b.0))
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(false);
    };
    let scope = Scope {
        company: row.get("company_id"),
        account: row.get("account_id"),
        mode,
    };
    let id: i64 = row.get("id");
    let family: String = row.get("family");
    let reference: String = row.get("reference");
    // Serializa consultas ao mesmo recurso entre instâncias, inclusive callbacks fora de ordem.
    query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "{}:{:?}:{family}:{reference}",
            scope.company, scope.account
        ))
        .execute(&mut *tx)
        .await?;
    let confirmed = if mode == "MOCK" {
        mock_confirmation(&mut tx, scope, &family, &reference).await
    } else {
        match app
            .bank
            .confirm_receipt(scope.company, &family, &reference)
            .await
        {
            Ok(c) => {
                let expected: Option<i64> = if family == "PIX" {
                    query_scalar("SELECT amount_cents FROM pix_charges WHERE company_id=$1 AND account_id=$2 AND provider_reference=$3")
                        .bind(scope.company).bind(scope.account).bind(c.txid.as_deref()).fetch_optional(&mut *tx).await?
                } else {
                    query_scalar("SELECT amount_cents FROM billing_charges WHERE company_id=$1 AND account_id=$2 AND provider_reference=$3")
                        .bind(scope.company).bind(scope.account).bind(&reference).fetch_optional(&mut *tx).await?
                };
                let result = match expected {
                    Some(v) if c.status == "RECEBIDO" && v == c.received => "CONCILIADO",
                    Some(_) if c.status == "RECEBIDO" => "DIVERGENTE",
                    _ => "PENDENTE",
                };
                Ok((c, result))
            }
            Err(e) => Err(e),
        }
    };
    match confirmed {
        Ok((c, reconciliation)) => {
            query("INSERT INTO collection_results(company_id,environment,account_id,family,reference,txid,status,gross_cents,received_cents,channel,reconciliation)
                VALUES($1,$2::text::integration_environment,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                ON CONFLICT(company_id,environment,account_id,family,reference) DO UPDATE SET
                txid=excluded.txid,status=excluded.status,gross_cents=excluded.gross_cents,received_cents=excluded.received_cents,
                channel=excluded.channel,reconciliation=excluded.reconciliation,checked_at=clock_timestamp()")
                .bind(scope.company).bind(mode).bind(scope.account).bind(family).bind(reference).bind(c.txid).bind(c.status)
                .bind(c.gross).bind(c.received).bind(c.channel).bind(reconciliation).execute(&mut *tx).await?;
            query("UPDATE collection_events SET state='CONCLUIDO',attempts=attempts+1,error_code=NULL WHERE id=$1")
                .bind(id).execute(&mut *tx).await?;
        }
        Err(BankError::Database(err)) => return Err(BankError::Database(err)),
        Err(e) => {
            let state: String = query_scalar(
                "UPDATE collection_events SET attempts=attempts+1,
                state=CASE WHEN attempts>=4 THEN 'ESGOTADO' ELSE 'AGUARDANDO' END::job_state,
                error_code=$2,next_at=clock_timestamp()+interval '5 seconds'*(1<<least(attempts,6))
                WHERE id=$1 RETURNING state::text",
            )
            .bind(id)
            .bind(e.code())
            .fetch_one(&mut *tx)
            .await?;
            tracing::warn!(
                classe = e.code(),
                estado = state,
                "Falha ao confirmar notificação"
            );
        }
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn run(app: App) {
    loop {
        match process_one(&app).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(err) => tracing::error!(classe = err.code(), "Falha na fila de recebimentos"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
