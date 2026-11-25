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
    let scope = fields.get("scope").map(String::as_str);
    if fields.len() != 4
        || fields.get("grant_type").map(String::as_str) != Some("client_credentials")
        || !matches!(
            scope,
            Some(
                "extrato.read"
                    | "cob.read"
                    | "pix.read"
                    | "boleto-cobranca.read"
                    | "cob.write"
                    | "boleto-cobranca.write"
                    | "rec.read"
            )
        )
        || fields.get("client_id").map(String::as_str) != Some("cliente-teste")
        || fields.get("client_secret").map(String::as_str) != Some("segredo-sintetico")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let n = state.grants.fetch_add(1, SeqCst) + 1;
    let requested = scope.unwrap();
    let granted = if requested == "cob.read" && state.scenario.load(SeqCst) == 13 {
        "extrato.read"
    } else {
        requested
    };
    let prefix = requested.replace(['.', '-'], "_");
    Json(json!({"access_token":format!("{prefix}-{n}"),"scope":granted,"token_type":"Bearer","expires_in":if state.scenario.load(SeqCst)==1{1}else{3600}})).into_response()
}
fn authorized(h: &HeaderMap) -> bool {
    h.get("authorization")
        .is_some_and(|v| v.as_bytes().starts_with(b"Bearer extrato_read-"))
        && h.get("x-conta-corrente").is_some_and(|v| v == "123")
}
fn authorized_scope(h: &HeaderMap, scope: &str) -> bool {
    let prefix = format!("Bearer {}-", scope.replace(['.', '-'], "_"));
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with(&prefix))
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
    let scenario = state.scenario.load(SeqCst);
    if matches!(scenario, 8 | 9) {
        return Json(
            json!({"totalPaginas":0,"totalElementos":if scenario==8 {0} else {1},
            "numeroDeElementos":0,"ultimaPagina":false,"transacoes":[]}),
        )
        .into_response();
    }
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
const PIX_ID: &str = "TESTE00000000000000000000001";
const REC_ID: &str = "RR1234567820240115abcdefghijk";
async fn recurrence(
    State(state): State<Arc<MockState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    h: HeaderMap,
) -> Response {
    if !authorized_scope(&h, "rec.read") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let scenario = state.scenario.load(SeqCst);
    if id != REC_ID {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(json!({"idRec": if scenario == 31 { "RR1234567820240115abcdefghijX" } else { REC_ID }, "status": if scenario == 32 { "INVALIDA" } else { "APROVADA" }})).into_response()
}
async fn pix(
    State(state): State<Arc<MockState>>,
    axum::extract::Path(txid): axum::extract::Path<String>,
    h: HeaderMap,
) -> Response {
    if h.get("x-conta-corrente").is_none_or(|v| v != "123")
        || !h
            .get("authorization")
            .is_some_and(|v| v.as_bytes().starts_with(b"Bearer cob_read-"))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let scenario = state.scenario.load(SeqCst);
    if txid != PIX_ID || scenario == 10 {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(json!({
        "txid": if scenario == 11 { "TESTE00000000000000000000002" } else { PIX_ID },
        "status": if scenario == 14 { "DESCONHECIDA" } else { "CONCLUIDA" },
        "revisao":1, "valor":{"original": if scenario == 12 { "0.001" } else { "123.45" }}
    }))
    .into_response()
}
const E2E_ID: &str = "E1234567890123456789012345678901";
const BILL_ID: &str = "1c0ca501-63bb-47d4-bb22-7c72ab79ab2f";
async fn issue_pix(
    axum::extract::Path(txid): axum::extract::Path<String>,
    h: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if !authorized_scope(&h, "cob.write") || txid != PIX_ID {
        return StatusCode::FORBIDDEN.into_response();
    }
    (
        StatusCode::CREATED,
        Json(json!({"txid":txid,"status":"ATIVA","revisao":0,"valor":{"original":body["valor"]["original"]}})),
    )
        .into_response()
}
async fn issue_bill(h: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
    if !authorized_scope(&h, "boleto-cobranca.write") || body["valorNominal"] != 98.76 {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(json!({"codigoSolicitacao":BILL_ID})).into_response()
}
async fn received_pix(
    axum::extract::Path(e2e): axum::extract::Path<String>,
    h: HeaderMap,
) -> Response {
    if !authorized_scope(&h, "pix.read") || e2e != E2E_ID {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(json!({"endToEndId":e2e,"txid":PIX_ID,"valor":"123.45","horario":"2026-11-24T12:00:00Z","devolucoes":[]})).into_response()
}
async fn received_bill(
    axum::extract::Path(id): axum::extract::Path<String>,
    h: HeaderMap,
) -> Response {
    if !authorized_scope(&h, "boleto-cobranca.read") || id != BILL_ID {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(json!({"cobranca":{"codigoSolicitacao":id,"situacao":"RECEBIDO","valorNominal":"98.76","valorTotalRecebido":"98.76","origemRecebimento":"BOLETO"}})).into_response()
}
async fn server(scenario: usize) -> (ClientInter, Arc<MockState>) {
    let state = Arc::new(MockState::default());
    state.scenario.store(scenario, SeqCst);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/oauth/v2/token", post(token))
        .route("/banking/v2/saldo", get(balance))
        .route("/pix/v2/cob/{txid}", get(pix).put(issue_pix))
        .route("/pix/v2/pix/{e2e}", get(received_pix))
        .route("/pix/v2/rec/{id}", get(recurrence))
        .route("/cobranca/v3/cobrancas", post(issue_bill))
        .route("/cobranca/v3/cobrancas/{id}", get(received_bill))
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
async fn consulta_recorrencia_valida_identificador_escopo_e_resposta() {
    let (client, state) = server(0).await;
    assert_eq!(client.recurrence(REC_ID).await.unwrap().status, "APROVADA");
    assert_eq!(client.recurrence(REC_ID).await.unwrap().id_rec, REC_ID);
    assert_eq!(state.grants.load(SeqCst), 1);
    assert!(matches!(
        client.recurrence("RR1234567820240115abcdefghijX").await,
        Err(BankError::RecurrenceNotFound)
    ));
    assert!(matches!(
        client.recurrence("RRinválido").await,
        Err(BankError::Period)
    ));
    for scenario in [31, 32] {
        let (client, _) = server(scenario).await;
        assert!(matches!(
            client.recurrence(REC_ID).await,
            Err(BankError::Contract)
        ));
    }
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
#[tokio::test]
async fn aceita_zero_paginas_somente_com_extrato_vazio() {
    let day = today();
    let (client, state) = server(8).await;
    assert!(client.statement(day, day).await.unwrap().is_empty());
    state.scenario.store(9, SeqCst);
    assert!(matches!(
        client.statement(day, day).await,
        Err(BankError::Contract)
    ));
}
#[tokio::test]
async fn pix_consulta_com_token_especifico_e_cache_isolado() {
    let (client, state) = server(0).await;
    let charge = || client.pix_charge(PIX_ID.to_owned().try_into().unwrap(), "SANDBOX");
    let (balance, first, second) = tokio::join!(client.balance(), charge(), charge());
    assert!(balance.is_ok());
    let data = first.unwrap();
    assert_eq!(data.original_cents, "12345");
    assert_eq!(data.txid.as_str(), PIX_ID);
    assert!(matches!(
        data.status,
        crate::domain::pix::PixStatus::Concluida
    ));
    assert!(second.is_ok());
    assert_eq!(state.grants.load(SeqCst), 2);
    for scenario in [10, 11, 12, 14] {
        state.scenario.store(scenario, SeqCst);
        let result = charge().await;
        if scenario == 10 {
            assert!(matches!(result, Err(BankError::PixNotFound)));
        } else {
            assert!(matches!(result, Err(BankError::Contract)));
        }
    }
    let (client, _) = server(13).await;
    assert!(matches!(
        client
            .pix_charge(PIX_ID.to_owned().try_into().unwrap(), "SANDBOX")
            .await,
        Err(BankError::Contract)
    ));
}
#[tokio::test]
async fn contratos_de_emissao_e_confirmacao_de_recebimentos() {
    let (client, state) = server(0).await;
    let pix = client
        .submit_collection(
            Some(PIX_ID),
            &json!({"calendario":{"expiracao":86400},"valor":{"original":"123.45"},"chave":"chave-sintetica"}),
        )
        .await
        .unwrap();
    assert_eq!(pix, PIX_ID);
    let billing = client
        .submit_collection(
            None,
            &json!({"valorNominal":98.76,"pagador":{"nome":"Cliente sintético"}}),
        )
        .await
        .unwrap();
    assert_eq!(billing, BILL_ID);

    let received = client.confirm_receipt("PIX", E2E_ID).await.unwrap();
    assert_eq!(received.txid.as_deref(), Some(PIX_ID));
    assert_eq!(received.status, "RECEBIDO");
    assert_eq!(received.gross, 12345);
    assert_eq!(received.received, 12345);
    assert_eq!(received.channel.as_deref(), Some("PIX"));

    let bill = client.confirm_receipt("V3", BILL_ID).await.unwrap();
    assert_eq!(bill.status, "RECEBIDO");
    assert_eq!(bill.gross, 9876);
    assert_eq!(bill.received, 9876);
    assert_eq!(bill.channel.as_deref(), Some("BOLETO"));
    assert_eq!(state.grants.load(SeqCst), 4);
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
        pix_lookup_enabled: false,
        db: pool.clone(),
        company,
        account,
        mode: "SANDBOX",
        client,
        expires: OffsetDateTime::now_utc() + time::Duration::days(7),
    };
    let pix_id = || PIX_ID.to_owned().try_into().unwrap();
    assert!(matches!(
        bank.pix_charge(company, pix_id()).await,
        Err(BankError::PixDisabled)
    ));
    assert!(matches!(
        bank.pix_charge(Uuid::new_v4(), pix_id()).await,
        Err(BankError::Account)
    ));
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

#[tokio::test]
async fn emissao_mock_ausente_nos_dois_ambientes_externos() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    for mode in ["SANDBOX", "PRODUCTION"] {
        let db = sqlx_postgres::PgPoolOptions::new()
            .connect_lazy_with(sqlx_postgres::PgConnectOptions::new());
        let (client, state) = server(0).await;
        let bank = InterBank {
            mode,
            company: Uuid::new_v4(),
            account: Uuid::new_v4(),
            db: db.clone(),
            client,
            expires: OffsetDateTime::now_utc() + time::Duration::days(7),
            pix_lookup_enabled: true,
        };
        let origin = "https://ensaio.invalid";
        let app = crate::router(crate::App {
            db,
            secure: true,
            origin: origin.into(),
            dummy: Arc::new(String::new()),
            workers: Arc::new(tokio::sync::Semaphore::new(1)),
            bank: Arc::new(bank),
            mfa_key: Arc::new([0; 32]),
        });
        let request = Request::post("/api/mock/pix/cobrancas")
            .header("origin", origin)
            .header("x-pj-request", "1")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(state.grants.load(SeqCst), 0);
    }
}
