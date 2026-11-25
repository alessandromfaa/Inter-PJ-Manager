use crate::{
    App,
    api::{Error, Result},
    auth::{self, Identity},
};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx_core::{
    query::query, query_as::query_as, query_scalar::query_scalar, transaction::Transaction,
};
use sqlx_postgres::Postgres;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    tipo: String,
    parte: String,
    descricao: String,
    valor_centavos: String,
    vencimento: Date,
    categoria_id: Option<Uuid>,
    centro_custo_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    id: Uuid,
    acao: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurringRuleCreate {
    tipo: String,
    parte: String,
    descricao: String,
    valor_centavos: String,
    dia_vencimento: u8,
    categoria_id: Option<Uuid>,
    centro_custo_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurringGenerate {
    ano: i32,
    mes: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecurringRuleChange {
    id: Uuid,
    ativo: bool,
}

#[derive(Serialize)]
pub struct RecurringRule {
    id: Uuid,
    tipo: String,
    parte: String,
    descricao: String,
    valor_centavos: String,
    dia_vencimento: i16,
    ativo: bool,
    categoria: Option<String>,
    centro_custo: Option<String>,
}

#[derive(Serialize)]
pub struct RecurringGenerateResult {
    pub criados: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    nome: String,
}

#[derive(Serialize)]
pub struct NamedItem {
    id: Uuid,
    nome: String,
}

pub async fn categories(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Vec<NamedItem>>> {
    named_list(&app, &headers, "mock_financial_categories").await
}

pub async fn cost_centers(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Vec<NamedItem>>> {
    named_list(&app, &headers, "mock_cost_centers").await
}

async fn named_list(app: &App, headers: &HeaderMap, table: &str) -> Result<Json<Vec<NamedItem>>> {
    let identity = user(app, headers, false).await?;
    let sql: &'static str = match table {
        "mock_financial_categories" => {
            "SELECT id,name FROM mock_financial_categories WHERE company_id=$1 ORDER BY name,id LIMIT 200"
        }
        "mock_cost_centers" => {
            "SELECT id,name FROM mock_cost_centers WHERE company_id=$1 ORDER BY name,id LIMIT 200"
        }
        _ => return Err(Error::Internal),
    };
    let rows: Vec<(Uuid, String)> = query_as(sql)
        .bind(identity.company_id)
        .fetch_all(&app.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| NamedItem {
                id: row.0,
                nome: row.1,
            })
            .collect(),
    ))
}

pub async fn create_category(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Named>, JsonRejection>,
) -> Result<(StatusCode, Json<NamedItem>)> {
    create_named(&app, &headers, body, "mock_financial_categories").await
}

pub async fn create_cost_center(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Named>, JsonRejection>,
) -> Result<(StatusCode, Json<NamedItem>)> {
    create_named(&app, &headers, body, "mock_cost_centers").await
}

async fn create_named(
    app: &App,
    headers: &HeaderMap,
    body: std::result::Result<Json<Named>, JsonRejection>,
    table: &str,
) -> Result<(StatusCode, Json<NamedItem>)> {
    let identity = user(app, headers, true).await?;
    let Json(input) = body.map_err(|error| Error::Invalid(error.status()))?;
    let name = input.nome.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let sql: &'static str = match table {
        "mock_financial_categories" => {
            "INSERT INTO mock_financial_categories(id,company_id,name) VALUES($1,$2,$3) ON CONFLICT(company_id,name) DO NOTHING RETURNING id,name"
        }
        "mock_cost_centers" => {
            "INSERT INTO mock_cost_centers(id,company_id,name) VALUES($1,$2,$3) ON CONFLICT(company_id,name) DO NOTHING RETURNING id,name"
        }
        _ => return Err(Error::Internal),
    };
    let mut tx = app.db.begin().await?;
    let row: Option<(Uuid, String)> = query_as(sql)
        .bind(Uuid::new_v4())
        .bind(identity.company_id)
        .bind(name)
        .fetch_optional(&mut *tx)
        .await?;
    let row = row.ok_or(Error::Conflict)?;
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(format!("Classificação MOCK criada: {}", row.1))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(NamedItem {
            id: row.0,
            nome: row.1,
        }),
    ))
}

#[derive(Serialize)]
pub struct Item {
    id: Uuid,
    tipo: String,
    parte: String,
    descricao: String,
    valor_centavos: String,
    vencimento: Date,
    situacao: String,
    #[serde(with = "time::serde::rfc3339")]
    criado_em: OffsetDateTime,
    liquidado_em: Option<OffsetDateTime>,
    categoria: Option<String>,
    centro_custo: Option<String>,
}

