use crate::{
    App,
    api::{Error, Result},
    auth,
};
use axum::{Json, extract::State, http::HeaderMap};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx_core::{
    query::query, query_as::query_as, query_scalar::query_scalar, transaction::Transaction,
};
use sqlx_postgres::Postgres;
use totp_rs::{Algorithm, Builder, Totp};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordInput {
    senha: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeInput {
    codigo: String,
}

#[derive(Serialize)]
pub struct MfaStatus {
    habilitado: bool,
}

#[derive(Serialize)]
pub struct Enrollment {
    otpauth_uri: String,
}

fn cipher(key: &[u8; 32]) -> Result<XChaCha20Poly1305> {
    XChaCha20Poly1305::new_from_slice(key).map_err(|_| Error::Internal)
}

pub(crate) fn encrypt(key: &[u8; 32], value: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0_u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher(key)?
        .encrypt(XNonce::from_slice(&nonce), value)
        .map_err(|_| Error::Internal)?;
    let mut output = nonce.to_vec();
    output.extend(encrypted);
    Ok(output)
}

pub(crate) fn decrypt(key: &[u8; 32], value: &[u8]) -> Result<Vec<u8>> {
    let (nonce, encrypted) = value.split_at_checked(24).ok_or(Error::Internal)?;
    cipher(key)?
        .decrypt(XNonce::from_slice(nonce), encrypted)
        .map_err(|_| Error::Internal)
}

fn totp(secret: &[u8], email: &str) -> Result<Totp> {
    Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_secret(secret.to_vec())
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_issuer(Some("Inter PJ Manager".to_owned()))
        .with_account_name(email.to_owned())
        .build()
        .map_err(|_| Error::Internal)
}

async fn identity(app: &App, headers: &HeaderMap) -> Result<auth::Identity> {
    if app.bank.environment() != "MOCK"
        && !crate::config::payment_preparation(app.bank.environment())
    {
        return Err(Error::Missing);
    }
    let user = auth::identity(app, headers).await?;
    if app.bank.environment() != "MOCK"
        && (app
            .bank
            .collection_binding()
            .is_none_or(|(company, _)| company != user.company_id)
            || !user
                .roles
                .iter()
                .any(|r| matches!(r.as_str(), "ADMIN" | "APPROVER" | "OPERATOR")))
    {
        return Err(Error::Forbidden);
    }
    Ok(user)
}

async fn check_password(app: &App, user: Uuid, password: String) -> Result<()> {
    if password.len() > 256 {
        return Err(Error::Unauthorized);
    }
    let permit = app
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Busy)?;
    let hash: String = query_scalar("SELECT password_hash FROM users WHERE id=$1 AND active")
        .bind(user)
        .fetch_optional(&app.db)
        .await?
        .ok_or(Error::Unauthorized)?;
    let verified = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        use argon2::{Argon2, PasswordHash, PasswordVerifier};
        PasswordHash::new(&hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    })
    .await
    .map_err(|_| Error::Internal)?;
    if verified {
        Ok(())
    } else {
        Err(Error::Unauthorized)
    }
}

pub async fn status(State(app): State<App>, headers: HeaderMap) -> Result<Json<MfaStatus>> {
    let user = identity(&app, &headers).await?;
    let habilitado: bool = query_scalar("SELECT EXISTS(SELECT FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NOT NULL)")
        .bind(user.company_id).bind(user.id).fetch_one(&app.db).await?;
    Ok(Json(MfaStatus { habilitado }))
}

