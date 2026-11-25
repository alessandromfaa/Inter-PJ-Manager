use crate::{
    App, auth,
    domain::{Dashboard, Statement, StatementQuery},
    infrastructure::banking::inter::BankError,
};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row};
use uuid::Uuid;
pub enum Error {
    Conflict,
    Missing,
    Invalid(StatusCode),
    Bank(BankError),
    Unauthorized,
    Forbidden,
    Busy,
    Internal,
}
pub type Result<T> = std::result::Result<T, Error>;
impl From<sqlx_core::Error> for Error {
    fn from(_: sqlx_core::Error) -> Self {
        tracing::error!("Falha de persistência");
        Error::Internal
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::Bank(err) => {
                let status = match err {
                    BankError::PixNotFound => StatusCode::NOT_FOUND,
                    BankError::PixConflict => StatusCode::CONFLICT,
                    BankError::PixDisabled => StatusCode::FORBIDDEN,
                    BankError::Account => StatusCode::FORBIDDEN,
                    BankError::Busy | BankError::Mock => StatusCode::CONFLICT,
                    BankError::Limit => StatusCode::TOO_MANY_REQUESTS,
                    BankError::Period => StatusCode::BAD_REQUEST,
                    BankError::Timeout => StatusCode::GATEWAY_TIMEOUT,
                    BankError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
                    _ => StatusCode::BAD_GATEWAY,
                };
                let msg = if matches!(err, BankError::Database(_)) {
                    "Falha de persistência".to_owned()
                } else {
                    err.to_string()
                };
                return (status, Json(msg)).into_response();
            }
            Self::Missing => (StatusCode::NOT_FOUND, "Registro não encontrado na empresa."),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "O estado atual ou os dados da solicitação impedem esta operação.",
            ),
            Self::Invalid(status) => (
                status,
                "Corpo da requisição inválido ou excede o limite permitido.",
            ),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Credenciais ou sessão inválidas"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Acesso negado"),
            Self::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                "Limite de tentativas atingido. Tente novamente mais tarde.",
            ),
            Self::Internal => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Serviço temporariamente indisponível",
            ),
        };
        (status, Json(msg)).into_response()
    }
}
pub async fn guard(State(app): State<App>, req: Request, next: Next) -> Response {
    let id = Uuid::new_v4();
    let start = std::time::Instant::now();
    let denied = req.method() == "POST"
        && (req.headers().get("origin").and_then(|v| v.to_str().ok()) != Some(app.origin.as_str())
            || req
                .headers()
                .get("x-pj-request")
                .and_then(|v| v.to_str().ok())
                != Some("1"));
    let mut response = if denied {
        Error::Forbidden.into_response()
    } else {
        tracing::Instrument::instrument(
            next.run(req),
            tracing::info_span!("requisicao",request_id=%id),
        )
        .await
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("x-request-id", id.to_string().parse().unwrap());
    tracing::info!(request_id=%id,status=response.status().as_u16(),elapsed_ms=start.elapsed().as_millis(),"Requisição concluída");
    response
}
pub async fn ready(State(a): State<App>) -> Result<&'static str> {
    query("SELECT 1").execute(&a.db).await?;
    Ok("ok")
}

