use crate::{
    App,
    api::{Error, Result},
    auth::{self, Identity},
};
use axum::{
    Json,
    extract::{Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx_core::{query::query, query_as::query_as};
use time::Date;
use uuid::Uuid;

async fn admin(app: &App, headers: &HeaderMap) -> Result<Identity> {
    let user = auth::identity(app, headers).await?;
    if !user.roles.iter().any(|role| role == "ADMIN") {
        return Err(Error::Forbidden);
    }
    Ok(user)
}

#[derive(Serialize)]
pub struct Employee {
    id: Uuid,
    nome: String,
    salario_centavos: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeInput {
    nome: String,
    salario_centavos: String,
    idempotencia: Uuid,
}

pub async fn employees(State(app): State<App>, headers: HeaderMap) -> Result<Json<Vec<Employee>>> {
    let user = admin(&app, &headers).await?;
    let rows: Vec<(Uuid, String, i64)> = query_as("SELECT id,name,salary_cents FROM mock_payroll_employees WHERE company_id=$1 ORDER BY name,id LIMIT 201")
        .bind(user.company_id).fetch_all(&app.db).await?;
    if rows.len() > 200 {
        return Err(Error::Invalid(StatusCode::PAYLOAD_TOO_LARGE));
    }
    Ok(Json(
        rows.into_iter()
            .map(|(id, nome, salario)| Employee {
                id,
                nome,
                salario_centavos: salario.to_string(),
            })
            .collect(),
    ))
}

pub async fn create_employee(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<EmployeeInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Employee>)> {
    let user = admin(&app, &headers).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let name = input.nome.trim();
    let salary = input
        .salario_centavos
        .parse::<i64>()
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    if name.is_empty() || name.len() > 120 || !(1..=999_999_999_999).contains(&salary) {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let mut tx = app.db.begin().await?;
    let row: Option<(Uuid, String, i64)> = query_as("INSERT INTO mock_payroll_employees(id,company_id,creator_id,idempotency_key,name,salary_cents) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(company_id,creator_id,idempotency_key) DO NOTHING RETURNING id,name,salary_cents")
        .bind(Uuid::new_v4()).bind(user.company_id).bind(user.id).bind(input.idempotencia).bind(name).bind(salary).fetch_optional(&mut *tx).await?;
    let (id, nome, salario) = if let Some(row) = row {
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Funcionário fictício criado')").bind(user.company_id).bind(user.id).execute(&mut *tx).await?;
        row
    } else {
        let existing: (Uuid, String, i64) = query_as("SELECT id,name,salary_cents FROM mock_payroll_employees WHERE company_id=$1 AND creator_id=$2 AND idempotency_key=$3")
            .bind(user.company_id).bind(user.id).bind(input.idempotencia).fetch_one(&mut *tx).await?;
        if existing.1 != name || existing.2 != salary {
            return Err(Error::Conflict);
        }
        existing
    };
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Employee {
            id,
            nome,
            salario_centavos: salario.to_string(),
        }),
    ))
}

#[derive(Deserialize)]
pub struct EntriesQuery {
    funcionario_id: Uuid,
}
#[derive(Serialize)]
pub struct Entry {
    id: Uuid,
    funcionario_id: Uuid,
    tipo: String,
    data: Date,
    valor_centavos: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryInput {
    funcionario_id: Uuid,
    tipo: String,
    data: Date,
    valor_centavos: Option<String>,
    idempotencia: Uuid,
}

pub async fn entries(
    State(app): State<App>,
    headers: HeaderMap,
    Query(input): Query<EntriesQuery>,
) -> Result<Json<Vec<Entry>>> {
    let user = admin(&app, &headers).await?;
    let rows: Vec<(Uuid, Uuid, String, Date, Option<i64>)> = query_as("SELECT id,employee_id,kind,occurred_on,amount_cents FROM mock_payroll_entries WHERE company_id=$1 AND employee_id=$2 ORDER BY occurred_on DESC,id LIMIT 1001")
        .bind(user.company_id).bind(input.funcionario_id).fetch_all(&app.db).await?;
    if rows.len() > 1000 {
        return Err(Error::Invalid(StatusCode::PAYLOAD_TOO_LARGE));
    }
    Ok(Json(
        rows.into_iter()
            .map(|(id, funcionario_id, tipo, data, valor)| Entry {
                id,
                funcionario_id,
                tipo,
                data,
                valor_centavos: valor.map(|v| v.to_string()),
            })
            .collect(),
    ))
}

pub async fn create_entry(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<EntryInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Entry>)> {
    let user = admin(&app, &headers).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let value = input
        .valor_centavos
        .as_deref()
        .map(|v| v.parse::<i64>())
        .transpose()
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    let valid = match input.tipo.as_str() {
        "FALTA_INJUSTIFICADA" | "FALTA_JUSTIFICADA" => value.is_none(),
        "ADIANTAMENTO" | "PRODUCAO" => value.is_some_and(|v| (1..=999_999_999_999).contains(&v)),
        _ => false,
    };
    if !valid {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let mut tx = app.db.begin().await?;
    let exists: Option<(Uuid,)> =
        query_as("SELECT id FROM mock_payroll_employees WHERE company_id=$1 AND id=$2")
            .bind(user.company_id)
            .bind(input.funcionario_id)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(Error::Missing);
    }
    let inserted: Option<(Uuid,)> = query_as("INSERT INTO mock_payroll_entries(id,company_id,employee_id,creator_id,idempotency_key,kind,occurred_on,amount_cents) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(company_id,creator_id,idempotency_key) DO NOTHING RETURNING id")
        .bind(Uuid::new_v4()).bind(user.company_id).bind(input.funcionario_id).bind(user.id).bind(input.idempotencia).bind(&input.tipo).bind(input.data).bind(value).fetch_optional(&mut *tx).await?;
    let id = if let Some((id,)) = inserted {
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Lançamento fictício de folha criado')").bind(user.company_id).bind(user.id).execute(&mut *tx).await?;
        id
    } else {
        let (id, employee, kind, date, amount): (Uuid, Uuid, String, Date, Option<i64>) = query_as("SELECT id,employee_id,kind,occurred_on,amount_cents FROM mock_payroll_entries WHERE company_id=$1 AND creator_id=$2 AND idempotency_key=$3")
            .bind(user.company_id).bind(user.id).bind(input.idempotencia).fetch_one(&mut *tx).await?;
        if employee != input.funcionario_id
            || kind != input.tipo
            || date != input.data
            || amount != value
        {
            return Err(Error::Conflict);
        }
        id
    };
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Entry {
            id,
            funcionario_id: input.funcionario_id,
            tipo: input.tipo,
            data: input.data,
            valor_centavos: value.map(|v| v.to_string()),
        }),
    ))
}