pub async fn begin_enrollment(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<PasswordInput>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Enrollment>> {
    let user = identity(&app, &headers).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    check_password(&app, user.id, input.senha).await?;
    let mut transaction = app.db.begin().await?;
    query("SELECT pg_advisory_xact_lock(hashtextextended($1::text,0))")
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    let enabled: bool = query_scalar("SELECT EXISTS(SELECT FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NOT NULL)")
        .bind(user.company_id).bind(user.id).fetch_one(&mut *transaction).await?;
    if enabled {
        return Err(Error::Conflict);
    }
    let credential = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_issuer(Some("Inter PJ Manager".to_owned()))
        .with_account_name(user.email.clone())
        .build()
        .map_err(|_| Error::Internal)?;
    let url = credential.to_url().map_err(|_| Error::Internal)?;
    let secret = credential.secret().as_bytes();
    let encrypted = encrypt(&app.mfa_key, secret)?;
    query("INSERT INTO user_mfa_credentials(company_id,user_id,encrypted_secret,pending_expires_at) VALUES($1,$2,$3,now()+interval '10 minutes') ON CONFLICT(company_id,user_id) DO UPDATE SET encrypted_secret=EXCLUDED.encrypted_secret,enabled_at=NULL,pending_expires_at=EXCLUDED.pending_expires_at,last_totp_counter=-1,failed_attempts=0,locked_until=NULL WHERE user_mfa_credentials.enabled_at IS NULL")
        .bind(user.company_id).bind(user.id).bind(encrypted).execute(&mut *transaction).await?;
    let pending: bool = query_scalar("SELECT EXISTS(SELECT FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NULL AND pending_expires_at>now())")
        .bind(user.company_id).bind(user.id).fetch_one(&mut *transaction).await?;
    if !pending {
        return Err(Error::Conflict);
    }
    audit(&mut transaction, &user, "Cadastro MFA TOTP iniciado").await?;
    transaction.commit().await?;
    Ok(Json(Enrollment { otpauth_uri: url }))
}

pub async fn confirm_enrollment(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<CodeInput>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<MfaStatus>> {
    let user = identity(&app, &headers).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    let mut tx = app.db.begin().await?;
    let row: Option<(Vec<u8>, i64, i32, Option<time::OffsetDateTime>)> = query_as("SELECT encrypted_secret,last_totp_counter,failed_attempts,locked_until FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NULL AND pending_expires_at>now() FOR UPDATE")
        .bind(user.company_id).bind(user.id).fetch_optional(&mut *tx).await?;
    let Some((secret, last_counter, failures, locked_until)) = row else {
        return Err(Error::Conflict);
    };
    if locked_until.is_some_and(|until| until > time::OffsetDateTime::now_utc()) {
        return Err(Error::Busy);
    }
    let secret = Zeroizing::new(decrypt(&app.mfa_key, &secret)?);
    let totp = totp(&secret, &user.email)?;
    let step = valid_step(&totp, &input.codigo);
    if step.is_none_or(|step| step as i64 <= last_counter) {
        record_failure(&mut tx, user.company_id, user.id, failures).await?;
        tx.commit().await?;
        return Err(Error::Unauthorized);
    }
    query("UPDATE user_mfa_credentials SET enabled_at=now(),pending_expires_at=NULL,last_totp_counter=$3,failed_attempts=0,locked_until=NULL WHERE company_id=$1 AND user_id=$2")
        .bind(user.company_id).bind(user.id).bind(step.unwrap() as i64).execute(&mut *tx).await?;
    audit(&mut tx, &user, "MFA TOTP habilitado").await?;
    tx.commit().await?;
    Ok(Json(MfaStatus { habilitado: true }))
}

pub async fn disable(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<DisableInput>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<MfaStatus>> {
    let user = identity(&app, &headers).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    check_password(&app, user.id, input.senha).await?;
    let mut tx = app.db.begin().await?;
    let row: Option<(Vec<u8>, i64, i32, Option<time::OffsetDateTime>)> = query_as("SELECT encrypted_secret,last_totp_counter,failed_attempts,locked_until FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NOT NULL FOR UPDATE")
        .bind(user.company_id).bind(user.id).fetch_optional(&mut *tx).await?;
    let Some((encrypted, last_counter, failures, locked_until)) = row else {
        return Err(Error::Conflict);
    };
    if locked_until.is_some_and(|until| until > time::OffsetDateTime::now_utc()) {
        return Err(Error::Busy);
    }
    let secret = Zeroizing::new(decrypt(&app.mfa_key, &encrypted)?);
    let totp = totp(&secret, &user.email)?;
    let step = valid_step(&totp, &input.codigo);
    if step.is_none_or(|step| step as i64 <= last_counter) {
        record_failure(&mut tx, user.company_id, user.id, failures).await?;
        tx.commit().await?;
        return Err(Error::Unauthorized);
    }
    query("DELETE FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2")
        .bind(user.company_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, &user, "MFA TOTP desabilitado").await?;
    tx.commit().await?;
    Ok(Json(MfaStatus { habilitado: false }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisableInput {
    senha: String,
    codigo: String,
}

pub(crate) async fn consume_code(
    tx: &mut Transaction<'_, Postgres>,
    app: &App,
    user: &auth::Identity,
    code: &str,
) -> Result<bool> {
    let row: Option<(Vec<u8>, i64, i32, Option<time::OffsetDateTime>)> = query_as("SELECT encrypted_secret,last_totp_counter,failed_attempts,locked_until FROM user_mfa_credentials WHERE company_id=$1 AND user_id=$2 AND enabled_at IS NOT NULL FOR UPDATE")
        .bind(user.company_id).bind(user.id).fetch_optional(&mut **tx).await?;
    let Some((encrypted, last_counter, failures, locked_until)) = row else {
        return Ok(false);
    };
    if locked_until.is_some_and(|until| until > time::OffsetDateTime::now_utc()) {
        return Ok(false);
    }
    let secret = Zeroizing::new(decrypt(&app.mfa_key, &encrypted)?);
    let totp = totp(&secret, &user.email)?;
    let Some(step) = valid_step(&totp, code).filter(|step| *step as i64 > last_counter) else {
        record_failure(tx, user.company_id, user.id, failures).await?;
        return Ok(false);
    };
    query("UPDATE user_mfa_credentials SET last_totp_counter=$3,failed_attempts=0,locked_until=NULL WHERE company_id=$1 AND user_id=$2")
        .bind(user.company_id).bind(user.id).bind(step as i64).execute(&mut **tx).await?;
    Ok(true)
}

fn valid_step(totp: &Totp, code: &str) -> Option<u64> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    totp.check_current(code)
}

async fn record_failure(
    tx: &mut Transaction<'_, Postgres>,
    company: Uuid,
    user: Uuid,
    failures: i32,
) -> Result<()> {
    let next = failures.saturating_add(1);
    query("UPDATE user_mfa_credentials SET failed_attempts=$3,locked_until=CASE WHEN $3>=5 THEN now()+interval '15 minutes' ELSE NULL END WHERE company_id=$1 AND user_id=$2")
        .bind(company).bind(user).bind(next).execute(&mut **tx).await?;
    Ok(())
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    user: &auth::Identity,
    action: &str,
) -> Result<()> {
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(user.company_id)
        .bind(user.id)
        .bind(action)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifragem_autenticada_rejeita_alteracao_e_chave_diferente() {
        let key = [7_u8; 32];
        let Ok(encrypted) = encrypt(&key, b"segredo-totp") else {
            panic!("cifragem deve funcionar");
        };
        assert!(decrypt(&key, &encrypted).is_ok_and(|clear| clear == b"segredo-totp"));
        let mut altered = encrypted.clone();
        altered[30] ^= 1;
        assert!(decrypt(&key, &altered).is_err());
        assert!(decrypt(&[8_u8; 32], &encrypted).is_err());
    }

    #[test]
    fn totp_rejeita_formato_invalido_e_aceita_codigo_vigente() {
        let Ok(totp) = totp(&[9_u8; 20], "operador@example.test") else {
            panic!("TOTP deve ser construído");
        };
        let code = totp.generate_current().to_string();
        assert_eq!(valid_step(&totp, &code), totp.check_current(&code));
        assert_eq!(valid_step(&totp, "12345"), None);
        assert_eq!(valid_step(&totp, "abcdef"), None);
    }

    #[test]
    fn senha_mfa_tem_resistencia_a_replay_pelo_contador_monotono() {
        let Ok(totp) = totp(&[4_u8; 20], "aprovador@example.test") else {
            panic!("TOTP deve ser construído");
        };
        let now = 1_800_000_000_u64;
        let code = totp.generate(now).to_string();
        let step = totp.check(&code, now).unwrap();
        assert!(step > 0);
        assert_eq!(totp.check("000000", now), None);
    }
}
