use crate::{
    App,
    api::{Error, Result, positive_cents},
    auth::{self, Identity},
    config,
    infrastructure::banking::inter::pagamento_por_chave,
    mfa,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx_core::{query::query, row::Row, transaction::Transaction};
use sqlx_postgres::{PgRow, Postgres};
use uuid::Uuid;
use zeroize::Zeroizing;

// O piloto autorizado é de um centavo por intenção.
const PILOT_CENTS: i64 = 1;
type Tx<'a> = Transaction<'a, Postgres>;

fn role(user: &Identity, required: &str) -> bool {
    user.roles.iter().any(|r| r == "ADMIN" || r == required)
}
async fn context(app: &App, headers: &HeaderMap) -> Result<(Identity, Uuid)> {
    let user = auth::identity(app, headers).await?;
    let (company, account) = app.bank.collection_binding().ok_or(Error::Forbidden)?;
    if user.company_id != company || !(role(&user, "OPERATOR") || role(&user, "APPROVER")) {
        return Err(Error::Forbidden);
    }
    Ok((user, account))
}
// Chave derivada com finalidade e identidade da linha. Trocas de ciphertext entre linhas falham.
fn row_key(app: &App, company: Uuid, account: Uuid, id: Uuid) -> Zeroizing<[u8; 32]> {
    Zeroizing::new(
        Sha256::new()
            .chain_update(b"pj-manager/pix/recipient/v1")
            .chain_update(company.as_bytes())
            .chain_update(account.as_bytes())
            .chain_update(id.as_bytes())
            .chain_update(app.mfa_key.as_slice())
            .finalize()
            .into(),
    )
}
fn recipient(app: &App, company: Uuid, account: Uuid, row: &PgRow) -> Result<String> {
    String::from_utf8(mfa::decrypt(
        &row_key(app, company, account, row.get("id")),
        row.get("recipient_ciphertext"),
    )?)
    .map_err(|_| Error::Internal)
}
async fn audit(tx: &mut Tx<'_>, user: &Identity, id: Uuid, action: &str) -> Result<()> {
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(user.company_id)
        .bind(user.id)
        .bind(format!("{action} ({id})"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepare {
    idempotencia: Uuid,
    chave: String,
    valor_centavos: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confirm {
    id: Uuid,
    chave: String,
    valor_centavos: String,
    codigo_mfa: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lookup {
    id: Uuid,
}
#[derive(Serialize)]
pub struct Outcome {
    id: Uuid,
    situacao: String,
}
#[derive(Serialize)]
pub struct Entry {
    id: Uuid,
    chave: String,
    valor_centavos: String,
    situacao: String,
    status_banco: Option<String>,
    pode_aprovar: bool,
    pode_enviar: bool,
    pode_consultar: bool,
}
#[derive(Serialize)]
pub struct List {
    intencoes: Vec<Entry>,
    envio_habilitado: bool,
}
pub async fn list(State(app): State<App>, headers: HeaderMap) -> Result<Json<List>> {
    let (user, account) = context(&app, &headers).await?;
    let send_enabled = config::payment_send_enabled(app.bank.environment());
    let rows = query("SELECT id,creator_id,amount_cents,recipient_ciphertext,state,bank_status,provider_reference IS NOT NULL AS known FROM pix_payment_intents WHERE company_id=$1 AND account_id=$2 ORDER BY created_at DESC,id DESC LIMIT 100")
        .bind(user.company_id).bind(account).fetch_all(&app.db).await?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let own = row.get::<Uuid, _>("creator_id") == user.id;
        let state = row.get::<String, _>("state");
        entries.push(Entry {
            id: row.get("id"),
            chave: recipient(&app, user.company_id, account, &row)?,
            valor_centavos: row.get::<i64, _>("amount_cents").to_string(),
            pode_aprovar: !own && state == "EM_REVISAO" && role(&user, "APPROVER"),
            pode_enviar: own && state == "APROVADO" && role(&user, "OPERATOR") && send_enabled,
            pode_consultar: row.get("known"),
            situacao: state,
            status_banco: row.get("bank_status"),
        });
    }
    Ok(Json(List {
        intencoes: entries,
        envio_habilitado: send_enabled,
    }))
}
pub async fn prepare(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Prepare>,
) -> Result<Json<Outcome>> {
    let (user, account) = context(&app, &headers).await?;
    if !role(&user, "OPERATOR") {
        return Err(Error::Forbidden);
    }
    let amount = positive_cents(&input.valor_centavos)?;
    if amount != PILOT_CENTS
        || input.idempotencia.is_nil()
        || pagamento_por_chave(amount, &input.chave).is_none()
    {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let id = Uuid::new_v4();
    let cipher = mfa::encrypt(
        &row_key(&app, user.company_id, account, id),
        input.chave.as_bytes(),
    )?;
    let mut tx = app.db.begin().await?;
    let created = query("INSERT INTO pix_payment_intents(id,company_id,account_id,creator_id,amount_cents,recipient_ciphertext,idempotency_key,state) VALUES($1,$2,$3,$4,$5,$6,$7,'EM_REVISAO') ON CONFLICT(company_id,idempotency_key) DO NOTHING")
        .bind(id).bind(user.company_id).bind(account).bind(user.id).bind(amount).bind(cipher).bind(input.idempotencia).execute(&mut *tx).await?.rows_affected();
    let row = query("SELECT id,amount_cents,recipient_ciphertext,state FROM pix_payment_intents WHERE company_id=$1 AND account_id=$2 AND creator_id=$3 AND idempotency_key=$4")
        .bind(user.company_id).bind(account).bind(user.id).bind(input.idempotencia).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?;
    if row.get::<i64, _>("amount_cents") != amount
        || recipient(&app, user.company_id, account, &row)? != input.chave
    {
        return Err(Error::Conflict);
    }
    if created == 1 {
        audit(&mut tx, &user, id, "Pagamento Pix encaminhado para revisão").await?;
    }
    tx.commit().await?;
    Ok(Json(Outcome {
        id: row.get("id"),
        situacao: row.get("state"),
    }))
}
async fn locked(tx: &mut Tx<'_>, user: &Identity, account: Uuid, id: Uuid) -> Result<PgRow> {
    query("SELECT id,creator_id,approver_id,amount_cents,recipient_ciphertext,state,provider_reference FROM pix_payment_intents WHERE company_id=$1 AND account_id=$2 AND id=$3 FOR UPDATE")
        .bind(user.company_id).bind(account).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::Missing)
}
async fn transition(
    tx: &mut Tx<'_>,
    user: &Identity,
    account: Uuid,
    id: Uuid,
    state: &str,
    approver: Option<Uuid>,
    reference: Option<Uuid>,
) -> Result<()> {
    query("UPDATE pix_payment_intents SET state=$4, approver_id=coalesce($5,approver_id), approved_at=CASE WHEN $5::uuid IS NULL THEN approved_at ELSE now() END, provider_reference=coalesce($6,provider_reference),updated_at=now() WHERE company_id=$1 AND account_id=$2 AND id=$3")
        .bind(user.company_id).bind(account).bind(id).bind(state).bind(approver).bind(reference).execute(&mut **tx).await?;
    audit(tx, user, id, &format!("Pagamento Pix: {state}")).await
}
fn check_details(
    app: &App,
    user: &Identity,
    account: Uuid,
    row: &PgRow,
    input: &Confirm,
) -> Result<()> {
    if row.get::<i64, _>("amount_cents") != PILOT_CENTS
        || positive_cents(&input.valor_centavos)? != PILOT_CENTS
        || recipient(app, user.company_id, account, row)? != input.chave
    {
        return Err(Error::Conflict);
    }
    Ok(())
}
pub async fn approve(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Confirm>,
) -> Result<Json<Outcome>> {
    let (user, account) = context(&app, &headers).await?;
    if !role(&user, "APPROVER") {
        return Err(Error::Forbidden);
    }
    let mut tx = app.db.begin().await?;
    let row = locked(&mut tx, &user, account, input.id).await?;
    if row.get::<Uuid, _>("creator_id") == user.id {
        return Err(Error::Forbidden);
    }
    check_details(&app, &user, account, &row, &input)?;
    let state: String = row.get("state");
    if row.get::<Option<Uuid>, _>("approver_id") == Some(user.id) {
        return Ok(Json(Outcome {
            id: input.id,
            situacao: state,
        }));
    }
    if state != "EM_REVISAO" {
        return Err(Error::Conflict);
    }
    if !mfa::consume_code(&mut tx, &app, &user, &input.codigo_mfa).await? {
        tx.commit().await?;
        return Err(Error::Forbidden);
    }
    transition(
        &mut tx,
        &user,
        account,
        input.id,
        "APROVADO",
        Some(user.id),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(Outcome {
        id: input.id,
        situacao: "APROVADO".into(),
    }))
}
pub async fn send(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Confirm>,
) -> Result<Json<Outcome>> {
    let (user, account) = context(&app, &headers).await?;
    if !role(&user, "OPERATOR") || !config::payment_send_enabled(app.bank.environment()) {
        return Err(Error::Forbidden);
    }
    let mut tx = app.db.begin().await?;
    let row = locked(&mut tx, &user, account, input.id).await?;
    if row.get::<Uuid, _>("creator_id") != user.id
        || row
            .get::<Option<Uuid>, _>("approver_id")
            .is_none_or(|id| id == user.id)
    {
        return Err(Error::Forbidden);
    }
    check_details(&app, &user, account, &row, &input)?;
    let state: String = row.get("state");
    if state != "APROVADO" {
        return Ok(Json(Outcome {
            id: input.id,
            situacao: state,
        }));
    }
    if !mfa::consume_code(&mut tx, &app, &user, &input.codigo_mfa).await? {
        tx.commit().await?;
        return Err(Error::Forbidden);
    }
    transition(&mut tx, &user, account, input.id, "EM_ENVIO", None, None).await?;
    tx.commit().await?;
    // A reserva é durável antes da transmissão. Após interrupção o mesmo id nunca volta a APROVADO.
    let reply = app
        .bank
        .submit_payment(user.company_id, input.id, PILOT_CENTS, &input.chave)
        .await;
    let (state, reference) = match reply {
        Ok(r) => (r.state, Some(r.reference)),
        Err(_) => ("INDETERMINADO", None),
    };
    let mut tx = app.db.begin().await?;
    transition(&mut tx, &user, account, input.id, state, None, reference).await?;
    tx.commit().await?;
    Ok(Json(Outcome {
        id: input.id,
        situacao: state.into(),
    }))
}
pub async fn reconcile(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Lookup>,
) -> Result<Json<Outcome>> {
    let (user, account) = context(&app, &headers).await?;
    let mut tx = app.db.begin().await?;
    let row = locked(&mut tx, &user, account, input.id).await?;
    let reference = row
        .get::<Option<Uuid>, _>("provider_reference")
        .ok_or(Error::Conflict)?;
    let raw = app
        .bank
        .payment_status(user.company_id, reference)
        .await
        .map_err(Error::Bank)?;
    let state = match raw.as_str() {
        "PAGO" => "PAGO",
        "AGUARDANDO_APROVACAO" => "AGUARDANDO_APROVACAO",
        "AGENDADO" => "AGENDADO",
        "REPROVADO" | "EXPIRADO" | "FALHA" => "REJEITADO",
        "CANCELADO" | "CANCELADO_SEM_SALDO" | "AGENDAMENTO_CANCELADO" => "CANCELADO",
        _ => "PROCESSADO",
    };
    query("UPDATE pix_payment_intents SET state=$4,bank_status=$5,updated_at=now() WHERE company_id=$1 AND account_id=$2 AND id=$3")
        .bind(user.company_id).bind(account).bind(input.id).bind(state).bind(&raw).execute(&mut *tx).await?;
    audit(
        &mut tx,
        &user,
        input.id,
        &format!("Consulta de pagamento Pix: {raw}"),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(Outcome {
        id: input.id,
        situacao: state.into(),
    }))
}
