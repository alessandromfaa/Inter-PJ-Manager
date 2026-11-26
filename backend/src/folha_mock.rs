use crate::{
    App,
    api::{Error, Result},
    auth::{self, Identity},
};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
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
pub struct SalaryChange {
    funcionario_id: Uuid,
    vigente_a_partir: Date,
    salario_centavos: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalaryInput {
    funcionario_id: Uuid,
    vigente_a_partir: Date,
    salario_centavos: String,
    idempotencia: Uuid,
}

pub async fn salary_changes(
    State(app): State<App>,
    headers: HeaderMap,
    Query(input): Query<EntriesQuery>,
) -> Result<Json<Vec<SalaryChange>>> {
    let user = admin(&app, &headers).await?;
    let rows: Vec<(Uuid, Date, i64)> = query_as("SELECT employee_id,effective_on,salary_cents FROM mock_payroll_salary_changes WHERE company_id=$1 AND employee_id=$2 ORDER BY effective_on LIMIT 1001")
        .bind(user.company_id).bind(input.funcionario_id).fetch_all(&app.db).await?;
    if rows.len() > 1000 {
        return Err(Error::Invalid(StatusCode::PAYLOAD_TOO_LARGE));
    }
    Ok(Json(
        rows.into_iter()
            .map(|(funcionario_id, vigente_a_partir, salario)| SalaryChange {
                funcionario_id,
                vigente_a_partir,
                salario_centavos: salario.to_string(),
            })
            .collect(),
    ))
}

pub async fn create_salary_change(
    State(app): State<App>,
    headers: HeaderMap,
    body: std::result::Result<Json<SalaryInput>, JsonRejection>,
) -> Result<(StatusCode, Json<SalaryChange>)> {
    let user = admin(&app, &headers).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let salary = input
        .salario_centavos
        .parse::<i64>()
        .map_err(|_| Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY))?;
    if !(1..=999_999_999_999).contains(&salary) || ![5, 20].contains(&input.vigente_a_partir.day())
    {
        return Err(Error::Invalid(StatusCode::UNPROCESSABLE_ENTITY));
    }
    let mut tx = app.db.begin().await?;
    let employee: Option<(Uuid,)> =
        query_as("SELECT id FROM mock_payroll_employees WHERE company_id=$1 AND id=$2 FOR UPDATE")
            .bind(user.company_id)
            .bind(input.funcionario_id)
            .fetch_optional(&mut *tx)
            .await?;
    if employee.is_none() {
        return Err(Error::Missing);
    }
    let existing: Option<(Uuid, Date, i64)> = query_as("SELECT employee_id,effective_on,salary_cents FROM mock_payroll_salary_changes WHERE company_id=$1 AND actor_id=$2 AND idempotency_key=$3")
        .bind(user.company_id).bind(user.id).bind(input.idempotencia).fetch_optional(&mut *tx).await?;
    if let Some((employee, date, amount)) = existing {
        if employee != input.funcionario_id || date != input.vigente_a_partir || amount != salary {
            return Err(Error::Conflict);
        }
    } else {
        if input.vigente_a_partir <= time::OffsetDateTime::now_utc().date() {
            return Err(Error::Conflict);
        }
        let created = query("INSERT INTO mock_payroll_salary_changes(id,company_id,employee_id,actor_id,idempotency_key,effective_on,salary_cents) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4()).bind(user.company_id).bind(input.funcionario_id).bind(user.id).bind(input.idempotencia).bind(input.vigente_a_partir).bind(salary).execute(&mut *tx).await?;
        if created.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Salário fictício alterado')")
            .bind(user.company_id).bind(user.id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(SalaryChange {
            funcionario_id: input.funcionario_id,
            vigente_a_partir: input.vigente_a_partir,
            salario_centavos: salary.to_string(),
        }),
    ))
}
#[derive(Serialize)]
pub struct Entry {
    id: Uuid,
    funcionario_id: Uuid,
    tipo: String,
    data: Date,
    valor_centavos: Option<String>,
    cancelado: bool,
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
    let rows: Vec<(Uuid, Uuid, String, Date, Option<i64>, bool)> = query_as("SELECT id,employee_id,kind,occurred_on,amount_cents,cancelled_at IS NOT NULL FROM mock_payroll_entries WHERE company_id=$1 AND employee_id=$2 ORDER BY occurred_on DESC,id LIMIT 1001")
        .bind(user.company_id).bind(input.funcionario_id).fetch_all(&app.db).await?;
    if rows.len() > 1000 {
        return Err(Error::Invalid(StatusCode::PAYLOAD_TOO_LARGE));
    }
    Ok(Json(
        rows.into_iter()
            .map(|(id, funcionario_id, tipo, data, valor, cancelado)| Entry {
                id,
                funcionario_id,
                tipo,
                data,
                valor_centavos: valor.map(|v| v.to_string()),
                cancelado,
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
    let inserted: Option<(Uuid,)> = query_as("INSERT INTO mock_payroll_entries(id,company_id,employee_id,creator_id,idempotency_key,kind,occurred_on,amount_cents) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING RETURNING id")
        .bind(Uuid::new_v4()).bind(user.company_id).bind(input.funcionario_id).bind(user.id).bind(input.idempotencia).bind(&input.tipo).bind(input.data).bind(value).fetch_optional(&mut *tx).await?;
    let id = if let Some((id,)) = inserted {
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Lançamento fictício de folha criado')").bind(user.company_id).bind(user.id).execute(&mut *tx).await?;
        id
    } else {
        let existing: Option<(Uuid, Uuid, String, Date, Option<i64>, bool)> = query_as("SELECT id,employee_id,kind,occurred_on,amount_cents,cancelled_at IS NOT NULL FROM mock_payroll_entries WHERE company_id=$1 AND creator_id=$2 AND idempotency_key=$3")
            .bind(user.company_id).bind(user.id).bind(input.idempotencia).fetch_optional(&mut *tx).await?;
        let Some((id, employee, kind, date, amount, cancelled)) = existing else {
            return Err(Error::Conflict);
        };
        if employee != input.funcionario_id
            || kind != input.tipo
            || date != input.data
            || amount != value
            || cancelled
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
            cancelado: false,
        }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationInput {
    idempotencia: Uuid,
}

pub async fn cancel_entry(
    State(app): State<App>,
    headers: HeaderMap,
    Path(entry_id): Path<Uuid>,
    body: std::result::Result<Json<CancellationInput>, JsonRejection>,
) -> Result<StatusCode> {
    let user = admin(&app, &headers).await?;
    let Json(input) = body.map_err(|e| Error::Invalid(e.status()))?;
    let mut tx = app.db.begin().await?;
    let exists: Option<(Uuid,)> =
        query_as("SELECT id FROM mock_payroll_entries WHERE company_id=$1 AND id=$2")
            .bind(user.company_id)
            .bind(entry_id)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(Error::Missing);
    }
    let inserted: Option<(Uuid,)> = query_as("INSERT INTO mock_payroll_entry_cancellations(company_id,actor_id,idempotency_key,entry_id) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING RETURNING entry_id")
        .bind(user.company_id).bind(user.id).bind(input.idempotencia).bind(entry_id).fetch_optional(&mut *tx).await?;
    if inserted.is_none() {
        let (existing,): (Uuid,) = query_as("SELECT entry_id FROM mock_payroll_entry_cancellations WHERE company_id=$1 AND actor_id=$2 AND idempotency_key=$3")
            .bind(user.company_id).bind(user.id).bind(input.idempotencia).fetch_one(&mut *tx).await?;
        if existing != entry_id {
            return Err(Error::Conflict);
        }
    } else {
        let changed = query("UPDATE mock_payroll_entries SET cancelled_at=now(),cancelled_by=$1 WHERE company_id=$2 AND id=$3 AND cancelled_at IS NULL")
            .bind(user.id).bind(user.company_id).bind(entry_id).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Lançamento fictício de folha cancelado')")
            .bind(user.company_id).bind(user.id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
