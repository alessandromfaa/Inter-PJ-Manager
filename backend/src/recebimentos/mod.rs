pub mod api;
pub mod webhook;
pub mod worker;

use crate::infrastructure::banking::inter::BankError;
use sqlx_core::{query::query, transaction::Transaction};
use sqlx_postgres::Postgres;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct Scope {
    pub company: Uuid,
    pub account: Option<Uuid>,
    pub mode: &'static str,
}

pub struct Confirmation {
    pub txid: Option<String>,
    pub status: String,
    pub gross: i64,
    pub received: i64,
    pub channel: Option<String>,
}

pub fn valid_e2e(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    scope: Scope,
    family: &str,
    reference: &str,
    key: &str,
) -> Result<(), BankError> {
    query("INSERT INTO collection_events(company_id,environment,account_id,family,reference,event_key)
           VALUES($1,$2::text::integration_environment,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(scope.company).bind(scope.mode).bind(scope.account).bind(family).bind(reference).bind(key)
        .execute(&mut **tx).await?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "family", deny_unknown_fields)]
pub enum SandboxOrder {
    #[serde(rename = "PIX")]
    Pix {
        intent_id: Uuid,
        original_cents: String,
    },
    #[serde(rename = "V3")]
    Billing {
        intent_id: Uuid,
        original_cents: String,
        customer_id: Uuid,
        due_on: time::Date,
    },
}
impl SandboxOrder {
    pub fn fields(&self) -> (Uuid, &str, &'static str) {
        match self {
            Self::Pix {
                intent_id,
                original_cents,
            } => (*intent_id, original_cents, "PIX"),
            Self::Billing {
                intent_id,
                original_cents,
                ..
            } => (*intent_id, original_cents, "V3"),
        }
    }
}
#[derive(serde::Serialize)]
pub struct OrderState {
    pub intent_id: Uuid,
    pub reference: Option<String>,
    pub state: String,
}
