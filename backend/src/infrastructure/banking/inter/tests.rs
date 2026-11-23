use super::*;
use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
};

#[derive(Default)]
struct MockState {
    grants: AtomicUsize,
    balances: AtomicUsize,
    scenario: AtomicUsize,
}
async fn token(
    State(state): State<Arc<MockState>>,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if fields.len() != 4
        || fields.get("grant_type").map(String::as_str) != Some("client_credentials")
        || fields.get("scope").map(String::as_str) != Some("extrato.read")
        || fields.get("client_id").map(String::as_str) != Some("cliente-teste")
        || fields.get("client_secret").map(String::as_str) != Some("segredo-sintetico")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let n = state.grants.fetch_add(1, SeqCst) + 1;
    Json(json!({"access_token":format!("sintetico-{n}"),"scope":"extrato.read","token_type":"Bearer","expires_in":if state.scenario.load(SeqCst)==1{1}else{3600}})).into_response()
}
fn authorized(h: &HeaderMap) -> bool {
    h.get("authorization")
        .is_some_and(|v| v.as_bytes().starts_with(b"Bearer sintetico-"))
        && h.get("x-conta-corrente").is_some_and(|v| v == "123")
}
async fn balance(State(state): State<Arc<MockState>>, h: HeaderMap) -> Response {
    if !authorized(&h) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let n = state.balances.fetch_add(1, SeqCst);
    match state.scenario.load(SeqCst) {
        2 if n == 0 => (StatusCode::TOO_MANY_REQUESTS, [("retry-after", "0")]).into_response(),
        3 => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        4 => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Json(json!({"disponivel":0})).into_response()
        }
        5 if n == 0 => StatusCode::UNAUTHORIZED.into_response(),
        _ => Json(json!({"disponivel":120.25,"bloqueadoCheque":0,"dataReferencia":"03/10/2026"}))
            .into_response(),
    }
}
async fn statement(
    State(state): State<Arc<MockState>>,
    h: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if !authorized(&h) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if params.get("tamanhoPagina").map(String::as_str) != Some("500")
        || !params.contains_key("dataInicio")
        || !params.contains_key("dataFim")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let page = &params["pagina"];
    let last = page == "1";
    if last && state.scenario.load(SeqCst) == 6 {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let id = if state.scenario.load(SeqCst) == 7 {
        "0"
    } else {
        page.as_str()
    };
    Json(json!({"totalElementos":2,"numeroDeElementos":1,"ultimaPagina":last,"transacoes":[{
 "idTransacao":id,"dataTransacao":params["dataFim"],"tipoTransacao":"PIX","tipoOperacao":if last{"D"}else{"C"},"valor":"145.38"}]})).into_response()
}
async fn server(scenario: usize) -> (ClientInter, Arc<MockState>) {
    let state = Arc::new(MockState::default());
    state.scenario.store(scenario, SeqCst);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/oauth/v2/token", post(token))
        .route("/banking/v2/saldo", get(balance))
        .route("/banking/v2/extrato/completo", get(statement))
        .with_state(state.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(if scenario == 4 {
            120
        } else {
            2000
        }))
        .build()
        .unwrap();
    (
        ClientInter::new(
            http,
            base,
            "123".into(),
            "cliente-teste".into(),
            Zeroizing::new("segredo-sintetico".into()),
        )
        .fast_mock(),
        state,
    )
}
#[tokio::test]
async fn cache_concorrente_e_renovacao() {
    let (client, state) = server(1).await;
    let (a, b) = tokio::join!(client.balance(), client.balance());
    assert_eq!(a.unwrap().available, "12025");
    assert!(b.is_ok());
    assert_eq!(state.grants.load(SeqCst), 1);
    tokio::time::sleep(Duration::from_millis(1010)).await;
    client.balance().await.unwrap();
    assert_eq!(state.grants.load(SeqCst), 2);
}
#[tokio::test]
async fn trata_limite_autorizacao_indisponibilidade_e_timeout() {
    for scenario in [2, 3, 4, 5] {
        let (client, state) = server(scenario).await;
        let result = client.balance().await;
        match scenario {
            2 => {
                assert!(result.is_ok());
                assert_eq!(state.balances.load(SeqCst), 2)
            }
            3 => assert!(matches!(result, Err(BankError::Http(503)))),
            4 => assert!(matches!(result, Err(BankError::Timeout))),
            _ => {
                assert!(result.is_ok());
                assert_eq!(state.grants.load(SeqCst), 2)
            }
        }
    }
}
#[tokio::test]
async fn exige_paginas_completas_e_ids_unicos() {
    let day = OffsetDateTime::now_utc().date();
    let (client, state) = server(0).await;
    let records = client.statement(day, day).await.unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].signed_cents().unwrap(), -14538);
    state.scenario.store(7, SeqCst);
    assert!(matches!(
        client.statement(day, day).await,
        Err(BankError::Contract)
    ));
}
#[cfg(feature = "testes-postgres")]
#[tokio::test]
async fn sincronizacao_atomica_e_vinculo_da_conta() -> anyhow::Result<()> {
    use sqlx_postgres::{PgConnectOptions, PgPoolOptions};
    use std::str::FromStr;
    let options = PgConnectOptions::from_str(&env::var("TEST_DATABASE_URL")?)?;
    ensure!(
        options.get_host() == "127.0.0.1" && options.get_database() == Some("inter_pj_test"),
        "Banco descartável obrigatório"
    );
    let pool = PgPoolOptions::new().connect_with(options).await?;
    sqlx_core::migrate::Migrator::new(Path::new("migrations"))
        .await?
        .run(&pool)
        .await?;
    let company = Uuid::new_v4();
    let actor = Uuid::new_v4();
    query("INSERT INTO companies VALUES($1,'Empresa sintética')")
        .bind(company)
        .execute(&pool)
        .await?;
    query("INSERT INTO users(id,company_id,email,password_hash) VALUES($1,$2,$3,'INATIVO')")
        .bind(actor)
        .bind(company)
        .bind(format!("{actor}@local.test"))
        .execute(&pool)
        .await?;
    let (client, state) = server(0).await;
    let account = Uuid::new_v4();
    let bank = InterBank {
        db: pool.clone(),
        company,
        account,
        mode: "SANDBOX",
        client,
        expires: OffsetDateTime::now_utc() + time::Duration::days(7),
    };
    let binding = Uuid::new_v4();
    bank.bind_account(binding.as_bytes()).await?;
    assert!(bank.bind_account(Uuid::new_v4().as_bytes()).await.is_err());
    assert!(matches!(
        bank.snapshot(Uuid::new_v4()).await,
        Err(BankError::Account)
    ));
    assert!(bank.dashboard().await?.reading.is_none());
    let (first, second) = tokio::join!(
        bank.synchronize(company, actor),
        bank.synchronize(company, actor)
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(BankError::Busy)) | (Err(BankError::Busy), Ok(()))
    ));
    bank.synchronize(company, actor).await?;
    let list = bank.list(StatementQuery::default()).await?;
    assert_eq!(list.entries.len(), 2);
    let before = bank.dashboard().await?.reading.unwrap();
    state.scenario.store(6, SeqCst);
    assert!(bank.synchronize(company, actor).await.is_err());
    let after = bank.dashboard().await?;
    assert_eq!(
        before.synchronized_at,
        after.reading.unwrap().synchronized_at
    );
    assert_eq!(after.last_error.as_deref(), Some("HTTP"));
    Ok(())
}