pub async fn metrics(
    State(a): State<App>,
    headers: HeaderMap,
) -> Result<impl axum::response::IntoResponse> {
    let user = auth::identity(&a, &headers).await?;
    if !user.roles.iter().any(|role| role == "ADMIN") {
        return Err(Error::Forbidden);
    }
    query_scalar::<_, i32>("SELECT 1").fetch_one(&a.db).await?;
    let body = format!(
        "inter_pj_manager_ready 1\ninter_pj_manager_mock_mode {}\n",
        if a.bank.environment() == "MOCK" { 1 } else { 0 }
    );
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    ))
}
pub async fn session(State(a): State<App>, h: HeaderMap) -> Result<Json<auth::Identity>> {
    Ok(Json(auth::identity(&a, &h).await?))
}
pub async fn dashboard(State(a): State<App>, h: HeaderMap) -> Result<Json<Dashboard>> {
    let user = auth::identity(&a, &h).await?;
    Ok(Json(
        a.bank
            .snapshot(user.company_id)
            .await
            .map_err(Error::Bank)?,
    ))
}
pub async fn environment(State(a): State<App>) -> Json<&'static str> {
    Json(a.bank.environment())
}
pub async fn sync(State(a): State<App>, h: HeaderMap) -> Result<StatusCode> {
    let user = auth::identity(&a, &h).await?;
    if !user.roles.iter().any(|r| r == "ADMIN") {
        return Err(Error::Forbidden);
    }
    a.bank
        .synchronize(user.company_id, user.id)
        .await
        .map_err(Error::Bank)?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn statement(
    State(a): State<App>,
    h: HeaderMap,
    filter: std::result::Result<
        axum::extract::Query<StatementQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Result<Json<Statement>> {
    let user = auth::identity(&a, &h).await?;
    let filter = filter.map_err(|_| Error::Bank(BankError::Period))?;
    Ok(Json(
        a.bank
            .statement(user.company_id, filter.0)
            .await
            .map_err(Error::Bank)?,
    ))
}
#[derive(Serialize)]
pub struct Audit {
    action: String,
    at: String,
}
pub async fn audit(State(a): State<App>, h: HeaderMap) -> Result<Json<Vec<Audit>>> {
    let u = auth::identity(&a, &h).await?;
    if !u.can_audit() {
        return Err(Error::Forbidden);
    }
    Ok(Json(query("SELECT action,to_char(at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') at FROM audit_logs WHERE company_id=$1 ORDER BY id DESC LIMIT 100").bind(u.company_id).map(|row: sqlx_postgres::PgRow| Audit { action: row.get("action"), at: row.get("at") }).fetch_all(&a.db).await?))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixLookup {
    txid: crate::domain::pix::Txid,
}
pub async fn pix_charge(
    State(a): State<App>,
    h: HeaderMap,
    body: std::result::Result<Json<PixLookup>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<crate::domain::pix::PixCharge>> {
    let user = auth::identity(&a, &h).await?;
    if !user
        .roles
        .iter()
        .any(|r| matches!(r.as_str(), "ADMIN" | "VIEWER" | "OPERATOR"))
    {
        return Err(Error::Forbidden);
    }
    let input = body.map_err(|e| Error::Invalid(e.status()))?;
    let charge = a
        .bank
        .pix_charge(user.company_id, input.0.txid)
        .await
        .map_err(Error::Bank)?;
    query("INSERT INTO audit_logs(company_id, actor, action) VALUES($1, $2, $3)")
        .bind(user.company_id)
        .bind(user.id)
        .bind(format!("Cobrança Pix consultada em {}", charge.source))
        .execute(&a.db)
        .await?;
    Ok(Json(charge))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockPixIntent {
    intent_id: Uuid,
    original_cents: String,
}

pub async fn issue_mock_pix(
    State(a): State<App>,
    h: HeaderMap,
    body: std::result::Result<Json<MockPixIntent>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<crate::domain::pix::PixCharge>)> {
    let user = auth::identity(&a, &h).await?;
    if !user
        .roles
        .iter()
        .any(|r| matches!(r.as_str(), "ADMIN" | "OPERATOR"))
    {
        return Err(Error::Forbidden);
    }
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let amount = positive_cents(&input.original_cents)?;
    let (charge, inserted) = crate::infrastructure::banking::mock_pix::issue(
        &a.db,
        user.company_id,
        user.id,
        input.intent_id,
        amount,
    )
    .await
    .map_err(Error::Bank)?;
    Ok((
        if inserted {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(charge),
    ))
}

pub fn positive_cents(raw: &str) -> Result<i64> {
    let invalid = || Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY);
    if raw.len() > 12 || raw.starts_with('0') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    raw.parse::<i64>().map_err(|_| invalid())
}
