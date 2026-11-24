use super::{Scope, enqueue, valid_e2e};
use crate::App;
use anyhow::{Context, Result, ensure};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self, RootCertStore,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
        server::WebPkiClientVerifier,
    },
};
use uuid::Uuid;

#[derive(Clone)]
struct Receiver {
    app: App,
    scope: Scope,
    number: Arc<String>,
}
#[derive(Deserialize)]
struct PixBatch {
    pix: Vec<PixNotice>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PixNotice {
    end_to_end_id: String,
    #[serde(default)]
    devolucoes: Vec<Refund>,
}
#[derive(Deserialize, Serialize)]
struct Refund {
    id: String,
    status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillingNotice {
    codigo_solicitacao: Uuid,
    situacao: String,
    data_hora_situacao: String,
}

fn account(s: &Receiver, h: &HeaderMap) -> Result<(), StatusCode> {
    let mut values = h.get_all("x-conta-corrente").iter();
    if values.next().and_then(|v| v.to_str().ok()) != Some(s.number.as_str())
        || values.next().is_some()
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
type Input<T> = Result<Json<T>, JsonRejection>;
async fn save(
    s: &Receiver,
    family: &str,
    items: Vec<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = s
        .app
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    for (reference, key) in items {
        enqueue(&mut tx, s.scope, family, &reference, &key)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    tx.commit()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(StatusCode::OK)
}
async fn pix(
    State(s): State<Receiver>,
    h: HeaderMap,
    body: Input<PixBatch>,
) -> Result<StatusCode, StatusCode> {
    account(&s, &h)?;
    let batch = body.map_err(|e| e.status())?.0.pix;
    if !(1..=256).contains(&batch.len()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut items = Vec::with_capacity(batch.len());
    for mut p in batch {
        if !valid_e2e(&p.end_to_end_id) {
            return Err(StatusCode::BAD_REQUEST);
        }
        p.devolucoes.sort_by(|a, b| a.id.cmp(&b.id));
        let bytes = serde_json::to_vec(&p).map_err(|_| StatusCode::BAD_REQUEST)?;
        items.push((p.end_to_end_id, format!("{:x}", Sha256::digest(bytes))));
    }
    save(&s, "PIX", items).await
}
async fn billing(
    State(s): State<Receiver>,
    h: HeaderMap,
    body: Input<Vec<BillingNotice>>,
) -> Result<StatusCode, StatusCode> {
    account(&s, &h)?;
    let batch = body.map_err(|e| e.status())?.0;
    if !(1..=256).contains(&batch.len()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut items = Vec::with_capacity(batch.len());
    for b in batch {
        if b.situacao.is_empty()
            || b.situacao.len() > 64
            || b.data_hora_situacao.is_empty()
            || b.data_hora_situacao.len() > 64
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        let key = Sha256::digest(format!(
            "{}:{}:{}",
            b.codigo_solicitacao, b.situacao, b.data_hora_situacao
        ));
        items.push((b.codigo_solicitacao.to_string(), format!("{key:x}")));
    }
    save(&s, "V3", items).await
}
fn tls(dir: &Path) -> Result<TlsAcceptor> {
    let chain = |name: &str| -> Result<Vec<CertificateDer<'static>>> {
        Ok(CertificateDer::pem_file_iter(dir.join(name))?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    };
    let mut roots = RootCertStore::empty();
    for c in chain("inter-webhook-ca.crt")? {
        roots.add(c)?
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verify =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone()).build()?;
    let file = dir.join("server.key");
    ensure!(
        file.metadata()?.permissions().mode() & 0o037 == 0,
        "Permissões da chave TLS inválidas"
    );
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_client_cert_verifier(verify)
        .with_single_cert(chain("server.crt")?, PrivateKeyDer::from_pem_file(file)?)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

pub async fn serve(app: App) -> Result<()> {
    let (company, account_id) = app
        .bank
        .collection_binding()
        .context("Receptor exige integração Inter externa")?;
    let mode = app.bank.environment();
    let acceptor = tls(Path::new(
        &env::var("WEBHOOK_TLS_DIR").context("Configure WEBHOOK_TLS_DIR")?,
    ))?;
    let port: u16 = crate::config::var("WEBHOOK_PORT", "9443").parse()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let receiver = Receiver {
        app: app.clone(),
        scope: Scope {
            company,
            account: Some(account_id),
            mode,
        },
        number: Arc::new(env::var("INTER_ACCOUNT_NUMBER")?),
    };
    let routes = Router::new()
        .route("/inter/pix", post(pix))
        .route("/inter/cobranca", post(billing))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(receiver);
    tokio::spawn(super::worker::run(app));
    let slots = Arc::new(Semaphore::new(64));
    tracing::info!(port, "Receptor mTLS em loopback");
    loop {
        let permit = slots.clone().acquire_owned().await?;
        let (stream, _) = listener.accept().await?;
        let tls = acceptor.clone();
        let svc = hyper_util::service::TowerToHyperService::new(routes.clone());
        tokio::spawn(async move {
            let _permit = permit;
            if let Ok(Ok(socket)) =
                tokio::time::timeout(Duration::from_secs(8), tls.accept(stream)).await
            {
                let connection = hyper::server::conn::http1::Builder::new()
                    .keep_alive(false)
                    .serve_connection(hyper_util::rt::TokioIo::new(socket), svc);
                if !matches!(
                    tokio::time::timeout(Duration::from_secs(25), connection).await,
                    Ok(Ok(()))
                ) {
                    tracing::warn!("Conexão de callback interrompida");
                }
            }
        });
    }
}
