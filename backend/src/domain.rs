pub mod pix;
use serde::Serialize;
#[derive(Serialize)]
pub struct Entry {
    pub date: String,
    pub description: String,
    pub category: String,
    pub cents: i64,
}
#[derive(Serialize)]
pub struct PeriodSummary {
    pub settled_charge_cents: i64,
    pub pending_charge_cents: i64,
    pub scheduled_payment_cents: i64,
    pub unreconciled_count: u32,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub source: &'static str,
    pub as_of: String,
    pub opening: i64,
    pub available: i64,
    pub incoming: i64,
    pub outgoing: i64,
    pub entries: Vec<Entry>,
    pub summary: PeriodSummary,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Dashboard {
    Mock(Snapshot),
    Inter(LiveDashboard),
}

#[derive(Serialize)]
pub struct LiveDashboard {
    pub pix_lookup_enabled: bool,
    pub source: &'static str,
    #[serde(with = "time::serde::rfc3339")]
    pub certificate_expires_at: time::OffsetDateTime,
    pub reading: Option<Reading>,
    pub last_error: Option<String>,
}
#[derive(Serialize)]
pub struct Reading {
    pub balance: crate::infrastructure::banking::inter::Balance,
    #[serde(with = "time::serde::rfc3339")]
    pub synchronized_at: time::OffsetDateTime,
    pub imported_from: time::Date,
    pub imported_until: time::Date,
}
#[derive(serde::Deserialize, Default)]
pub struct StatementQuery {
    pub inicio: Option<time::Date>,
    pub fim: Option<time::Date>,
    pub pagina: Option<u32>,
}
#[derive(Serialize)]
pub struct Statement {
    pub entries: Vec<Posted>,
    pub has_more: bool,
}
#[derive(Serialize)]
pub struct Posted {
    pub date: time::Date,
    pub kind: String,
    pub cents: String,
}
