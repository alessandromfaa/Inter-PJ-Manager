use super::{Scope, enqueue};
use crate::{
    App,
    api::{Error, Result},
    auth::{self, Identity},
    domain::pix::Txid,
};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row, transaction::Transaction};
use sqlx_postgres::Postgres;
use uuid::Uuid;

async fn session(a: &App, h: &HeaderMap, write: bool) -> Result<(Identity, Scope)> {
    let user = auth::identity(a, h).await?;
    let allowed = user.roles.iter().any(|r| match r.as_str() {
        "ADMIN" | "OPERATOR" => true,
        "AUDITOR" | "VIEWER" => !write,
        _ => false,
    });
    if !allowed {
        return Err(Error::Forbidden);
    }
    let binding = a.bank.collection_binding();
    if binding.is_some_and(|b| b.0 != user.company_id) {
        return Err(Error::Forbidden);
    }
    let scope = Scope {
        company: user.company_id,
        account: binding.map(|b| b.1),
        mode: a.bank.environment(),
    };
    Ok((user, scope))
}
fn invalid() -> Error {
    Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY)
}
async fn audit(tx: &mut Transaction<'_, Postgres>, u: &Identity, action: &str) -> Result<()> {
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(u.company_id)
        .bind(u.id)
        .bind(action)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    id: Uuid,
    name: String,
}
pub async fn customer(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<Customer>,
) -> Result<StatusCode> {
    let (u, scope) = session(&a, &h, true).await?;
    let name = i.name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err(invalid());
    }
    let mut tx = a.db.begin().await?;
    let added = query(
        "INSERT INTO customers(id,company_id,name) VALUES($1,$2,$3) ON CONFLICT(id) DO NOTHING",
    )
    .bind(i.id)
    .bind(scope.company)
    .bind(name)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if added == 0 {
        let same: bool = query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE company_id=$1 AND id=$2 AND name=$3)",
        )
        .bind(scope.company)
        .bind(i.id)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        if !same {
            return Err(Error::Conflict);
        }
    } else {
        audit(&mut tx, &u, "Cliente cadastrado localmente").await?
    }
    tx.commit().await?;
    Ok(if added == 1 {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BillingIntent {
    intent_id: Uuid,
    customer_id: Uuid,
    original_cents: String,
    due_on: time::Date,
}
#[derive(Serialize)]
pub struct Accepted {
    pub codigo_solicitacao: Uuid,
    pub situacao: String,
}
pub async fn billing(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<BillingIntent>,
) -> Result<(StatusCode, Json<Accepted>)> {
    let (u, scope) = session(&a, &h, true).await?;
    let cents = crate::api::positive_cents(&i.original_cents)?;
    if !(250..=9999999999).contains(&cents) {
        return Err(invalid());
    }
    let mut tx = a.db.begin().await?;
    let customer: bool =
        query_scalar("SELECT EXISTS(SELECT 1 FROM customers WHERE company_id=$1 AND id=$2)")
            .bind(scope.company)
            .bind(i.customer_id)
            .fetch_one(&mut *tx)
            .await?;
    if !customer {
        return Err(Error::Missing);
    }
    let inserted=query("INSERT INTO mock_billing_requests(company_id,code,customer_id,cents,due_on) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
        .bind(scope.company).bind(i.intent_id).bind(i.customer_id).bind(cents).bind(i.due_on).execute(&mut *tx).await?.rows_affected()==1;
    if inserted {
        let today = time::OffsetDateTime::now_utc()
            .to_offset(time::macros::offset!(-3))
            .date();
        if i.due_on < today {
            return Err(invalid());
        }
        enqueue(
            &mut tx,
            scope,
            "V3",
            &i.intent_id.to_string(),
            &format!("emitir:{}", i.intent_id),
        )
        .await
        .map_err(Error::Bank)?;
        audit(&mut tx, &u, "Solicitada emissão de Cobrança V3 em MOCK").await?;
    }
    let status:Option<String>=query_scalar("SELECT status FROM mock_billing_requests WHERE company_id=$1 AND code=$2 AND customer_id=$3 AND cents=$4 AND due_on=$5")
       .bind(scope.company).bind(i.intent_id).bind(i.customer_id).bind(cents).bind(i.due_on).fetch_optional(&mut *tx).await?;
    let status = status.ok_or(Error::Conflict)?;
    tx.commit().await?;
    Ok((
        if inserted {
            StatusCode::ACCEPTED
        } else {
            StatusCode::OK
        },
        Json(Accepted {
            codigo_solicitacao: i.intent_id,
            situacao: status,
        }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatePix {
    txid: Txid,
}
#[derive(Serialize)]
pub struct Simulated {
    end_to_end_id: String,
}
pub async fn simulate_pix(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<SimulatePix>,
) -> Result<(StatusCode, Json<Simulated>)> {
    let (u, scope) = session(&a, &h, true).await?;
    let mut tx = a.db.begin().await?;
    let charge=query("SELECT original_cents,created_at + interval '24 hours'>clock_timestamp() AS active FROM mock_pix_charges WHERE company_id=$1 AND txid=$2 FOR UPDATE")
        .bind(scope.company).bind(i.txid.as_str()).fetch_optional(&mut *tx).await?.ok_or(Error::Missing)?;
    let existing: Option<String> =
        query_scalar("SELECT e2e FROM mock_pix_receipts WHERE company_id=$1 AND txid=$2")
            .bind(scope.company)
            .bind(i.txid.as_str())
            .fetch_optional(&mut *tx)
            .await?;
    let e2e = if let Some(id) = existing {
        id
    } else {
        if !charge.get::<bool, _>("active") {
            return Err(Error::Conflict);
        }
        let mut id = Uuid::new_v4().simple().to_string();
        id.replace_range(..1, "E");
        query("INSERT INTO mock_pix_receipts(company_id,e2e,txid,cents) VALUES($1,$2,$3,$4)")
            .bind(scope.company)
            .bind(&id)
            .bind(i.txid.as_str())
            .bind(charge.get::<i64, _>("original_cents"))
            .execute(&mut *tx)
            .await?;
        audit(&mut tx, &u, "Recebimento Pix simulado em MOCK").await?;
        id
    };
    enqueue(&mut tx, scope, "PIX", &e2e, &e2e)
        .await
        .map_err(Error::Bank)?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(Simulated { end_to_end_id: e2e })))
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingAction {
    Pix,
    Boleto,
    Cancelar,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    code: Uuid,
    action: BillingAction,
}
pub async fn change_billing(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<Change>,
) -> Result<StatusCode> {
    let (u, scope) = session(&a, &h, true).await?;
    let (next, way) = match i.action {
        BillingAction::Pix => ("RECEBIDO", Some("PIX")),
        BillingAction::Boleto => ("RECEBIDO", Some("BOLETO")),
        BillingAction::Cancelar => ("CANCELADO", None),
    };
    let mut tx = a.db.begin().await?;
    let row=query("SELECT status,channel FROM mock_billing_requests WHERE company_id=$1 AND code=$2 FOR UPDATE").bind(scope.company).bind(i.code).fetch_optional(&mut *tx).await?.ok_or(Error::Missing)?;
    let status: String = row.get("status");
    if status == "A_RECEBER" {
        query(
            "UPDATE mock_billing_requests SET status=$3,channel=$4 WHERE company_id=$1 AND code=$2",
        )
        .bind(scope.company)
        .bind(i.code)
        .bind(next)
        .bind(way)
        .execute(&mut *tx)
        .await?;
        audit(&mut tx, &u, "Atualização de Cobrança V3 simulada em MOCK").await?;
    } else if status != next || row.get::<Option<String>, _>("channel").as_deref() != way {
        return Err(Error::Conflict);
    }
    enqueue(
        &mut tx,
        scope,
        "V3",
        &i.code.to_string(),
        &format!("{}:{next}", i.code),
    )
    .await
    .map_err(Error::Bank)?;
    tx.commit().await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
pub struct Retry {
    id: String,
}
pub async fn retry(State(a): State<App>, h: HeaderMap, Json(i): Json<Retry>) -> Result<StatusCode> {
    let (u, s) = session(&a, &h, true).await?;
    if !u.roles.iter().any(|r| r == "ADMIN") {
        return Err(Error::Forbidden);
    }
    let mut tx = a.db.begin().await?;
    let n=query("UPDATE collection_events SET state='AGUARDANDO',attempts=0,next_at=now(),error_code=NULL WHERE company_id=$1 AND environment=$2::text::integration_environment AND account_id IS NOT DISTINCT FROM $3 AND id=$4 AND state='ESGOTADO'")
       .bind(s.company).bind(s.mode).bind(s.account).bind(i.id.parse::<i64>().map_err(|_|invalid())?).execute(&mut *tx).await?.rows_affected();
    if n == 0 {
        return Err(Error::Conflict);
    }
    audit(&mut tx, &u, "Reprocessamento de notificação solicitado").await?;
    tx.commit().await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum List {
    Clientes,
    Pix,
    Cobrancas,
    Eventos,
    Confirmacoes,
}
#[derive(Deserialize)]
pub struct Filter {
    lista: List,
    #[serde(default)]
    pagina: u32,
}
#[derive(Serialize)]
pub struct Page {
    items: Vec<serde_json::Value>,
    has_more: bool,
}

pub async fn list(
    State(a): State<App>,
    h: HeaderMap,
    Query(f): Query<Filter>,
) -> Result<Json<Page>> {
    let (_, s) = session(&a, &h, false).await?;
    let sql = match f.lista {
        List::Clientes => {
            "SELECT json_build_object('id',id,'name',name)::text FROM customers WHERE company_id=$1 ORDER BY name,id LIMIT 26 OFFSET $2"
        }
        List::Pix if s.mode == "MOCK" => {
            "SELECT json_build_object('id',c.txid,'cents',c.original_cents::text,'status',CASE WHEN r.reconciliation='CONCILIADO' THEN 'CONCLUIDA' WHEN p.e2e IS NOT NULL THEN 'EM_CONCILIACAO' WHEN c.created_at+interval '24 hours'<now() THEN 'EXPIRADA' ELSE 'ATIVA' END)::text FROM mock_pix_charges c LEFT JOIN mock_pix_receipts p USING(company_id,txid) LEFT JOIN collection_results r ON r.company_id=p.company_id AND r.family='PIX' AND r.environment='MOCK' AND r.reference=p.e2e WHERE c.company_id=$1 ORDER BY c.created_at DESC,c.txid LIMIT 26 OFFSET $2"
        }
        List::Cobrancas if s.mode == "MOCK" => {
            "SELECT json_build_object('id',b.code,'name',c.name,'cents',b.cents::text,'due_on',b.due_on,'status',b.status,'channel',b.channel)::text FROM mock_billing_requests b JOIN customers c ON (b.company_id,b.customer_id)=(c.company_id,c.id) WHERE b.company_id=$1 ORDER BY b.created_at DESC,b.code LIMIT 26 OFFSET $2"
        }
        List::Eventos => {
            "SELECT json_build_object('id',id::text,'family',family,'reference',reference,'status',state,'attempts',attempts,'error',error_code)::text FROM collection_events WHERE company_id=$1 AND environment=$3::text::integration_environment AND account_id IS NOT DISTINCT FROM $4 ORDER BY id DESC LIMIT 26 OFFSET $2"
        }
        List::Confirmacoes => {
            "SELECT json_build_object('id',reference,'family',family,'txid',txid,'status',status,'cents',received_cents::text,'channel',channel,'reconciliation',reconciliation)::text FROM collection_results WHERE company_id=$1 AND environment=$3::text::integration_environment AND account_id IS NOT DISTINCT FROM $4 ORDER BY checked_at DESC,family,reference LIMIT 26 OFFSET $2"
        }
        _ => return Err(Error::Forbidden),
    };
    let mut q = query_scalar::<_, String>(sql)
        .bind(s.company)
        .bind(i64::from(f.pagina) * 25);
    if matches!(f.lista, List::Eventos | List::Confirmacoes) {
        q = q.bind(s.mode).bind(s.account)
    }
    let rows = q.fetch_all(&a.db).await?;
    let more = rows.len() > 25;
    let items = rows
        .iter()
        .take(25)
        .map(|r| serde_json::from_str(r).map_err(|_| Error::Internal))
        .collect::<Result<_>>()?;
    Ok(Json(Page {
        items,
        has_more: more,
    }))
}

pub async fn sandbox_issue(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<super::SandboxOrder>,
) -> Result<Json<super::OrderState>> {
    let (u, s) = session(&a, &h, true).await?;
    Ok(Json(
        a.bank
            .issue_sandbox(s.company, u.id, i)
            .await
            .map_err(Error::Bank)?,
    ))
}
