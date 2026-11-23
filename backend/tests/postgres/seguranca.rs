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
        bank: Arc::new(MockBank),
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
