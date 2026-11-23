use crate::{App, auth, domain::Snapshot};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx_core::{query::query, row::Row};
use uuid::Uuid;
pub enum Error {
    Invalid(StatusCode),
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
        next.run(req).await
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
pub async fn session(State(a): State<App>, h: HeaderMap) -> Result<Json<auth::Identity>> {
    Ok(Json(auth::identity(&a, &h).await?))
}
pub async fn dashboard(State(a): State<App>, h: HeaderMap) -> Result<Json<Snapshot>> {
    let user = auth::identity(&a, &h).await?;
    Ok(Json(
        a.bank
            .snapshot(user.company_id)
            .await
            .map_err(|_| Error::Internal)?,
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