#[derive(Deserialize)]
pub struct ReportPeriod {
    inicio: Date,
    fim: Date,
}

pub async fn report_csv(
    State(app): State<App>,
    headers: HeaderMap,
    period: std::result::Result<
        axum::extract::Query<ReportPeriod>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Result<impl axum::response::IntoResponse> {
    let identity = user(&app, &headers, false).await?;
    let axum::extract::Query(period) = period.map_err(|error| Error::Invalid(error.status()))?;
    if period.fim < period.inicio || (period.fim - period.inicio).whole_days() > 366 {
        return Err(Error::Invalid(StatusCode::BAD_REQUEST));
    }
    let rows: Vec<ReportRow> = query_as(
        "SELECT i.kind::text,i.party_name,i.description,i.amount_cents,i.due_on,i.state::text,c.name,cc.name
         FROM mock_financial_items i
         LEFT JOIN mock_financial_categories c ON (c.company_id,c.id)=(i.company_id,i.category_id)
         LEFT JOIN mock_cost_centers cc ON (cc.company_id,cc.id)=(i.company_id,i.cost_center_id)
         WHERE i.company_id=$1 AND i.due_on BETWEEN $2 AND $3 ORDER BY i.due_on,i.id LIMIT 10001",
    ).bind(identity.company_id).bind(period.inicio).bind(period.fim).fetch_all(&app.db).await?;
    if rows.len() > 10_000 {
        return Err(Error::Invalid(StatusCode::PAYLOAD_TOO_LARGE));
    }
    let mut csv = String::from(
        "tipo;parte;descricao;valor_centavos;vencimento;situacao;categoria;centro_custo\r\n",
    );
    for row in rows {
        let fields = [
            row.0,
            row.1,
            row.2,
            row.3.to_string(),
            row.4.to_string(),
            row.5,
            row.6.unwrap_or_default(),
            row.7.unwrap_or_default(),
        ];
        csv.push_str(&fields.map(|field| csv_field(&field)).join(";"));
        csv.push_str("\r\n");
    }
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=relatorio-financeiro-mock.csv",
            ),
        ],
        format!("\u{feff}{csv}"),
    ))
}

fn csv_field(value: &str) -> String {
    let dangerous = value
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_control())
        .starts_with(['=', '+', '-', '@']);
    let prefix = if dangerous { "'" } else { "" };
    format!("\"{}{}\"", prefix, value.replace('"', "\"\""))
}

#[cfg(test)]
mod csv_tests {
    use super::csv_field;

    #[test]
    fn protects_spreadsheet_formulas_and_preserves_csv_escaping() {
        for value in ["=1+1", "  +1", "\t@cmd", "\r-1"] {
            assert!(csv_field(value).starts_with("\"'"));
        }
        assert_eq!(
            csv_field("Acme \"Sul\"; filial"),
            "\"Acme \"\"Sul\"\"; filial\""
        );
        assert_eq!(csv_field("123"), "\"123\"");
    }
}

type ReportRow = (
    String,
    String,
    String,
    i64,
    Date,
    String,
    Option<String>,
    Option<String>,
);

type Row = (
    Uuid,
    String,
    String,
    String,
    i64,
    Date,
    String,
    OffsetDateTime,
    Option<OffsetDateTime>,
    Option<String>,
    Option<String>,
);

type RecurringRuleRow = (
    Uuid,
    String,
    String,
    String,
    i64,
    i16,
    bool,
    Option<String>,
    Option<String>,
);

type RecurringRuleInsertRow = (
    Uuid,
    String,
    String,
    String,
    i64,
    i16,
    Option<Uuid>,
    Option<Uuid>,
);

async fn user(app: &App, headers: &HeaderMap, write: bool) -> Result<Identity> {
    if app.bank.environment() != "MOCK" {
        return Err(Error::Missing);
    }
    let identity = auth::identity(app, headers).await?;
    if write
        && !identity
            .roles
            .iter()
            .any(|r| matches!(r.as_str(), "ADMIN" | "OPERATOR"))
    {
        return Err(Error::Forbidden);
    }
    Ok(identity)
}

