mod client;
mod contract;
pub use contract::Balance;
mod store;
#[cfg(test)]
mod tests;

use crate::{
    application::BankProvider,
    config::var,
    domain::{Dashboard, LiveDashboard, Statement, StatementQuery},
};
use anyhow::{Context, Result, ensure};
use client::ClientInter;
use reqwest::{Client, Identity};
use sha2::{Digest, Sha256};
use sqlx_core::{query::query, query_scalar::query_scalar};
use sqlx_postgres::PgPool;
use std::{env, fs::File, io::Read, os::unix::fs::PermissionsExt, path::Path, time::Duration};
use time::OffsetDateTime;
use uuid::Uuid;
use x509_parser::pem::parse_x509_pem;
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum BankError {
    #[error("A conta não está vinculada a esta empresa.")]
    Account,
    #[error("Outra sincronização está em andamento.")]
    Busy,
    #[error("O Inter limitou as consultas. Aguarde antes de sincronizar novamente.")]
    Limit,
    #[error("Tempo de resposta do Inter excedido.")]
    Timeout,
    #[error("Falha de conexão segura com o Inter.")]
    Transport,
    #[error("O Inter respondeu com HTTP {0}.")]
    Http(u16),
    #[error("A resposta bancária diverge do contrato.")]
    Contract,
    #[error("O volume excedeu o limite da importação.")]
    Volume,
    #[error("O certificado da integração está vencido.")]
    Certificate,
    #[error("Falha ao persistir a leitura bancária.")]
    Database(#[from] sqlx_core::Error),
    #[error("Operação indisponível no ambiente MOCK.")]
    Mock,
    #[error("Período ou página inválidos.")]
    Period,
}
impl BankError {
    fn transport(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else {
            Self::Transport
        }
    }
    pub fn code(&self) -> &'static str {
        match self {
            Self::Account => "CONTA",
            Self::Busy => "EM_ANDAMENTO",
            Self::Limit => "LIMITE",
            Self::Timeout => "TEMPO_ESGOTADO",
            Self::Transport => "CONEXAO",
            Self::Http(401 | 403) => "AUTORIZACAO",
            Self::Http(_) => "HTTP",
            Self::Contract => "CONTRATO",
            Self::Volume => "VOLUME",
            Self::Certificate => "CERTIFICADO",
            Self::Database(_) => "PERSISTENCIA",
            Self::Mock => "MOCK",
            Self::Period => "PERIODO",
        }
    }
}
pub struct InterBank {
    db: PgPool,
    company: Uuid,
    account: Uuid,
    mode: &'static str,
    client: ClientInter,
    expires: OffsetDateTime,
}
fn material(path: &Path, private: bool) -> Result<Zeroizing<Vec<u8>>> {
    let file = File::open(path).context("Arquivo da integração inacessível")?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 131072,
        "Arquivo da integração inválido"
    );
    if private {
        ensure!(
            metadata.permissions().mode() & 0o037 == 0,
            "Chave e segredo exigem modo 0600 ou 0640"
        );
    }
    let mut data = Zeroizing::new(Vec::new());
    file.take(131073).read_to_end(&mut data)?;
    ensure!(
        !data.is_empty() && data.len() <= 131072,
        "Arquivo da integração inválido"
    );
    Ok(data)
}
impl InterBank {
    pub async fn configured(db: PgPool) -> Result<Self> {
        let (mode, base) = match var("INTER_ENV", "mock").as_str() {
            "sandbox" => ("SANDBOX", "https://cdpj-sandbox.partners.uatinter.co"),
            "production" => ("PRODUCTION", "https://cdpj.partners.bancointer.com.br"),
            _ => anyhow::bail!("Ambiente Inter inválido"),
        };
        let company = env::var("INTER_COMPANY_ID")?.parse()?;
        let account = env::var("INTER_ACCOUNT_ID")?.parse()?;
        let number = env::var("INTER_ACCOUNT_NUMBER")?;
        ensure!(
            !number.is_empty()
                && !number.starts_with('0')
                && number.len() <= 32
                && number.bytes().all(|b| b.is_ascii_digit()),
            "Número de conta inválido"
        );
        let directory = std::path::PathBuf::from(env::var("INTER_CREDENTIALS_ROOT")?)
            .join(mode.to_ascii_lowercase());
        let id = material(&directory.join("client_id"), false)?;
        let client_id = std::str::from_utf8(&id)?.trim().to_string();
        let secret = material(&directory.join("client_secret"), true)?;
        let secret = Zeroizing::new(
            std::str::from_utf8(&secret)?
                .trim_end_matches(['\r', '\n'])
                .to_string(),
        );
        ensure!(
            !client_id.is_empty() && !secret.is_empty(),
            "Credenciais vazias"
        );
        let mut bundle = material(&directory.join("cert.pem"), false)?;
        let (_, pem) =
            parse_x509_pem(&bundle).map_err(|_| anyhow::anyhow!("Certificado PEM inválido"))?;
        let cert = pem
            .parse_x509()
            .map_err(|_| anyhow::anyhow!("Certificado X.509 inválido"))?;
        ensure!(
            cert.validity().is_valid(),
            "Certificado fora do prazo de validade"
        );
        let expires = OffsetDateTime::from_unix_timestamp(cert.validity().not_after.timestamp())
            .context("Validade do certificado inválida")?;
        bundle.push(b'\n');
        bundle.extend_from_slice(&material(&directory.join("key.pem"), true)?);
        let identity =
            Identity::from_pem(&bundle).map_err(|_| anyhow::anyhow!("Identidade mTLS inválida"))?;
        let http = Self::http_builder()
            .identity(identity)
            .build()
            .context("Falha ao carregar identidade mTLS")?;
        let binding = Sha256::digest(format!("{mode}:{number}")).to_vec();
        let bank = Self {
            db,
            company,
            account,
            mode,
            expires,
            client: ClientInter::new(http, base.into(), number, client_id, secret),
        };
        bank.bind_account(&binding).await?;
        Ok(bank)
    }
    fn http_builder() -> reqwest::ClientBuilder {
        Client::builder()
            .use_rustls_tls()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(12))
            .timeout(Duration::from_secs(20))
    }
    async fn bind_account(&self, binding: &[u8]) -> Result<()> {
        let mut tx = self.db.begin().await?;
        query("INSERT INTO bank_accounts(id,company_id,label) VALUES($1,$2,'Conta Inter PJ') ON CONFLICT(id) DO NOTHING")
    .bind(self.account).bind(self.company).execute(&mut *tx).await?;
        let id:Option<Uuid>=query_scalar("INSERT INTO bank_integrations(id,company_id,account_id,environment,certificate_expires_at,binding_digest)
       VALUES($1,$2,$3,$4::text::integration_environment,$5,$6)
       ON CONFLICT(account_id) DO UPDATE SET certificate_expires_at=excluded.certificate_expires_at
       WHERE bank_integrations.company_id=excluded.company_id AND bank_integrations.binding_digest=excluded.binding_digest AND bank_integrations.environment=excluded.environment RETURNING id")
    .bind(Uuid::new_v4()).bind(self.company).bind(self.account).bind(self.mode).bind(self.expires).bind(binding).fetch_optional(&mut *tx).await?;
        ensure!(id.is_some(), "Conta já vinculada a outra integração");
        tx.commit().await?;
        Ok(())
    }
    fn tenant(&self, company: Uuid) -> Result<(), BankError> {
        if self.company == company {
            Ok(())
        } else {
            Err(BankError::Account)
        }
    }
}
#[async_trait::async_trait]
impl BankProvider for InterBank {
    fn environment(&self) -> &'static str {
        self.mode
    }
    async fn snapshot(&self, company: Uuid) -> Result<Dashboard, BankError> {
        self.tenant(company)?;
        Ok(Dashboard::Inter(self.dashboard().await?))
    }
    async fn synchronize(&self, company: Uuid, actor: Uuid) -> Result<(), BankError> {
        self.tenant(company)?;
        self.run_sync(actor).await
    }
    async fn statement(
        &self,
        company: Uuid,
        filter: StatementQuery,
    ) -> Result<Statement, BankError> {
        self.tenant(company)?;
        self.list(filter).await
    }
}
