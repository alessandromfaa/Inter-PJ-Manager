mod api;
mod application;
mod auth;
mod config;
mod domain;
mod financeiro;
mod mfa;
mod pagamentos;
mod recebimentos;
mod infrastructure {
    pub mod banking;
}

use anyhow::{Context, Result, ensure};
use application::BankProvider;
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use infrastructure::banking::MockBank;
use sqlx_core::{migrate::Migrator, query::query, query_scalar::query_scalar};
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::{env, str::FromStr, sync::Arc};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone)]
struct App {
    db: PgPool,
    secure: bool,
    origin: String,
    dummy: Arc<String>,
    workers: Arc<Semaphore>,
    bank: Arc<dyn BankProvider>,
    mfa_key: Arc<[u8; 32]>,
}
fn hash(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|v| v.to_string())
        .map_err(|_| anyhow::anyhow!("Falha ao proteger a senha"))
}
fn secret(path: &str) -> Result<String> {
    Ok(std::fs::read_to_string(path)
        .context("Falha ao ler arquivo de segredo")?
        .trim_end_matches(['\r', '\n'])
        .into())
}
fn encryption_key(path: &str) -> Result<[u8; 32]> {
    let encoded = secret(path)?;
    let decoded = hex::decode(encoded).context("Chave de MFA inválida")?;
    decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("Chave de MFA deve conter 32 bytes"))
}
async fn provision(db: &PgPool, bootstrap: bool) -> Result<()> {
    let email = env::var(if bootstrap {
        "ADMIN_EMAIL"
    } else {
        "USER_EMAIL"
    })?
    .trim()
    .to_lowercase();
    ensure!(email.contains('@') && email.len() <= 254, "E-mail inválido");
    let mut tx = db.begin().await?;
    if bootstrap {
        query("SELECT pg_advisory_xact_lock(18502210)")
            .execute(&mut *tx)
            .await?;
        let exists: bool = query_scalar("SELECT EXISTS(SELECT FROM users)")
            .fetch_one(&mut *tx)
            .await?;
        if exists {
            tx.commit().await?;
            return Ok(());
        }
    }
    let password = secret(&env::var(if bootstrap {
        "ADMIN_PASSWORD_FILE"
    } else {
        "USER_PASSWORD_FILE"
    })?)?;
    ensure!(
        (16..=256).contains(&password.len()),
        "A senha inicial deve ter de 16 a 256 bytes"
    );
    let company = if bootstrap {
        let id = Uuid::new_v4();
        query("INSERT INTO companies VALUES($1,$2)")
            .bind(id)
            .bind(config::var("COMPANY_NAME", "Empresa de demonstração"))
            .execute(&mut *tx)
            .await?;
        id
    } else {
        Uuid::parse_str(&env::var("COMPANY_ID")?)?
    };
    let user = Uuid::new_v4();
    let role = if bootstrap {
        "ADMIN".into()
    } else {
        env::var("USER_ROLE")?
    };
    ensure!(
        matches!(
            role.as_str(),
            "ADMIN" | "APPROVER" | "OPERATOR" | "AUDITOR" | "VIEWER"
        ),
        "Perfil inválido"
    );
    query("INSERT INTO users(id,company_id,email,password_hash) VALUES($1,$2,$3,$4)")
        .bind(user)
        .bind(company)
        .bind(email)
        .bind(hash(&password)?)
        .execute(&mut *tx)
        .await?;
    query("INSERT INTO user_roles VALUES($1,$2::text::user_role)")
        .bind(user)
        .bind(role)
        .execute(&mut *tx)
        .await?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES ($1,$2,$3)")
        .bind(company)
        .bind(user)
        .bind(if bootstrap {
            "Administrador inicial criado"
        } else {
            "Usuário criado pelo provisionamento local"
        })
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let secure = config::validate()?;
    match env::args().nth(1).as_deref() {
        Some("verificar-acesso-saldo") => {
            return infrastructure::banking::inter::verify_balance_access().await;
        }
        Some("verificar-acesso-extrato") => {
            return infrastructure::banking::inter::verify_statement_access().await;
        }
        _ => {}
    }
    let mut options = PgConnectOptions::from_str(&env::var("DATABASE_URL")?)
        .context("Configuração PostgreSQL inválida")?;
    if let Ok(path) = env::var("DB_PASSWORD_FILE") {
        options = options.password(&secret(&path)?);
    }
    let db = PgPoolOptions::new()
        .max_connections(12)
        .connect_with(options)
        .await?;
    Migrator::new(std::path::Path::new("migrations"))
        .await?
        .run(&db)
        .await?;
    match env::args().nth(1).as_deref() {
        Some("bootstrap") => return provision(&db, true).await,
        Some("criar-usuario") => return provision(&db, false).await,
        Some("migrate") => return Ok(()),
        None | Some("receber-webhooks") => (),
        _ => anyhow::bail!("Comando desconhecido"),
    }
    let origin = config::var("APP_ORIGIN", "http://127.0.0.1:8080");
    ensure!(
        !secure || origin.starts_with("https://"),
        "Produção exige origem HTTPS"
    );
    let bank: Arc<dyn BankProvider> = if config::var("INTER_ENV", "mock") == "mock" {
        Arc::new(MockBank(db.clone()))
    } else {
        Arc::new(infrastructure::banking::inter::InterBank::configured(db.clone()).await?)
    };
    tracing::info!(
        ambiente = bank.environment(),
        "Provedor de leitura configurado"
    );
    let app = App {
        db,
        secure,
        origin,
        dummy: Arc::new(hash(&Uuid::new_v4().to_string())?),
        workers: Arc::new(Semaphore::new(4)),
        bank,
        mfa_key: Arc::new(encryption_key(&env::var("MFA_ENCRYPTION_KEY_FILE")?)?),
    };
    if env::args().nth(1).as_deref() == Some("receber-webhooks") {
        return recebimentos::webhook::serve(app).await;
    }
    if app.bank.environment() == "MOCK" {
        tokio::spawn(recebimentos::worker::run(app.clone()));
    }
    let router = router(app);
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        config::var("APP_HOST", "127.0.0.1"),
        config::var("APP_PORT", "8080")
    ))
    .await?;
    tracing::info!("Servidor iniciado");
    axum::serve(listener, router).await?;
    Ok(())
}

