use super::{BankError, contract::cents};
use crate::recebimentos::Confirmation;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(untagged)]
enum Money {
    Text(String),
    Number(serde_json::Number),
}
impl Money {
    fn read(self) -> Result<i64, BankError> {
        let n = match self {
            Self::Text(s) => cents(&s),
            Self::Number(n) => cents(&n.to_string()),
        }?;
        if n < 0 {
            return Err(BankError::Contract);
        }
        Ok(n)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BillingResponse {
    cobranca: Bill,
}
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum BillStatus {
    Recebido,
    AReceber,
    MarcadoRecebido,
    Atrasado,
    Cancelado,
    Expirado,
    FalhaEmissao,
    EmProcessamento,
    Protesto,
}
impl BillStatus {
    fn label(&self) -> &'static str {
        match self {
            Self::Recebido => "RECEBIDO",
            Self::AReceber => "A_RECEBER",
            Self::MarcadoRecebido => "MARCADO_RECEBIDO",
            Self::Atrasado => "ATRASADO",
            Self::Cancelado => "CANCELADO",
            Self::Expirado => "EXPIRADO",
            Self::FalhaEmissao => "FALHA_EMISSAO",
            Self::EmProcessamento => "EM_PROCESSAMENTO",
            Self::Protesto => "PROTESTO",
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bill {
    codigo_solicitacao: Uuid,
    situacao: BillStatus,
    valor_nominal: Money,
    valor_total_recebido: Option<Money>,
    origem_recebimento: Option<String>,
}
impl BillingResponse {
    pub fn confirm(self, code: Uuid) -> Result<Confirmation, BankError> {
        let b = self.cobranca;
        if b.codigo_solicitacao != code {
            return Err(BankError::Contract);
        }
        let (received, channel) = if matches!(b.situacao, BillStatus::Recebido) {
            let amount = b.valor_total_recebido.ok_or(BankError::Contract)?.read()?;
            if amount == 0 || !matches!(b.origem_recebimento.as_deref(), Some("PIX" | "BOLETO")) {
                return Err(BankError::Contract);
            }
            (amount, b.origem_recebimento)
        } else {
            (0, None)
        };
        Ok(Confirmation {
            status: b.situacao.label().into(),
            txid: None,
            gross: b.valor_nominal.read()?,
            received,
            channel,
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PixReceipt {
    end_to_end_id: String,
    txid: Option<String>,
    valor: String,
    #[serde(with = "time::serde::rfc3339", rename = "horario")]
    _at: time::OffsetDateTime,
    #[serde(default)]
    devolucoes: Vec<Refund>,
}
#[derive(Deserialize)]
struct Refund {
    id: String,
    status: RefundStatus,
    valor: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum RefundStatus {
    EmProcessamento,
    Devolvido,
    NaoRealizado,
}
impl PixReceipt {
    pub fn confirm(self, id: &str) -> Result<Confirmation, BankError> {
        if self.end_to_end_id != id
            || self.txid.as_ref().is_some_and(|t| {
                t.len() > 35 || t.is_empty() || !t.bytes().all(|b| b.is_ascii_alphanumeric())
            })
        {
            return Err(BankError::Contract);
        }
        let gross = cents(&self.valor)?;
        if gross <= 0 {
            return Err(BankError::Contract);
        }
        let mut received = gross;
        let mut ids = std::collections::HashSet::new();
        for refund in self.devolucoes {
            let amount = cents(&refund.valor)?;
            if amount <= 0 || refund.id.is_empty() || !ids.insert(refund.id) {
                return Err(BankError::Contract);
            }
            if matches!(refund.status, RefundStatus::Devolvido) {
                received = received
                    .checked_sub(amount)
                    .filter(|x| *x >= 0)
                    .ok_or(BankError::Contract)?;
            }
        }
        Ok(Confirmation {
            status: if received == gross {
                "RECEBIDO"
            } else if received == 0 {
                "DEVOLVIDO"
            } else {
                "DEVOLVIDO_PARCIAL"
            }
            .into(),
            gross,
            received,
            txid: self.txid,
            channel: Some("PIX".into()),
        })
    }
}
