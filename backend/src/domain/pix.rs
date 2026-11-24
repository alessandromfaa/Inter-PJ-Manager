use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub struct Txid(String);

impl TryFrom<String> for Txid {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if (26..=35).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_alphanumeric()) {
            Ok(Self(value))
        } else {
            Err("O txid deve ter de 26 a 35 letras ou números.")
        }
    }
}
impl Txid {
    pub fn for_mock_intent(intent: uuid::Uuid) -> Self {
        Self(format!("M{}", intent.simple()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PixStatus {
    Ativa,
    Concluida,
    RemovidaPeloUsuarioRecebedor,
    RemovidaPeloPsp,
}
#[derive(Serialize)]
pub struct PixCharge {
    pub source: &'static str,
    pub txid: Txid,
    pub status: PixStatus,
    pub revision: u32,
    pub original_cents: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_code: Option<MockPaymentCode>,
}

#[derive(Serialize)]
pub struct MockPaymentCode {
    pub copy_paste: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: time::OffsetDateTime,
}
