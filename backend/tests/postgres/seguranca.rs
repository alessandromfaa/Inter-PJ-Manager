use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

// A URL, o nome do banco e a origem são fixos ou validados antes de gravar.
async fn chamada(
    app: &Router,
    path: &str,
    cookie: &str,
    body: Option<Value>,
    origin: &str,
) -> axum::response::Response {
    let mut request = Request::builder().uri(path).header(header::COOKIE, cookie);
    let payload = if let Some(v) = body {
        request = request
            .method("POST")
            .header(header::ORIGIN, origin)
            .header("x-pj-request", "1")
            .header(header::CONTENT_TYPE, "application/json");
        Body::from(v.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(payload).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn sessoes_permissoes_e_segregacao() -> Result<()> {
    let options = PgConnectOptions::from_str(&env::var("TEST_DATABASE_URL")?)?;
    ensure!(
        options.get_host() == "127.0.0.1" && options.get_database() == Some("inter_pj_test"),
        "Use um PostgreSQL descartável em loopback com banco inter_pj_test"
    );
    let pool = PgPoolOptions::new().connect_with(options).await?;
    Migrator::new(std::path::Path::new("migrations"))
        .await?
        .run(&pool)
        .await?;
    let tenant = Uuid::new_v4();
    let user = Uuid::new_v4();
    let pw = Uuid::new_v4().to_string();
    let email = format!("{user}@local.test");
    let other = Uuid::new_v4();
    let actor = Uuid::new_v4();
    for (company, person, mail) in [
        (tenant, user, email.clone()),
        (other, actor, format!("{actor}@local.test")),
    ] {
        query("INSERT INTO companies VALUES ($1,'Teste isolado')")
            .bind(company)
            .execute(&pool)
            .await?;
        query("INSERT INTO users(id,company_id,email,password_hash) VALUES($1,$2,$3,$4)")
            .bind(person)
            .bind(company)
            .bind(mail)
            .bind(hash(&pw)?)
            .execute(&pool)
            .await?;
        query("INSERT INTO user_roles VALUES ($1,'VIEWER')")
            .bind(person)
            .execute(&pool)
            .await?;
    }
    let marker = Uuid::new_v4().to_string();
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(other)
        .bind(actor)
        .bind(&marker)
        .execute(&pool)
        .await?;
    let origin = "https://mock.local.test";
    let api = router(App {
        db: pool.clone(),
        secure: true,
        origin: origin.into(),
        dummy: Arc::new(hash(&pw)?),
        workers: Arc::new(Semaphore::new(4)),
        bank: Arc::new(MockBank(pool.clone())),
        mfa_key: Arc::new([0; 32]),
    });
    let creds = json!({"email":email,"password":pw});
    assert_eq!(
        chamada(&api, "/api/dashboard", "", None, origin)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        chamada(
            &api,
            "/api/login",
            "",
            Some(creds.clone()),
            "https://outra.invalid"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let login = chamada(&api, "/api/login", "", Some(creds.clone()), origin).await;
    assert_eq!(login.status(), StatusCode::OK);
    let raw = login.headers()[header::SET_COOKIE].to_str()?;
    for flag in ["HttpOnly", "Secure", "SameSite=Strict"] {
        assert!(raw.contains(flag));
    }
    let cookie = raw.split(';').next().unwrap();
    assert_eq!(
        chamada(&api, "/api/dashboard", cookie, None, origin)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        chamada(&api, "/api/auditoria", cookie, None, origin)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    exercitar_emissao_mock(&api, &pool, cookie, origin, tenant, user, other).await?;
    let pix = "/api/pix/cobrancas/consultar";
    let demo = json!({"txid":"DEMO000000000000000000000001"});
    for (cred, payload, source, expected) in [
        ("", demo.clone(), origin, StatusCode::UNAUTHORIZED),
        (
            cookie,
            demo.clone(),
            "https://outra.invalid",
            StatusCode::FORBIDDEN,
        ),
        (
            cookie,
            json!({"txid":"curto"}),
            origin,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (cookie, demo.clone(), origin, StatusCode::OK),
    ] {
        assert_eq!(
            chamada(&api, pix, cred, Some(payload), source)
                .await
                .status(),
            expected
        );
    }
    let audited: i64 = query_scalar("SELECT count(*) FROM audit_logs WHERE company_id=$1 AND actor=$2 AND action='Cobrança Pix consultada em MOCK'")
        .bind(tenant).bind(user).fetch_one(&pool).await?;
    assert_eq!(audited, 1);
    query("UPDATE user_roles SET role='AUDITOR' WHERE user_id=$1 AND role='VIEWER'")
        .bind(user)
        .execute(&pool)
        .await?;
    assert_eq!(
        chamada(&api, pix, cookie, Some(demo), origin)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    query("UPDATE user_roles SET role='VIEWER' WHERE user_id=$1 AND role='AUDITOR'")
        .bind(user)
        .execute(&pool)
        .await?;
    query("INSERT INTO user_roles VALUES ($1,'AUDITOR')")
        .bind(user)
        .execute(&pool)
        .await?;
    let audit = chamada(&api, "/api/auditoria", cookie, None, origin).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let events = String::from_utf8(to_bytes(audit.into_body(), 65536).await?.to_vec())?;
    assert!(events.contains("Sessão iniciada") && !events.contains(&marker));
    assert_eq!(
        chamada(&api, "/api/logout", cookie, Some(json!({})), origin)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        chamada(&api, "/api/session", cookie, None, origin)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let again = chamada(&api, "/api/login", "", Some(creds.clone()), origin).await;
    assert_eq!(again.status(), StatusCode::OK);
    let active = again.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .unwrap();
    query("UPDATE user_sessions SET expires_at=now()-interval '1 second' WHERE user_id=$1")
        .bind(user)
        .execute(&pool)
        .await?;
    assert_eq!(
        chamada(&api, "/api/session", active, None, origin)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let invalid = json!({"email":email,"password":""});
    let (first, second) = tokio::join!(
        chamada(&api, "/api/login", "", Some(invalid.clone()), origin),
        chamada(&api, "/api/login", "", Some(invalid), origin)
    );
    assert_eq!(first.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
    for _ in 0..3 {
        assert_eq!(
            chamada(
                &api,
                "/api/login",
                "",
                Some(json!({"email":email,"password":""})),
                origin
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        chamada(&api, "/api/login", "", Some(creds), origin)
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    Ok(())
}

async fn exercitar_emissao_mock(
    api: &Router,
    pool: &PgPool,
    cookie: &str,
    origin: &str,
    company: Uuid,
    actor: Uuid,
    other: Uuid,
) -> Result<()> {
    const ROUTE: &str = "/api/mock/pix/cobrancas";
    let id = Uuid::new_v4();
    let request = |cents: &str| json!({"intent_id":id,"original_cents":cents});
    assert_eq!(
        chamada(api, ROUTE, cookie, Some(request("4250")), origin)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    query("INSERT INTO user_roles VALUES ($1, 'OPERATOR')")
        .bind(actor)
        .execute(pool)
        .await?;
    assert_eq!(
        chamada(
            api,
            ROUTE,
            cookie,
            Some(request("4250")),
            "https://outra.invalid"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for invalid in ["0", "-10", "4.25", "9999999999999", "01", ""] {
        assert_eq!(
            chamada(api, ROUTE, cookie, Some(request(invalid)), origin)
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let send = || chamada(api, ROUTE, cookie, Some(request("4250")), origin);
    let (first, second) = tokio::join!(send(), send());
    let mut statuses = [first.status(), second.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CREATED]);
    let body = to_bytes(first.into_body(), 8192).await?;
    assert_eq!(body, to_bytes(second.into_body(), 8192).await?);
    let charge: Value = serde_json::from_slice(&body)?;
    let txid = domain::pix::Txid::for_mock_intent(id);
    assert_eq!(charge["txid"], txid.as_str());
    assert_eq!(charge["source"], "MOCK");
    assert_eq!(charge["status"], "ATIVA");
    assert_eq!(charge["original_cents"], "4250");
    assert!(
        charge["mock_code"]["copy_paste"]
            .as_str()
            .unwrap()
            .contains("pix.invalid/")
    );
    assert_eq!(
        chamada(api, ROUTE, cookie, Some(request("4251")), origin)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let read = infrastructure::banking::mock_pix::find(
        pool,
        company,
        domain::pix::Txid::for_mock_intent(id),
    )
    .await?;
    assert_eq!(serde_json::to_value(read)?, charge);
    assert!(matches!(
        infrastructure::banking::mock_pix::find(
            pool,
            other,
            domain::pix::Txid::for_mock_intent(id),
        )
        .await,
        Err(infrastructure::banking::inter::BankError::PixNotFound)
    ));
    let n: i64 = query_scalar("SELECT count(*) FROM audit_logs WHERE company_id=$1 AND actor=$2 AND action='Cobrança Pix emitida em MOCK'")
        .bind(company).bind(actor).fetch_one(pool).await?;
    assert_eq!(n, 1);
    exercitar_recebimentos_mock(api, pool, cookie, origin, company, txid).await?;
    query("DELETE FROM user_roles WHERE user_id=$1 AND role='OPERATOR'")
        .bind(actor)
        .execute(pool)
        .await?;
    Ok(())
}

async fn exercitar_recebimentos_mock(
    api: &Router,
    pool: &PgPool,
    cookie: &str,
    origin: &str,
    company: Uuid,
    txid: domain::pix::Txid,
) -> Result<()> {
    let receive = || {
        chamada(
            api,
            "/api/mock/recebimentos/pix",
            cookie,
            Some(json!({"txid":txid.as_str()})),
            origin,
        )
    };
    let (first, second) = tokio::join!(receive(), receive());
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    let first: Value = serde_json::from_slice(&to_bytes(first.into_body(), 8192).await?)?;
    let second: Value = serde_json::from_slice(&to_bytes(second.into_body(), 8192).await?)?;
    assert_eq!(first, second);
    let e2e = first["end_to_end_id"].as_str().unwrap();
    assert_eq!(e2e.len(), 32);
    query("UPDATE collection_events SET next_at='-infinity' WHERE company_id=$1 AND family='PIX' AND reference=$2 AND state='AGUARDANDO'")
        .bind(company).bind(e2e).execute(pool).await?;
    assert!(
        recebimentos::worker::process_one(&App {
            db: pool.clone(),
            secure: true,
            origin: origin.into(),
            dummy: Arc::new(String::new()),
            workers: Arc::new(Semaphore::new(1)),
            bank: Arc::new(MockBank(pool.clone())),
            mfa_key: Arc::new([0; 32]),
        })
        .await?
    );
    let result: (String, i64, String) = sqlx_core::query_as::query_as(
        "SELECT reconciliation,received_cents,status FROM collection_results WHERE company_id=$1 AND family='PIX' AND reference=$2",
    )
    .bind(company)
    .bind(e2e)
    .fetch_one(pool)
    .await?;
    assert_eq!(result, ("CONCILIADO".into(), 4250, "RECEBIDO".into()));
    let pix_events: i64 = query_scalar(
        "SELECT count(*) FROM collection_events WHERE company_id=$1 AND family='PIX' AND reference=$2",
    )
    .bind(company)
    .bind(e2e)
    .fetch_one(pool)
    .await?;
    assert_eq!(pix_events, 1);

    let customer = Uuid::new_v4();
    assert_eq!(
        chamada(
            api,
            "/api/clientes",
            cookie,
            Some(json!({"id":customer,"name":"Cliente de teste"})),
            origin,
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let code = Uuid::new_v4();
    let billing = json!({
        "intent_id":code,
        "customer_id":customer,
        "original_cents":"9876",
        "due_on":(time::OffsetDateTime::now_utc()+time::Duration::days(3)).date(),
    });
    let issued = chamada(
        api,
        "/api/mock/cobrancas-v3",
        cookie,
        Some(billing.clone()),
        origin,
    )
    .await;
    assert_eq!(issued.status(), StatusCode::ACCEPTED);
    let replay = chamada(api, "/api/mock/cobrancas-v3", cookie, Some(billing), origin).await;
    assert_eq!(replay.status(), StatusCode::OK);
    query("UPDATE collection_events SET next_at='-infinity' WHERE company_id=$1 AND family='V3' AND reference=$2 AND state='AGUARDANDO'")
        .bind(company).bind(code.to_string()).execute(pool).await?;
    let app = App {
        db: pool.clone(),
        secure: true,
        origin: origin.into(),
        dummy: Arc::new(String::new()),
        workers: Arc::new(Semaphore::new(1)),
        bank: Arc::new(MockBank(pool.clone())),
        mfa_key: Arc::new([0; 32]),
    };
    assert!(recebimentos::worker::process_one(&app).await?);
    let pending: String = query_scalar(
        "SELECT status FROM collection_results WHERE company_id=$1 AND family='V3' AND reference=$2",
    )
    .bind(company)
    .bind(code.to_string())
    .fetch_one(pool)
    .await?;
    assert_eq!(pending, "A_RECEBER");
    assert_eq!(
        chamada(
            api,
            "/api/mock/cobrancas-v3/atualizar",
            cookie,
            Some(json!({"code":code,"action":"PIX"})),
            origin,
        )
        .await
        .status(),
        StatusCode::ACCEPTED
    );
    query("UPDATE collection_events SET next_at='-infinity' WHERE company_id=$1 AND family='V3' AND reference=$2 AND state='AGUARDANDO'")
        .bind(company).bind(code.to_string()).execute(pool).await?;
    assert!(recebimentos::worker::process_one(&app).await?);
    let paid: (String, i64, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT reconciliation,received_cents,channel FROM collection_results WHERE company_id=$1 AND family='V3' AND reference=$2",
    )
    .bind(company)
    .bind(code.to_string())
    .fetch_one(pool)
    .await?;
    assert_eq!(paid, ("CONCILIADO".into(), 9876, Some("PIX".into())));
    let v3_events: i64 = query_scalar(
        "SELECT count(*) FROM collection_events WHERE company_id=$1 AND family='V3' AND reference=$2",
    )
    .bind(company)
    .bind(code.to_string())
    .fetch_one(pool)
    .await?;
    assert_eq!(v3_events, 2);
    Ok(())
}
