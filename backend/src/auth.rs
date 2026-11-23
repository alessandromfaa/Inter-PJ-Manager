use crate::{
    App,
    api::{Error, Result},
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx_core::{query::query, row::Row};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Login {
    email: String,
    password: String,
}
#[derive(Serialize)]
pub struct Identity {
    pub id: Uuid,
    pub company_id: Uuid,
    pub company: String,
    pub email: String,
    pub roles: Vec<String>,
}
impl Identity {
    pub fn can_audit(&self) -> bool {
        self.roles.iter().any(|r| r == "ADMIN" || r == "AUDITOR")
    }
}
fn digest(raw: &str) -> Vec<u8> {
    Sha256::digest(raw.as_bytes()).to_vec()
}
fn session_digest(h: &HeaderMap) -> Result<Vec<u8>> {
    let cookies = h
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    cookies
        .split(';')
        .filter_map(|c| c.trim().strip_prefix("pj_session="))
        .find(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(digest)
        .ok_or(Error::Unauthorized)
}
pub async fn identity(app: &App, h: &HeaderMap) -> Result<Identity> {
    let row=query("SELECT u.id,u.company_id,u.email,c.name,ARRAY(SELECT role::text FROM user_roles WHERE user_id=u.id) roles FROM user_sessions s JOIN users u ON s.user_id=u.id JOIN companies c ON c.id=u.company_id WHERE s.digest=$1 AND s.expires_at>now() AND u.active").bind(session_digest(h)?).fetch_optional(&app.db).await?.ok_or(Error::Unauthorized)?;
    let identity = Identity {
        id: row.get("id"),
        company_id: row.get("company_id"),
        company: row.get("name"),
        email: row.get("email"),
        roles: row.get("roles"),
    };
    if identity.roles.is_empty() {
        return Err(Error::Forbidden);
    }
    Ok(identity)
}
fn cookie(token: &str, age: u32, secure: bool) -> String {
    format!(
        "pj_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{}",
        if secure { "; Secure" } else { "" }
    )
}
pub async fn login(
    State(app): State<App>,
    payload: std::result::Result<Json<Login>, axum::extract::rejection::JsonRejection>,
) -> Result<Response> {
    let Json(input) = payload.map_err(|err| Error::Invalid(err.status()))?;
    if input.email.len() > 254 || input.password.len() > 256 {
        return Err(Error::Unauthorized);
    }
    let permit = app
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Busy)?;
    let mut tx = app.db.begin().await?;
    let row=query("SELECT id,company_id,password_hash,active,failures>=5 AND window_start>now()-interval '15 minutes' locked FROM users WHERE email=$1 FOR UPDATE").bind(input.email.trim().to_lowercase()).fetch_optional(&mut *tx).await?;
    if row.as_ref().is_some_and(|r| r.get::<bool, _>("locked")) {
        return Err(Error::Busy);
    }
    let stored = row
        .as_ref()
        .map(|r| r.get("password_hash"))
        .unwrap_or_else(|| app.dummy.as_ref().clone());
    let valid = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        PasswordHash::new(&stored).is_ok_and(|hash| {
            Argon2::default()
                .verify_password(input.password.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
    .map_err(|_| Error::Internal)?;
    let row = row.ok_or(Error::Unauthorized)?;
    let user: Uuid = row.get("id");
    let company: Uuid = row.get("company_id");
    if !valid || !row.get::<bool, _>("active") {
        query("UPDATE users SET failures=CASE WHEN window_start<=now()-interval '15 minutes' THEN 1 ELSE failures+1 END,window_start=CASE WHEN failures=0 OR window_start<=now()-interval '15 minutes' THEN now() ELSE window_start END WHERE id=$1").bind(user).execute(&mut *tx).await?;
        tx.commit().await?;
        return Err(Error::Unauthorized);
    }
    let raw = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    query("UPDATE users SET failures=0 WHERE id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    query("DELETE FROM user_sessions WHERE expires_at<=now()")
        .execute(&mut *tx)
        .await?;
    query("INSERT INTO user_sessions VALUES($1,$2,now()+interval '8 hours')")
        .bind(digest(&raw))
        .bind(user)
        .execute(&mut *tx)
        .await?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Sessão iniciada')")
        .bind(company)
        .bind(user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((
        [(header::SET_COOKIE, cookie(&raw, 28800, app.secure))],
        Json(true),
    )
        .into_response())
}
pub async fn logout(State(app): State<App>, h: HeaderMap) -> Result<Response> {
    let mut tx = app.db.begin().await?;
    query("WITH revoked AS (DELETE FROM user_sessions WHERE digest=$1 RETURNING user_id) INSERT INTO audit_logs(company_id,actor,action) SELECT u.company_id,u.id,'Sessão encerrada' FROM revoked r JOIN users u ON u.id=r.user_id").bind(session_digest(&h)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((
        [(header::SET_COOKIE, cookie("", 0, app.secure))],
        Json(true),
    )
        .into_response())
}