fn router(app: App) -> Router {
    let mut routes = Router::new();
    if app.bank.sandbox_issuance() {
        routes = routes.route(
            "/api/sandbox/recebimentos/emitir",
            post(recebimentos::api::sandbox_issue),
        );
    }
    if app.bank.environment() == "MOCK" {
        routes = routes
            .route(
                "/api/mock/financeiro",
                get(financeiro::list).post(financeiro::create),
            )
            .route(
                "/api/mock/financeiro/relatorio.csv",
                get(financeiro::report_csv),
            )
            .route(
                "/api/mock/financeiro/recorrencias",
                get(financeiro::list_recurring).post(financeiro::create_recurring),
            )
            .route(
                "/api/mock/financeiro/recorrencias/gerar",
                post(financeiro::generate_recurring),
            )
            .route(
                "/api/mock/financeiro/recorrencias/alterar",
                post(financeiro::change_recurring),
            )
            .route(
                "/api/mock/categorias",
                get(financeiro::categories).post(financeiro::create_category),
            )
            .route(
                "/api/mock/centros-custo",
                get(financeiro::cost_centers).post(financeiro::create_cost_center),
            )
            .route("/api/mock/financeiro/alterar", post(financeiro::change))
            .route(
                "/api/mock/financeiro/lembretes",
                get(financeiro::list_reminders).post(financeiro::reminders),
            )
            .route(
                "/api/mock/pagamentos/pix",
                get(pagamentos::list).post(pagamentos::create),
            )
            .route("/api/mock/pagamentos/pix/revisar", post(pagamentos::review))
            .route(
                "/api/mock/pagamentos/pix/aprovar",
                post(pagamentos::approve),
            )
            .route("/api/mock/pix/cobrancas", post(api::issue_mock_pix))
            .route(
                "/api/mock/recebimentos/pix",
                post(recebimentos::api::simulate_pix),
            )
            .route("/api/mock/cobrancas-v3", post(recebimentos::api::billing))
            .route(
                "/api/mock/cobrancas-v3/atualizar",
                post(recebimentos::api::change_billing),
            );
    }
    routes
        .route("/api/recebimentos", get(recebimentos::api::list))
        .route("/api/clientes", post(recebimentos::api::customer))
        .route(
            "/api/recebimentos/reprocessar",
            post(recebimentos::api::retry),
        )
        .route("/health", get(|| async { "ok" }))
        .route("/ready", get(api::ready))
        .route("/metrics", get(api::metrics))
        .route("/api/login", post(auth::login))
        .route("/api/mfa", get(mfa::status))
        .route("/api/mfa/iniciar", post(mfa::begin_enrollment))
        .route("/api/mfa/confirmar", post(mfa::confirm_enrollment))
        .route("/api/mfa/desativar", post(mfa::disable))
        .route("/api/logout", post(auth::logout))
        .route("/api/session", get(api::session))
        .route("/api/dashboard", get(api::dashboard))
        .route("/api/ambiente", get(api::environment))
        .route("/api/pix/cobrancas/consultar", post(api::pix_charge))
        .route("/api/extrato", get(api::statement))
        .route("/api/sincronizar", post(api::sync))
        .route("/api/auditoria", get(api::audit))
        .layer(DefaultBodyLimit::max(8192))
        .layer(middleware::from_fn_with_state(app.clone(), api::guard))
        .with_state(app)
}

#[cfg(all(test, feature = "testes-postgres"))]
#[path = "../tests/postgres/seguranca.rs"]
mod testes_postgres;
