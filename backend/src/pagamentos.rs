use crate::{
    App,
    api::{Error, Result, positive_cents},
    auth::{self, Identity},
};
use axum::{
    Json,
    extract::{
        Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx_core::{query::query, query_as::query_as, transaction::Transaction};
use sqlx_postgres::Postgres;
use time::OffsetDateTime;
use uuid::Uuid;

const PAGE_SIZE: i64 = 25;
const MAX_PAGE: u32 = 1000;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Recipient {
    FornecedorDemo,
    ServicosDemo,
}

impl Recipient {
    fn code(self) -> &'static str {
        match self {
            Self::FornecedorDemo => "FORNECEDOR_DEMO",
            Self::ServicosDemo => "SERVICOS_DEMO",
        }
    }
}

#[derive(Serialize)]
struct RecipientOption {
    id: Recipient,
    nome: &'static str,
}

const RECIPIENTS: [RecipientOption; 2] = [
    RecipientOption {
        id: Recipient::FornecedorDemo,
        nome: "Fornecedor fictício",
    },
    RecipientOption {
        id: Recipient::ServicosDemo,
        nome: "Serviços fictícios",
    },
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    intent_id: Uuid,
    destinatario: Recipient,
    valor_centavos: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    id: Uuid,
    destinatario: Recipient,
    valor_centavos: String,
}

#[derive(Deserialize)]
pub struct Page {
    page: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
    Rascunho,
    EmRevisao,
}

#[derive(Serialize)]
pub struct Intent {
    id: Uuid,
    destinatario: String,
    valor_centavos: String,
    situacao: Status,
    #[serde(with = "time::serde::rfc3339")]
    criado_em: OffsetDateTime,
    pode_revisar: bool,
}

#[derive(Serialize)]
pub struct IntentList {
    destinatarios: [RecipientOption; 2],
    intencoes: Vec<Intent>,
    pode_preparar: bool,
    tem_proxima: bool,
}

type IntentRow = (Uuid, String, i64, OffsetDateTime, Option<OffsetDateTime>, Uuid);

fn can_prepare(user: &Identity) -> bool {
    user.roles
        .iter()
        .any(|role| matches!(role.as_str(), "ADMIN" | "OPERATOR"))
}

impl Intent {
    fn from_row(
        (id, recipient, cents, created_at, reviewed_at, creator_id): IntentRow,
        user: &Identity,
    ) -> Self {
        Self {
            id,
            destinatario: recipient,
            valor_centavos: cents.to_string(),
            situacao: if reviewed_at.is_some() {
                Status::EmRevisao
            } else {
                Status::Rascunho
            },
            criado_em: created_at,
            pode_revisar: reviewed_at.is_none() && creator_id == user.id && can_prepare(user),
        }
    }
}

async fn session(app: &App, headers: &HeaderMap, write: bool) -> Result<Identity> {
    if app.bank.environment() != "MOCK" {
        return Err(Error::Missing);
    }
    let user = auth::identity(app, headers).await?;
    if write && !can_prepare(&user) {
        return Err(Error::Forbidden);
    }
    Ok(user)
}

pub async fn list(
    State(app): State<App>,
    headers: HeaderMap,
    filter: std::result::Result<Query<Page>, QueryRejection>,
) -> Result<Json<IntentList>> {
    let user = session(&app, &headers, false).await?;
    let Query(filter) = filter.map_err(|err| Error::Invalid(err.status()))?;
    let page = filter.page.unwrap_or(1);
    if !(1..=MAX_PAGE).contains(&page) {
        return Err(Error::Invalid(StatusCode::BAD_REQUEST));
    }
    let mut rows: Vec<IntentRow> = query_as(
        "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id
         FROM mock_payment_intents WHERE company_id=$1
         ORDER BY created_at DESC,id DESC LIMIT $2 OFFSET $3",
    )
    .bind(user.company_id)
    .bind(PAGE_SIZE + 1)
    .bind(i64::from(page - 1) * PAGE_SIZE)
    .fetch_all(&app.db)
    .await?;
    let tem_proxima = page < MAX_PAGE && rows.len() > PAGE_SIZE as usize;
    rows.truncate(PAGE_SIZE as usize);
    Ok(Json(IntentList {
        destinatarios: RECIPIENTS,
        intencoes: rows
            .into_iter()
            .map(|row| Intent::from_row(row, &user))
            .collect(),
        pode_preparar: can_prepare(&user),
        tem_proxima,
    }))
}

pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Create>, JsonRejection>,
) -> Result<(StatusCode, Json<Intent>)> {
    let user = session(&app, &headers, true).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    let amount = positive_cents(&input.valor_centavos)?;
    let mut tx = app.db.begin().await?;
    let inserted: Option<IntentRow> = query_as(
        "INSERT INTO mock_payment_intents (id,company_id,creator_id,idempotency_key,recipient,amount_cents)
         VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (company_id,creator_id,idempotency_key) DO NOTHING
         RETURNING id,recipient,amount_cents,created_at,review_requested_at,creator_id",
    )
    .bind(Uuid::new_v4())
    .bind(user.company_id)
    .bind(user.id)
    .bind(input.intent_id)
    .bind(input.destinatario.code())
    .bind(amount)
    .fetch_optional(&mut *tx)
    .await?;
    let (status, row) = if let Some(row) = inserted {
        audit(&mut tx, &user, row.0, "Intenção Pix fictícia criada").await?;
        (StatusCode::CREATED, row)
    } else {
        let same: IntentRow = query_as(
            "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id
             FROM mock_payment_intents WHERE company_id=$1 AND creator_id=$2
             AND idempotency_key=$3 AND recipient=$4 AND amount_cents=$5",
        )
        .bind(user.company_id)
        .bind(user.id)
        .bind(input.intent_id)
        .bind(input.destinatario.code())
        .bind(amount)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Conflict)?;
        (StatusCode::OK, same)
    };
    tx.commit().await?;
    Ok((status, Json(Intent::from_row(row, &user))))
}

