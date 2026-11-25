use super::{BankError, InterBank, LiveDashboard, today};
use crate::domain::{Posted, Reading, Statement, StatementQuery, StatementSummary};
use sqlx_core::transaction::Transaction;
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row};
use sqlx_postgres::{PgRow, Postgres};
use time::{Date, Duration as CalendarDuration, OffsetDateTime};
use uuid::Uuid;

impl InterBank {
    pub(super) async fn dashboard(&self) -> Result<LiveDashboard, BankError> {
        let row=query("SELECT balance_json,imported_from,imported_until,synchronized_at FROM inter_read_state WHERE account_id=$1")
       .bind(self.account).fetch_optional(&self.db).await?;
        let reading = row
            .map(|row| {
                Ok::<_, BankError>(Reading {
                    balance: serde_json::from_str(row.get("balance_json"))
                        .map_err(|_| BankError::Contract)?,
                    imported_from: row.get("imported_from"),
                    imported_until: row.get("imported_until"),
                    synchronized_at: row.get("synchronized_at"),
                })
            })
            .transpose()?;
        let last_error=query_scalar::<_,Option<String>>("SELECT error_code FROM integration_sync_jobs WHERE company_id=$1 AND account_id=$2 ORDER BY created_at DESC LIMIT 1")
        .bind(self.company).bind(self.account).fetch_optional(&self.db).await?.flatten();
        Ok(LiveDashboard {
            payment_preparation: crate::config::payment_preparation(self.mode),
            pix_lookup_enabled: self.pix_lookup_enabled,
            source: self.mode,
            certificate_expires_at: self.expires,
            reading,
            last_error,
        })
    }
    pub(super) async fn list(&self, filter: StatementQuery) -> Result<Statement, BankError> {
        let end = filter.fim.unwrap_or_else(today);
        let begin = filter
            .inicio
            .or_else(|| end.checked_sub(CalendarDuration::days(29)))
            .ok_or(BankError::Period)?;
        if !(0..=89).contains(&(end - begin).whole_days()) || end > today() {
            return Err(BankError::Period);
        }
        let offset = i64::from(filter.pagina.unwrap_or(0)) * 50;
        let totals = query("SELECT COUNT(*) AS total_entries,
          COALESCE(SUM(amount_cents) FILTER (WHERE amount_cents > 0), 0)::text AS incoming_cents,
          COALESCE(SUM(amount_cents) FILTER (WHERE amount_cents < 0), 0)::text AS outgoing_cents,
          COALESCE(SUM(amount_cents), 0)::text AS net_cents
          FROM bank_transactions
          WHERE company_id=$1 AND account_id=$2 AND inter_current AND inter_kind IS NOT NULL
          AND posted_at>=($3::date::timestamp AT TIME ZONE 'UTC') AND posted_at<(($4::date+1)::timestamp AT TIME ZONE 'UTC')")
            .bind(self.company).bind(self.account).bind(begin).bind(end)
            .fetch_one(&self.db).await?;
        let total_entries: i64 = totals.get("total_entries");
        let incoming_cents: String = totals.get("incoming_cents");
        let outgoing_cents: String = totals.get("outgoing_cents");
        let net_cents: String = totals.get("net_cents");
        let mut entries=query("SELECT provider_reference,(posted_at AT TIME ZONE 'UTC')::date AS date,
          inter_included_on,inter_kind,inter_operation,inter_title,inter_description,inter_document_number,amount_cents
          FROM bank_transactions
       WHERE company_id=$1 AND account_id=$2 AND inter_current AND inter_kind IS NOT NULL
       AND posted_at>=($3::date::timestamp AT TIME ZONE 'UTC') AND posted_at<(($4::date+1)::timestamp AT TIME ZONE 'UTC')
       ORDER BY posted_at DESC, provider_reference LIMIT 51 OFFSET $5")
      .bind(self.company).bind(self.account).bind(begin).bind(end).bind(offset)
      .map(|r:PgRow|Posted{
          transaction_id:r.get("provider_reference"), date:r.get("date"), included_on:r.get("inter_included_on"),
          kind:r.get("inter_kind"), operation:r.get::<Option<String>,_>("inter_operation").unwrap_or_else(|| if r.get::<i64,_>("amount_cents") >= 0 { "C".into() } else { "D".into() }),
          title:r.get("inter_title"), description:r.get("inter_description"), document_number:r.get("inter_document_number"),
          cents:r.get::<i64,_>("amount_cents").to_string()
      }).fetch_all(&self.db).await?;
        let has_more = entries.len() > 50;
        entries.truncate(50);
        Ok(Statement {
            entries,
            has_more,
            summary: StatementSummary {
                total_entries,
                incoming_cents,
                outgoing_cents,
                net_cents,
            },
        })
    }
    pub(super) async fn run_sync(&self, actor: Uuid) -> Result<(), BankError> {
        let mut tx = self.db.begin().await?;
        let locked=query("SELECT id FROM bank_integrations WHERE company_id=$1 AND account_id=$2 FOR UPDATE NOWAIT")
       .bind(self.company).bind(self.account).fetch_one(&mut *tx).await;
        if let Err(sqlx_core::Error::Database(ref e)) = locked
            && e.code().as_deref() == Some("55P03")
        {
            return Err(BankError::Busy);
        }
        locked?;
        query("UPDATE integration_sync_jobs SET state='FALHOU',finished_at=now(),error_code='INTERROMPIDO'
           WHERE account_id=$1 AND state='PROCESSANDO'").bind(self.account).execute(&self.db).await?;
        let job = Uuid::new_v4();
        query("INSERT INTO integration_sync_jobs(id,company_id,account_id,state) VALUES($1,$2,$3,'PROCESSANDO')")
        .bind(job).bind(self.company).bind(self.account).execute(&self.db).await?;
        let run = tokio::time::timeout(
            std::time::Duration::from_secs(240),
            self.import(&mut tx, job, actor),
        )
        .await
        .unwrap_or(Err(BankError::Timeout));
        match run {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(error) => {
                tx.rollback().await?;
                query("UPDATE integration_sync_jobs SET state='FALHOU',finished_at=clock_timestamp(),error_code=$2 WHERE id=$1 AND state='PROCESSANDO'")
            .bind(job).bind(error.code()).execute(&self.db).await?;
                tracing::warn!(sync_id=%job,classe=error.code(),"Falha na sincronização");
                Err(error)
            }
        }
    }
    async fn import(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        job: Uuid,
        actor: Uuid,
    ) -> Result<(), BankError> {
        if self.expires <= OffsetDateTime::now_utc() {
            return Err(BankError::Certificate);
        }
        let now = today();
        let checkpoint: Option<Date> =
            query_scalar("SELECT imported_until FROM inter_read_state WHERE account_id=$1")
                .bind(self.account)
                .fetch_optional(&mut **tx)
                .await?;
        let begin = checkpoint
            .map(|date| date - CalendarDuration::days(2))
            .unwrap_or(now - CalendarDuration::days(29));
        let end = now.min(begin + CalendarDuration::days(89));
        if begin > end {
            return Err(BankError::Period);
        }
        let entries = self.client.statement(begin, end).await?;
        let balance = self.client.balance().await?;
        // Somente os registros deste adaptador no intervalo verificado são substituídos.
        // A identidade local e a categorização permanecem estáveis em novas leituras.
        query("UPDATE bank_transactions SET inter_current=false WHERE account_id=$1 AND inter_kind IS NOT NULL
       AND (posted_at AT TIME ZONE 'UTC')::date BETWEEN $2 AND $3")
       .bind(self.account).bind(begin).bind(end).execute(&mut **tx).await?;
        for chunk in entries.chunks(500) {
            let mut ids = Vec::new();
            let mut refs = Vec::new();
            let mut dates = Vec::new();
            let mut kinds = Vec::new();
            let mut amounts = Vec::new();
            let mut operations = Vec::new();
            let mut titles = Vec::new();
            let mut descriptions = Vec::new();
            let mut documents = Vec::new();
            let mut included_on = Vec::new();
            for e in chunk {
                ids.push(Uuid::new_v4());
                refs.push(e.id_transacao.as_str());
                dates.push(e.data_transacao);
                kinds.push(e.tipo_transacao.as_str());
                amounts.push(e.signed_cents()?);
                operations.push(e.operation());
                titles.push(e.titulo.as_deref());
                descriptions.push(e.descricao.as_deref());
                documents.push(e.numero_documento.as_deref());
                included_on.push(e.data_inclusao);
            }
            query("INSERT INTO bank_transactions(id,company_id,account_id,provider_reference,posted_at,inter_kind,inter_operation,inter_title,inter_description,inter_document_number,inter_included_on,amount_cents)
          SELECT t.id,$1,$2,t.ref,t.day::timestamp AT TIME ZONE 'UTC',t.kind,t.operation,t.title,t.description,t.document,t.included_on,t.amount
          FROM unnest($3::uuid[],$4::text[],$5::date[],$6::text[],$7::text[],$8::text[],$9::text[],$10::text[],$11::date[],$12::bigint[]) AS t(id,ref,day,kind,operation,title,description,document,included_on,amount)
          ON CONFLICT(account_id,provider_reference) DO UPDATE SET posted_at=excluded.posted_at,inter_kind=excluded.inter_kind,
          inter_operation=excluded.inter_operation,inter_title=excluded.inter_title,inter_description=excluded.inter_description,
          inter_document_number=excluded.inter_document_number,inter_included_on=excluded.inter_included_on,
          amount_cents=excluded.amount_cents,inter_current=true")
          .bind(self.company).bind(self.account).bind(ids).bind(refs).bind(dates).bind(kinds).bind(operations)
          .bind(titles).bind(descriptions).bind(documents).bind(included_on).bind(amounts).execute(&mut **tx).await?;
        }
        query("INSERT INTO inter_read_state(account_id,imported_from,imported_until,balance_json,synchronized_at) VALUES($1,$2,$3,$4,clock_timestamp())
      ON CONFLICT(account_id) DO UPDATE SET imported_from=least(inter_read_state.imported_from,excluded.imported_from),imported_until=excluded.imported_until,balance_json=excluded.balance_json,synchronized_at=excluded.synchronized_at")
      .bind(self.account).bind(begin).bind(end).bind(serde_json::to_string(&balance).map_err(|_|BankError::Contract)?).execute(&mut **tx).await?;
        query("UPDATE bank_integrations SET scopes=ARRAY['extrato.read'] WHERE account_id=$1")
            .bind(self.account)
            .execute(&mut **tx)
            .await?;
        query("UPDATE integration_sync_jobs SET state='CONCLUIDO',finished_at=clock_timestamp() WHERE id=$1").bind(job).execute(&mut **tx).await?;
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Saldo e extrato sincronizados')").bind(self.company).bind(actor).execute(&mut **tx).await?;
        Ok(())
    }
}