pub async fn list(State(app): State<App>, headers: HeaderMap) -> Result<Json<Vec<Item>>> {
    let identity = user(&app, &headers, false).await?;
    let rows: Vec<Row> = query_as(
        "SELECT i.id,i.kind::text,i.party_name,i.description,i.amount_cents,i.due_on,i.state::text,i.created_at,i.settled_at,c.name,cc.name
         FROM mock_financial_items i
         LEFT JOIN mock_financial_categories c ON (c.company_id,c.id)=(i.company_id,i.category_id)
         LEFT JOIN mock_cost_centers cc ON (cc.company_id,cc.id)=(i.company_id,i.cost_center_id)
         WHERE i.company_id=$1 ORDER BY i.due_on,i.id LIMIT 200",
    ).bind(identity.company_id).fetch_all(&app.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Item {
                id: r.0,
                tipo: r.1,
                parte: r.2,
                descricao: r.3,
                valor_centavos: r.4.to_string(),
                vencimento: r.5,
                situacao: r.6,
                criado_em: r.7,
                liquidado_em: r.8,
                categoria: r.9,
                centro_custo: r.10,
            })
            .collect(),
    ))
}

pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Create>, JsonRejection>,
) -> Result<(StatusCode, Json<Item>)> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let cents = input
        .valor_centavos
        .parse::<i64>()
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    if !(1..=999_999_999_999).contains(&cents)
        || input.parte.trim().is_empty()
        || input.parte.len() > 160
        || input.descricao.trim().is_empty()
        || input.descricao.len() > 240
        || !matches!(input.tipo.as_str(), "PAGAR" | "RECEBER")
    {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let mut tx = app.db.begin().await?;
    let row: Row = query_as(
        "INSERT INTO mock_financial_items(id,company_id,creator_id,kind,party_name,description,amount_cents,due_on,category_id,cost_center_id)
         SELECT $1,$2,$3,$4::mock_financial_kind,$5,$6,$7,$8,$9,$10
         WHERE ($9::uuid IS NULL OR EXISTS(SELECT 1 FROM mock_financial_categories WHERE company_id=$2 AND id=$9))
         AND ($10::uuid IS NULL OR EXISTS(SELECT 1 FROM mock_cost_centers WHERE company_id=$2 AND id=$10))
         RETURNING id,kind::text,party_name,description,amount_cents,due_on,state::text,created_at,settled_at,
         (SELECT name FROM mock_financial_categories WHERE company_id=$2 AND id=category_id),
         (SELECT name FROM mock_cost_centers WHERE company_id=$2 AND id=cost_center_id)",
    ).bind(Uuid::new_v4()).bind(identity.company_id).bind(identity.id).bind(input.tipo).bind(input.parte.trim()).bind(input.descricao.trim()).bind(cents).bind(input.vencimento).bind(input.categoria_id).bind(input.centro_custo_id).fetch_optional(&mut *tx).await?.ok_or(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    audit(
        &mut tx,
        &identity,
        row.0,
        "Lançamento financeiro MOCK criado",
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(item(row))))
}

pub async fn list_recurring(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Vec<RecurringRule>>> {
    let identity = user(&app, &headers, false).await?;
    let rules: Vec<RecurringRuleRow> = query_as(
            "SELECT r.id,r.kind::text,r.party_name,r.description,r.amount_cents,r.due_day,r.active,c.name,cc.name
             FROM mock_financial_recurring_rules r
             LEFT JOIN mock_financial_categories c ON (c.company_id,c.id)=(r.company_id,r.category_id)
             LEFT JOIN mock_cost_centers cc ON (cc.company_id,cc.id)=(r.company_id,r.cost_center_id)
             WHERE r.company_id=$1 ORDER BY r.active DESC,r.created_at DESC,r.id LIMIT 200",
        )
        .bind(identity.company_id)
        .fetch_all(&app.db)
        .await?;
    Ok(Json(
        rules
            .into_iter()
            .map(|r| RecurringRule {
                id: r.0,
                tipo: r.1,
                parte: r.2,
                descricao: r.3,
                valor_centavos: r.4.to_string(),
                dia_vencimento: r.5,
                ativo: r.6,
                categoria: r.7,
                centro_custo: r.8,
            })
            .collect(),
    ))
}

pub async fn create_recurring(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<RecurringRuleCreate>, JsonRejection>,
) -> Result<(StatusCode, Json<RecurringRule>)> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|error| Error::Invalid(error.status()))?;
    let cents = input
        .valor_centavos
        .parse::<i64>()
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    if !(1..=999_999_999_999).contains(&cents)
        || input.parte.trim().is_empty()
        || input.parte.len() > 160
        || input.descricao.trim().is_empty()
        || input.descricao.len() > 240
        || !matches!(input.tipo.as_str(), "PAGAR" | "RECEBER")
        || !(1..=31).contains(&input.dia_vencimento)
    {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let id = Uuid::new_v4();
    let mut tx = app.db.begin().await?;
    let row: Option<RecurringRuleRow> = query_as(
            "INSERT INTO mock_financial_recurring_rules(id,company_id,creator_id,kind,party_name,description,amount_cents,due_day,category_id,cost_center_id)
             SELECT $1,$2,$3,$4::mock_financial_kind,$5,$6,$7,$8,$9,$10
             WHERE ($9::uuid IS NULL OR EXISTS(SELECT 1 FROM mock_financial_categories WHERE company_id=$2 AND id=$9))
               AND ($10::uuid IS NULL OR EXISTS(SELECT 1 FROM mock_cost_centers WHERE company_id=$2 AND id=$10))
             RETURNING id,kind::text,party_name,description,amount_cents,due_day,active,
             (SELECT name FROM mock_financial_categories WHERE company_id=$2 AND id=category_id),
             (SELECT name FROM mock_cost_centers WHERE company_id=$2 AND id=cost_center_id)",
        )
        .bind(id)
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(&input.tipo)
        .bind(input.parte.trim())
        .bind(input.descricao.trim())
        .bind(cents)
        .bind(i16::from(input.dia_vencimento))
        .bind(input.categoria_id)
        .bind(input.centro_custo_id)
        .fetch_optional(&mut *tx)
        .await?;
    let row = row.ok_or(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    audit(
        &mut tx,
        &identity,
        id,
        "Regra recorrente financeira MOCK criada",
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(RecurringRule {
            id: row.0,
            tipo: row.1,
            parte: row.2,
            descricao: row.3,
            valor_centavos: row.4.to_string(),
            dia_vencimento: row.5,
            ativo: row.6,
            categoria: row.7,
            centro_custo: row.8,
        }),
    ))
}