#[tokio::test]
async fn mtls_exige_cliente_e_cadeia_confiavel() -> anyhow::Result<()> {
    use std::{
        io::{BufRead, BufReader},
        process::{Child, Command, Stdio},
    };
    struct Server(Child);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = tempfile::tempdir()?;
    let mut server = Server(
        Command::new("python3")
            .args(["tests/fixtures/mtls.py"])
            .arg(directory.path())
            .stdout(Stdio::piped())
            .spawn()?,
    );
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().context("Porta de teste ausente")?)
        .read_line(&mut line)?;
    let base = format!("https://127.0.0.1:{}", line.trim().parse::<u16>()?);
    let read = |file| std::fs::read(directory.path().join(file));
    let ca = reqwest::Certificate::from_pem(&read("ca.pem")?)?;
    let mut pem = Zeroizing::new(read("client.pem")?);
    pem.extend(read("client.key")?);
    for (trusted, identified) in [(true, false), (false, true), (true, true)] {
        let mut builder = InterBank::http_builder();
        if trusted {
            builder = builder.add_root_certificate(ca.clone());
        }
        if identified {
            builder = builder.identity(Identity::from_pem(&pem)?);
        }
        let client = ClientInter::new(
            builder.build()?,
            base.clone(),
            "123".into(),
            "teste".into(),
            Zeroizing::new("teste".into()),
        )
        .fast_mock();
        let result = client.balance().await;
        if trusted && identified {
            assert_eq!(result?.available, "1025")
        } else {
            assert!(matches!(result, Err(BankError::Transport)))
        }
    }
    Ok(())
}
