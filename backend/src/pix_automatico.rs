use crate::{
    App,
    api::{Error, Result},
    auth::{self, Identity},
};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx_core::{query::query, query_as::query_as};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAuthorization {
    nome_pagador: String,
    objeto: String,
    contrato: String,
    periodicidade: String,
    data_inicial: Date,
    data_final: Option<Date>,
    valor_centavos: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    id: Uuid,
    aceitar: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCharge {
    authorization_id: Uuid,
    competencia: Date,
    valor_centavos: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settle {
    id: Uuid,
}

#[derive(Serialize)]
pub struct Authorization {
    id: Uuid,
    nome_pagador: String,
    objeto: String,
    contrato: String,
    periodicidade: String,
    data_inicial: Date,
    data_final: Option<Date>,
    valor_centavos: String,
    status: String,
    solicitacao_id: Uuid,
}

#[derive(Serialize)]
pub struct Charge {
    id: Uuid,
    authorization_id: Uuid,
    competencia: Date,
    valor_centavos: String,
    status: String,
    end_to_end_id: Option<String>,
}

async fn user(app: &App, headers: &HeaderMap, write: bool) -> Result<Identity> {
    if app.bank.environment() != "MOCK" {
        return Err(Error::Missing);
    }
    let identity = auth::identity(app, headers).await?;
    let allowed = if write {
        identity
            .roles
            .iter()
            .any(|r| matches!(r.as_str(), "ADMIN" | "OPERATOR"))
    } else {
        identity.roles.iter().any(|r| {
            matches!(
                r.as_str(),
                "ADMIN" | "OPERATOR" | "VIEWER" | "APPROVER" | "AUDITOR"
            )
        })
    };
    if !allowed {
        return Err(Error::Forbidden);
    }
    Ok(identity)
}

fn invalid() -> Error {
    Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY)
}

fn valid_amount(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 12
        && value.bytes().all(|b| b.is_ascii_digit())
        && !value.starts_with('0')
        && value.parse::<i64>().is_ok_and(|n| n > 0)
}

pub async fn list(State(app): State<App>, headers: HeaderMap) -> Result<Json<Vec<Authorization>>> {
    let identity = user(&app, &headers, false).await?;
    let rows = query_as::<_, (Uuid,String,String,String,String,Date,Option<Date>,i64,String,Uuid)>(
        "SELECT id,nome_pagador,objeto,contrato,periodicidade,data_inicial,data_final,valor_centavos,status::text,solicitacao_id FROM mock_pix_recurring_authorizations WHERE company_id=$1 ORDER BY created_at DESC LIMIT 200"
    ).bind(identity.company_id).fetch_all(&app.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Authorization {
                id: r.0,
                nome_pagador: r.1,
                objeto: r.2,
                contrato: r.3,
                periodicidade: r.4,
                data_inicial: r.5,
                data_final: r.6,
                valor_centavos: r.7.to_string(),
                status: r.8,
                solicitacao_id: r.9,
            })
            .collect(),
    ))
}

pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<NewAuthorization>, JsonRejection>,
) -> Result<(StatusCode, Json<Authorization>)> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    if input.nome_pagador.trim().is_empty()
        || input.nome_pagador.trim().len() > 120
        || input.objeto.trim().is_empty()
        || input.objeto.trim().len() > 35
        || input.contrato.trim().is_empty()
        || input.contrato.trim().len() > 35
        || !matches!(
            input.periodicidade.as_str(),
            "SEMANAL" | "MENSAL" | "TRIMESTRAL" | "SEMESTRAL" | "ANUAL"
        )
        || !valid_amount(&input.valor_centavos)
        || input.data_inicial < OffsetDateTime::now_utc().date()
        || input.data_final.is_some_and(|d| d < input.data_inicial)
    {
        return Err(invalid());
    }
    let id = Uuid::new_v4();
    let solicitud = Uuid::new_v4();
    let cents = input.valor_centavos.parse::<i64>().map_err(|_| invalid())?;
    let mut tx = app.db.begin().await?;
    let row=query_as::<_,(Uuid,String,String,String,String,Date,Option<Date>,i64,String,Uuid)>("INSERT INTO mock_pix_recurring_authorizations(id,company_id,creator_id,nome_pagador,objeto,contrato,periodicidade,data_inicial,data_final,valor_centavos,solicitacao_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id,nome_pagador,objeto,contrato,periodicidade,data_inicial,data_final,valor_centavos,status::text,solicitacao_id")
        .bind(id).bind(identity.company_id).bind(identity.id).bind(input.nome_pagador.trim()).bind(input.objeto.trim()).bind(input.contrato.trim()).bind(input.periodicidade).bind(input.data_inicial).bind(input.data_final).bind(cents).bind(solicitud).fetch_one(&mut *tx).await?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Solicitação Pix Automático MOCK criada; aguardando consentimento fictício')").bind(identity.company_id).bind(identity.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Authorization {
            id: row.0,
            nome_pagador: row.1,
            objeto: row.2,
            contrato: row.3,
            periodicidade: row.4,
            data_inicial: row.5,
            data_final: row.6,
            valor_centavos: row.7.to_string(),
            status: row.8,
            solicitacao_id: row.9,
        }),
    ))
}