pub async fn generate_recurring(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<RecurringGenerate>, JsonRejection>,
) -> Result<Json<RecurringGenerateResult>> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|error| Error::Invalid(error.status()))?;
    let month =
        Month::try_from(input.mes).map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    let period = Date::from_calendar_date(input.ano, month, 1)
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    let mut tx = app.db.begin().await?;
    query("SELECT pg_advisory_xact_lock(hashtextextended($1::text || ':' || $2::text,0))")
        .bind(identity.company_id)
        .bind(period)
        .execute(&mut *tx)
        .await?;
    let rules: Vec<RecurringRuleInsertRow> = query_as(
            "SELECT id,kind::text,party_name,description,amount_cents,due_day,category_id,cost_center_id
             FROM mock_financial_recurring_rules WHERE company_id=$1 AND active ORDER BY id",
        )
        .bind(identity.company_id)
        .fetch_all(&mut *tx)
        .await?;
    let days_in_month: i16 = match month {
        Month::February if input.ano % 4 == 0 && (input.ano % 100 != 0 || input.ano % 400 == 0) => {
            29
        }
        Month::February => 28,
        Month::April | Month::June | Month::September | Month::November => 30,
        _ => 31,
    };
    let mut created = 0_i64;
    for rule in rules {
        let exists: bool = query_scalar(
            "SELECT EXISTS(SELECT 1 FROM mock_financial_recurring_occurrences WHERE company_id=$1 AND rule_id=$2 AND period=$3)",
        )
        .bind(identity.company_id)
        .bind(rule.0)
        .bind(period)
        .fetch_one(&mut *tx)
        .await?;
        if exists {
            continue;
        }
        let item_id = Uuid::new_v4();
        query(
            "INSERT INTO mock_financial_items(id,company_id,creator_id,kind,party_name,description,amount_cents,due_on,category_id,cost_center_id)
             VALUES($1,$2,$3,$4::mock_financial_kind,$5,$6,$7,$8 + ($9::integer - 1),$10,$11)",
        )
        .bind(item_id)
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(&rule.1)
        .bind(&rule.2)
        .bind(&rule.3)
        .bind(rule.4)
        .bind(period)
        .bind(rule.5.min(days_in_month))
        .bind(rule.6)
        .bind(rule.7)
        .execute(&mut *tx)
        .await?;
        query("INSERT INTO mock_financial_recurring_occurrences(company_id,rule_id,period,item_id) VALUES($1,$2,$3,$4)")
            .bind(identity.company_id)
            .bind(rule.0)
            .bind(period)
            .bind(item_id)
            .execute(&mut *tx)
            .await?;
        audit(
            &mut tx,
            &identity,
            item_id,
            "Lançamento recorrente MOCK gerado",
        )
        .await?;
        created += 1;
    }
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(format!(
            "Lançamentos recorrentes MOCK gerados para {}/{}: {created}",
            input.ano, input.mes
        ))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(RecurringGenerateResult { criados: created }))
}