pub async fn review(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Review>, JsonRejection>,
) -> Result<Json<Intent>> {
    let user = session(&app, &headers, true).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    let amount = positive_cents(&input.valor_centavos)?;
    let mut tx = app.db.begin().await?;
    let updated: Option<IntentRow> = query_as(
        "UPDATE mock_payment_intents SET review_requested_at=now()
         WHERE company_id=$1 AND creator_id=$2 AND id=$3 AND recipient=$4
         AND amount_cents=$5 AND review_requested_at IS NULL
         RETURNING id,recipient,amount_cents,created_at,review_requested_at,creator_id",
    )
    .bind(user.company_id)
    .bind(user.id)
    .bind(input.id)
    .bind(input.destinatario.code())
    .bind(amount)
    .fetch_optional(&mut *tx)
    .await?;
    let row = if let Some(row) = updated {
        audit(
            &mut tx,
            &user,
            row.0,
            "Revisão de intenção Pix fictícia solicitada",
        )
        .await?;
        row
    } else {
        let existing: IntentRow = query_as(
            "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id
             FROM mock_payment_intents WHERE company_id=$1 AND creator_id=$2 AND id=$3",
        )
        .bind(user.company_id)
        .bind(user.id)
        .bind(input.id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Missing)?;
        if existing.1 != input.destinatario.code() || existing.2 != amount || existing.4.is_none() {
            return Err(Error::Conflict);
        }
        existing
    };
    tx.commit().await?;
    Ok(Json(Intent::from_row(row, &user)))
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    user: &Identity,
    id: Uuid,
    action: &str,
) -> Result<()> {
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(user.company_id)
        .bind(user.id)
        .bind(format!("{action} ({id})"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalogo_so_aceita_identificadores_ficticios() {
        for option in RECIPIENTS {
            let encoded = serde_json::to_value(option.id).unwrap();
            assert_eq!(encoded, option.id.code());
        }
        let received = json!({
            "intent_id": Uuid::new_v4(),
            "destinatario": "chave-externa",
            "valor_centavos": "100"
        });
        assert!(serde_json::from_value::<Create>(received).is_err());
    }

    #[test]
    fn so_autor_habilitado_pode_pedir_revisao_de_rascunho() {
        let author = Uuid::new_v4();
        let mut user = Identity {
            id: author,
            company_id: Uuid::new_v4(),
            company: String::new(),
            email: String::new(),
            roles: vec!["OPERATOR".into()],
        };
        let now = OffsetDateTime::UNIX_EPOCH;
        let row = (Uuid::new_v4(), "FORNECEDOR_DEMO".into(), 42, now, None, author);
        let draft = serde_json::to_value(Intent::from_row(row.clone(), &user)).unwrap();
        assert_eq!(draft["situacao"], "RASCUNHO");
        assert_eq!(draft["valor_centavos"], "42");
        assert_eq!(draft["pode_revisar"], true);

        let mut reviewed = row.clone();
        reviewed.4 = Some(now);
        let review = serde_json::to_value(Intent::from_row(reviewed, &user)).unwrap();
        assert_eq!(review["situacao"], "EM_REVISAO");
        assert_eq!(review["pode_revisar"], false);

        user.roles = vec!["APPROVER".into()];
        assert!(!Intent::from_row(row.clone(), &user).pode_revisar);
        user.roles = vec!["ADMIN".into()];
        user.id = Uuid::new_v4();
        assert!(!Intent::from_row(row, &user).pode_revisar);
    }
}
