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