pub async fn change_recurring(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<RecurringRuleChange>, JsonRejection>,
) -> Result<StatusCode> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|error| Error::Invalid(error.status()))?;
    let mut tx = app.db.begin().await?;
    let changed = query("UPDATE mock_financial_recurring_rules SET active=$3 WHERE company_id=$1 AND id=$2 AND active<>$3")
        .bind(identity.company_id)
        .bind(input.id)
        .bind(input.ativo)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        let exists: bool = query_scalar("SELECT EXISTS(SELECT 1 FROM mock_financial_recurring_rules WHERE company_id=$1 AND id=$2)")
            .bind(identity.company_id)
            .bind(input.id)
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Err(Error::Missing);
        }
    }
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(format!(
            "Regra recorrente MOCK {}: {}",
            if input.ativo { "ativada" } else { "pausada" },
            input.id
        ))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<Change>, JsonRejection>,
) -> Result<Json<Item>> {
    let identity = user(&app, &headers, true).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    if !matches!(input.acao.as_str(), "LIQUIDAR" | "CANCELAR") {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let mut tx = app.db.begin().await?;
    let row: Option<Row> = query_as(
        "UPDATE mock_financial_items SET state=$3::mock_financial_state,
         settled_at=CASE WHEN $3='LIQUIDADO' THEN now() ELSE settled_at END
         WHERE company_id=$1 AND id=$2 AND state='ABERTO'
         RETURNING id,kind::text,party_name,description,amount_cents,due_on,state::text,created_at,settled_at,
         (SELECT name FROM mock_financial_categories WHERE company_id=$1 AND id=category_id),
         (SELECT name FROM mock_cost_centers WHERE company_id=$1 AND id=cost_center_id)",
    ).bind(identity.company_id).bind(input.id).bind(if input.acao=="LIQUIDAR"{"LIQUIDADO"}else{"CANCELADO"}).fetch_optional(&mut *tx).await?;
    let row = row.ok_or(Error::Conflict)?;
    audit(
        &mut tx,
        &identity,
        row.0,
        if input.acao == "LIQUIDAR" {
            "Lançamento financeiro MOCK liquidado"
        } else {
            "Lançamento financeiro MOCK cancelado"
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(item(row)))
}

#[derive(Serialize)]
pub struct ReminderResult {
    pub criados: i64,
}

#[derive(Serialize)]
pub struct Reminder {
    pub item_id: Uuid,
    pub reminder_date: Date,
    pub party_name: String,
    pub description: String,
    pub due_on: Date,
    pub amount_cents: String,
}

pub async fn list_reminders(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Vec<Reminder>>> {
    let identity = user(&app, &headers, false).await?;
    let reminders: Vec<(Uuid, Date, String, String, Date, i64)> = query_as(
        "SELECT r.item_id,r.reminder_date,i.party_name,i.description,i.due_on,i.amount_cents
         FROM mock_financial_reminders r JOIN mock_financial_items i
         ON (i.company_id,i.id)=(r.company_id,r.item_id)
         WHERE r.company_id=$1 ORDER BY r.reminder_date DESC,i.due_on,i.id LIMIT 200",
    )
    .bind(identity.company_id)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(
        reminders
            .into_iter()
            .map(|r| Reminder {
                item_id: r.0,
                reminder_date: r.1,
                party_name: r.2,
                description: r.3,
                due_on: r.4,
                amount_cents: r.5.to_string(),
            })
            .collect(),
    ))
}

pub async fn reminders(State(app): State<App>, headers: HeaderMap) -> Result<Json<ReminderResult>> {
    let identity = user(&app, &headers, true).await?;
    let mut tx = app.db.begin().await?;
    let created = query(
        "INSERT INTO mock_financial_reminders(company_id,item_id,reminder_date)
         SELECT company_id,id,current_date FROM mock_financial_items
         WHERE company_id=$1 AND state='ABERTO' AND due_on BETWEEN current_date AND current_date+7
         ON CONFLICT DO NOTHING",
    )
    .bind(identity.company_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
        .bind(identity.company_id)
        .bind(identity.id)
        .bind(format!("Lembretes MOCK de vencimento gerados: {created}"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(ReminderResult {
        criados: created as i64,
    }))
}

fn item(r: Row) -> Item {
    Item {
        id: r.0,
        tipo: r.1,
        parte: r.2,
        descricao: r.3,
        valor_centavos: r.4.to_string(),
        vencimento: r.5,
        situacao: r.6,
        criado_em: r.7,
        liquidado_em: r.8,
        categoria: r.9,
        centro_custo: r.10,
    }
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