pub async fn decide(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Decision>, JsonRejection>,
) -> Result<Json<Authorization>> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let mut tx = app.db.begin().await?;
    let row=query_as::<_,(Uuid,String,String,String,String,Date,Option<Date>,i64,String,Uuid)>("UPDATE mock_pix_recurring_authorizations SET status=CASE WHEN $3 THEN 'ATIVA'::mock_rec_status ELSE 'RECUSADA'::mock_rec_status END,updated_at=now() WHERE company_id=$1 AND id=$2 AND status='AGUARDANDO_CONSENTIMENTO' RETURNING id,nome_pagador,objeto,contrato,periodicidade,data_inicial,data_final,valor_centavos,status::text,solicitacao_id").bind(identity.company_id).bind(input.id).bind(input.aceitar).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(if input.aceitar {
            "Consentimento Pix Automático fictício aceito"
        } else {
            "Consentimento Pix Automático fictício recusado"
        })
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(Authorization {
        id: row.0,
        nome_pagador: row.1,
        objeto: row.2,
        contrato: row.3,
        periodicidade: row.4,
        data_inicial: row.5,
        data_final: row.6,
        valor_centavos: row.7.to_string(),
        status: row.8,
        solicitacao_id: row.9,
    }))
}

pub async fn charges(State(app): State<App>, headers: HeaderMap) -> Result<Json<Vec<Charge>>> {
    let identity = user(&app, &headers, false).await?;
    let rows=query_as::<_,(Uuid,Uuid,Date,i64,String,Option<String>)>("SELECT id,authorization_id,competencia,valor_centavos,status::text,end_to_end_id FROM mock_pix_recurring_charges WHERE company_id=$1 ORDER BY competencia DESC,created_at DESC LIMIT 300").bind(identity.company_id).fetch_all(&app.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Charge {
                id: r.0,
                authorization_id: r.1,
                competencia: r.2,
                valor_centavos: r.3.to_string(),
                status: r.4,
                end_to_end_id: r.5,
            })
            .collect(),
    ))
}

pub async fn create_charge(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<NewCharge>, JsonRejection>,
) -> Result<(StatusCode, Json<Charge>)> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    if !valid_amount(&input.valor_centavos) {
        return Err(invalid());
    }
    let cents = input.valor_centavos.parse::<i64>().map_err(|_| invalid())?;
    let mut tx = app.db.begin().await?;
    let auth=query("SELECT status::text,data_inicial,data_final,valor_centavos FROM mock_pix_recurring_authorizations WHERE company_id=$1 AND id=$2 FOR UPDATE").bind(identity.company_id).bind(input.authorization_id).fetch_optional(&mut *tx).await?.ok_or(Error::Missing)?;
    let status: String = sqlx_core::row::Row::get(&auth, "status");
    let start: Date = sqlx_core::row::Row::get(&auth, "data_inicial");
    let end: Option<Date> = sqlx_core::row::Row::get(&auth, "data_final");
    let expected: i64 = sqlx_core::row::Row::get(&auth, "valor_centavos");
    if status != "ATIVA"
        || input.competencia < start
        || end.is_some_and(|d| input.competencia > d)
        || cents != expected
    {
        return Err(Error::Conflict);
    }
    let id = Uuid::new_v4();
    let row=query_as::<_,(Uuid,Uuid,Date,i64,String,Option<String>)>("INSERT INTO mock_pix_recurring_charges(id,company_id,authorization_id,competencia,valor_centavos) VALUES($1,$2,$3,$4,$5) ON CONFLICT(company_id,authorization_id,competencia) DO NOTHING RETURNING id,authorization_id,competencia,valor_centavos,status::text,end_to_end_id").bind(id).bind(identity.company_id).bind(input.authorization_id).bind(input.competencia).bind(cents).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Cobrança Pix Automático fictícia criada')").bind(identity.company_id).bind(identity.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Charge {
            id: row.0,
            authorization_id: row.1,
            competencia: row.2,
            valor_centavos: row.3.to_string(),
            status: row.4,
            end_to_end_id: row.5,
        }),
    ))
}

pub async fn settle_charge(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Settle>, JsonRejection>,
) -> Result<Json<Charge>> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let mut tx = app.db.begin().await?;
    let e2e = format!("E{}", Uuid::new_v4().simple());
    let row=query_as::<_,(Uuid,Uuid,Date,i64,String,Option<String>)>("UPDATE mock_pix_recurring_charges SET status='LIQUIDADA',end_to_end_id=$3,settled_at=now() WHERE company_id=$1 AND id=$2 AND status='CRIADA' RETURNING id,authorization_id,competencia,valor_centavos,status::text,end_to_end_id").bind(identity.company_id).bind(input.id).bind(&e2e).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Liquidação de cobrança Pix Automático simulada em MOCK; nenhuma transação bancária enviada')").bind(identity.company_id).bind(identity.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(Charge {
        id: row.0,
        authorization_id: row.1,
        competencia: row.2,
        valor_centavos: row.3.to_string(),
        status: row.4,
        end_to_end_id: row.5,
    }))
}

#[cfg(test)]
mod tests {
    use super::valid_amount;
    #[test]
    fn centavos_exatos_com_validacao_estrita() {
        assert!(valid_amount("1"));
        assert!(valid_amount("999999999999"));
        assert!(!valid_amount("0"));
        assert!(!valid_amount("01"));
        assert!(!valid_amount("1.00"));
        assert!(!valid_amount("1000000000000"));
    }
}
