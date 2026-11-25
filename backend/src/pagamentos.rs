use crate::{
    App,
    api::{Error, Result, positive_cents},
    auth::{self, Identity},
    mfa,
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
use sqlx_core::{
    query::query, query_as::query_as, query_scalar::query_scalar, transaction::Transaction,
};
use sqlx_postgres::Postgres;
use time::OffsetDateTime;
use uuid::Uuid;

const PAGE_SIZE: i64 = 25;
const MAX_PAGE: u32 = 1000;
const MOCK_APPROVAL_LIMIT_CENTS: i64 = 10_000_000;

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
#[serde(deny_unknown_fields)]
pub struct Approve {
    id: Uuid,
    destinatario: Recipient,
    valor_centavos: String,
    chave_idempotencia: Uuid,
    codigo_mfa: String,
}

#[derive(Serialize)]
pub struct ApprovalResult {
    id: Uuid,
    situacao: &'static str,
    envio_bancario: bool,
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
    Aprovado,
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
    pode_aprovar: bool,
}

#[derive(Serialize)]
pub struct IntentList {
    destinatarios: [RecipientOption; 2],
    intencoes: Vec<Intent>,
    pode_preparar: bool,
    tem_proxima: bool,
}

type IntentRow = (
    Uuid,
    String,
    i64,
    OffsetDateTime,
    Option<OffsetDateTime>,
    Uuid,
    Option<OffsetDateTime>,
    Option<Uuid>,
);
type ApprovalRow = (
    Uuid,
    String,
    i64,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<Uuid>,
    Option<Uuid>,
);

fn can_prepare(user: &Identity) -> bool {
    user.roles
        .iter()
        .any(|role| matches!(role.as_str(), "ADMIN" | "OPERATOR"))
}

fn can_approve(creator_id: Uuid, approver: &Identity, amount_cents: i64) -> bool {
    creator_id != approver.id && approver_allowed(approver, amount_cents)
}

fn approver_allowed(approver: &Identity, amount_cents: i64) -> bool {
    approver
        .roles
        .iter()
        .any(|role| matches!(role.as_str(), "ADMIN" | "APPROVER"))
        && (1..=MOCK_APPROVAL_LIMIT_CENTS).contains(&amount_cents)
}

fn can_record_approval(
    creator_id: Uuid,
    reviewed_at: Option<OffsetDateTime>,
    approved_at: Option<OffsetDateTime>,
    approver: &Identity,
    amount_cents: i64,
) -> bool {
    reviewed_at.is_some()
        && approved_at.is_none()
        && can_approve(creator_id, approver, amount_cents)
}

impl Intent {
    fn from_row(
        (id, recipient, cents, created_at, reviewed_at, creator_id, approved_at, _approved_by): IntentRow,
        user: &Identity,
    ) -> Self {
        Self {
            id,
            destinatario: recipient,
            valor_centavos: cents.to_string(),
            situacao: if approved_at.is_some() {
                Status::Aprovado
            } else if reviewed_at.is_some() {
                Status::EmRevisao
            } else {
                Status::Rascunho
            },
            criado_em: created_at,
            pode_revisar: reviewed_at.is_none() && creator_id == user.id && can_prepare(user),
            pode_aprovar: reviewed_at.is_some()
                && approved_at.is_none()
                && can_approve(creator_id, user, cents),
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
        "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id,approved_at,approved_by
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
         RETURNING id,recipient,amount_cents,created_at,review_requested_at,creator_id,approved_at,approved_by",
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
            "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id,approved_at,approved_by
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
         RETURNING id,recipient,amount_cents,created_at,review_requested_at,creator_id,approved_at,approved_by",
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
            "SELECT id,recipient,amount_cents,created_at,review_requested_at,creator_id,approved_at,approved_by
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

pub async fn approve(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Approve>, JsonRejection>,
) -> Result<(StatusCode, Json<ApprovalResult>)> {
    let user = session(&app, &headers, false).await?;
    let Json(input) = body.map_err(|err| Error::Invalid(err.status()))?;
    let amount = positive_cents(&input.valor_centavos)?;
    if !approver_allowed(&user, amount) {
        return Err(Error::Forbidden);
    }
    let mut tx = app.db.begin().await?;
    let row: Option<ApprovalRow> = query_as(
        "SELECT creator_id,recipient,amount_cents,review_requested_at,approved_at,approved_by,approval_idempotency_key
         FROM mock_payment_intents WHERE company_id=$1 AND id=$2 FOR UPDATE")
        .bind(user.company_id).bind(input.id).fetch_optional(&mut *tx).await?;
    let Some((creator, recipient, cents, reviewed_at, approved_at, approved_by, previous_key)) =
        row
    else {
        return Err(Error::Missing);
    };
    if recipient != input.destinatario.code() || cents != amount {
        return Err(Error::Conflict);
    }
    if approved_at.is_some() {
        if approved_by == Some(user.id) && previous_key == Some(input.chave_idempotencia) {
            tx.commit().await?;
            return Ok((
                StatusCode::OK,
                Json(ApprovalResult {
                    id: input.id,
                    situacao: "APROVADO_MOCK",
                    envio_bancario: false,
                }),
            ));
        }
        return Err(Error::Conflict);
    }
    if !can_record_approval(creator, reviewed_at, approved_at, &user, amount) {
        return Err(Error::Conflict);
    }
    let reused_key: bool = query_scalar("SELECT EXISTS(SELECT FROM mock_payment_intents WHERE company_id=$1 AND approved_by=$2 AND approval_idempotency_key=$3)")
        .bind(user.company_id).bind(user.id).bind(input.chave_idempotencia).fetch_one(&mut *tx).await?;
    if reused_key {
        return Err(Error::Conflict);
    }
    if !mfa::consume_code(&mut tx, &app, &user, &input.codigo_mfa).await? {
        tx.commit().await?;
        return Err(Error::Unauthorized);
    }
    query("UPDATE mock_payment_intents SET approved_at=now(),approved_by=$3,approval_idempotency_key=$4 WHERE company_id=$1 AND id=$2 AND approved_at IS NULL")
        .bind(user.company_id).bind(input.id).bind(user.id).bind(input.chave_idempotencia).execute(&mut *tx).await?;
    audit(
        &mut tx,
        &user,
        input.id,
        "Intenção Pix fictícia aprovada; nenhum envio bancário realizado",
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::OK,
        Json(ApprovalResult {
            id: input.id,
            situacao: "APROVADO_MOCK",
            envio_bancario: false,
        }),
    ))
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
        let row = (
            Uuid::new_v4(),
            "FORNECEDOR_DEMO".into(),
            42,
            now,
            None,
            author,
            None,
            None,
        );
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

    #[test]
    fn aprovacao_exige_outro_usuario_perfil_e_limite() {
        let creator = Uuid::new_v4();
        let mut approver = Identity {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            company: String::new(),
            email: String::new(),
            roles: vec!["APPROVER".into()],
        };
        assert!(can_approve(creator, &approver, 1));
        assert!(!can_approve(creator, &approver, 10_000_001));
        assert!(!can_approve(creator, &approver, 0));
        approver.id = creator;
        assert!(!can_approve(creator, &approver, 1));
        approver.id = Uuid::new_v4();
        approver.roles = vec!["OPERATOR".into()];
        assert!(!can_approve(creator, &approver, 1));
    }

    #[test]
    fn registro_de_aprovacao_exige_revisao_previa_e_estado_pendente() {
        let creator = Uuid::new_v4();
        let approver = Identity {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            company: String::new(),
            email: String::new(),
            roles: vec!["APPROVER".into()],
        };
        let now = OffsetDateTime::UNIX_EPOCH;
        assert!(can_record_approval(
            creator,
            Some(now),
            None,
            &approver,
            10_000
        ));
        assert!(!can_record_approval(creator, None, None, &approver, 10_000));
        assert!(!can_record_approval(
            creator,
            Some(now),
            Some(now),
            &approver,
            10_000
        ));
        assert!(!can_record_approval(
            approver.id,
            Some(now),
            None,
            &approver,
            10_000
        ));
    }
}
