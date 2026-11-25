use super::BankError;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use time::Date;

pub(super) fn cents(raw: &str) -> Result<i64, BankError> {
    // from_str_exact impede arredondamento na leitura do JSON.
    let value = Decimal::from_str_exact(raw).map_err(|_| BankError::Contract)?;
    let scaled = value
        .checked_mul(Decimal::from(100))
        .ok_or(BankError::Contract)?;
    if !scaled.fract().is_zero() {
        return Err(BankError::Contract);
    }
    scaled.to_i64().ok_or(BankError::Contract)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    disponivel: Number,
    bloqueado_cheque: Option<Number>,
    bloqueado_judicialmente: Option<Number>,
    bloqueado_administrativo: Option<Number>,
    limite: Option<Number>,
    data_referencia: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Balance {
    // Strings inteiras mantêm todos os centavos no transporte ao navegador.
    pub available: String,
    pub blocked_check: Option<String>,
    pub blocked_court: Option<String>,
    pub blocked_admin: Option<String>,
    pub credit_limit: Option<String>,
    pub reference_date: Option<Date>,
}
impl BalanceResponse {
    pub fn convert(self) -> Result<Balance, BankError> {
        let optional = |v: Option<Number>| {
            v.map(|n| cents(&n.to_string()).map(|c| c.to_string()))
                .transpose()
        };
        Ok(Balance {
            available: cents(&self.disponivel.to_string())?.to_string(),
            blocked_check: optional(self.bloqueado_cheque)?,
            blocked_court: optional(self.bloqueado_judicialmente)?,
            blocked_admin: optional(self.bloqueado_administrativo)?,
            credit_limit: optional(self.limite)?,
            reference_date: self
                .data_referencia
                .map(|v| {
                    Date::parse(
                        &v,
                        time::macros::format_description!("[day]/[month]/[year]"),
                    )
                    .or_else(|_| {
                        Date::parse(
                            &v,
                            time::macros::format_description!("[year]-[month]-[day]"),
                        )
                    })
                    .map_err(|_| BankError::Contract)
                })
                .transpose()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub total_paginas: Option<usize>,
    pub total_elementos: usize,
    pub ultima_pagina: bool,
    pub numero_de_elementos: usize,
    pub transacoes: Vec<Transaction>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id_transacao: String,
    pub data_transacao: Date,
    pub data_inclusao: Option<Date>,
    pub tipo_transacao: String,
    pub titulo: Option<String>,
    pub descricao: Option<String>,
    pub numero_documento: Option<String>,
    tipo_operacao: Direction,
    valor: String,
}
#[derive(Deserialize)]
enum Direction {
    C,
    D,
}
impl Transaction {
    pub fn operation(&self) -> &'static str {
        match self.tipo_operacao {
            Direction::C => "C",
            Direction::D => "D",
        }
    }

    pub fn signed_cents(&self) -> Result<i64, BankError> {
        let amount = cents(&self.valor)?;
        match self.tipo_operacao {
            Direction::C if amount >= 0 => Ok(amount),
            Direction::D => amount
                .checked_abs()
                .and_then(i64::checked_neg)
                .ok_or(BankError::Contract),
            _ => Err(BankError::Contract),
        }
    }
}

pub(crate) mod pagamento_pix_contract {
    use serde::{Deserialize, Serialize};
    use serde_json::Number;
    use std::str::FromStr;
    use uuid::Uuid;

    #[derive(Serialize)]
    struct Destinatario<'a> {
        tipo: &'static str,
        chave: &'a str,
    }

    #[derive(Serialize)]
    pub(crate) struct Requisicao<'a> {
        valor: Number,
        destinatario: Destinatario<'a>,
    }

    pub(crate) fn pagamento_por_chave(centavos: i64, chave: &str) -> Option<Requisicao<'_>> {
        if centavos <= 0
            || chave.is_empty()
            || chave.len() > 77
            || chave.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return None;
        }
        let valor = Number::from_str(&format!("{}.{:02}", centavos / 100, centavos % 100)).ok()?;
        Some(Requisicao {
            valor,
            destinatario: Destinatario {
                tipo: "CHAVE",
                chave,
            },
        })
    }

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub(crate) enum TipoRetorno {
        Aprovacao,
        Processado,
        Agendado,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct Resposta {
        pub(crate) tipo_retorno: TipoRetorno,
        pub(crate) codigo_solicitacao: Uuid,
    }

    #[cfg(test)]
    #[test]
    fn valor_e_serializado_sem_ponto_flutuante() {
        let pagamento = pagamento_por_chave(9_007_199_254_740_993, "chave-ficticia").unwrap();
        let json = serde_json::to_string(&pagamento).unwrap();
        assert!(json.contains("\"valor\":90071992547409.93"));
        assert!(pagamento_por_chave(0, "chave").is_none());
        assert!(pagamento_por_chave(1, " ").is_none());
    }

    #[cfg(test)]
    #[test]
    fn resultado_de_inclusao_preserva_estados_distintos() {
        for (valor, esperado) in [
            ("APROVACAO", TipoRetorno::Aprovacao),
            ("PROCESSADO", TipoRetorno::Processado),
            ("AGENDADO", TipoRetorno::Agendado),
        ] {
            let json = format!(
                r#"{{"tipoRetorno":"{valor}","codigoSolicitacao":"c42f0787-02cb-4b31-827e-459ec9d7ece1"}}"#
            );
            let resposta: Resposta = serde_json::from_str(&json).unwrap();
            assert_eq!(resposta.tipo_retorno, esperado);
            assert!(!resposta.codigo_solicitacao.is_nil());
        }
        assert!(serde_json::from_str::<Resposta>(r#"{"tipoRetorno":"PAGO","codigoSolicitacao":"c42f0787-02cb-4b31-827e-459ec9d7ece1"}"#).is_err());
    }
}

#[derive(Deserialize)]
pub(super) struct PixResponse {
    pub txid: crate::domain::pix::Txid,
    pub status: crate::domain::pix::PixStatus,
    pub revisao: u32,
    valor: PixValue,
}

#[derive(Deserialize)]
struct PixValue {
    original: String,
}

impl PixResponse {
    pub fn into_charge(
        self,
        expected: &crate::domain::pix::Txid,
        source: &'static str,
    ) -> Result<crate::domain::pix::PixCharge, BankError> {
        let original = &self.valor.original;
        let (whole, fraction) = original.split_once('.').ok_or(BankError::Contract)?;
        if self.txid != *expected
            || !(1..=10).contains(&whole.len())
            || fraction.len() != 2
            || !whole
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return Err(BankError::Contract);
        }
        Ok(crate::domain::pix::PixCharge {
            source,
            txid: self.txid,
            status: self.status,
            revision: self.revisao,
            original_cents: cents(original)?.to_string(),
            mock_code: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precisao_monetaria() {
        assert_eq!(cents("90071992547409.93").unwrap(), 9007199254740993);
        assert_eq!(cents("-0.01").unwrap(), -1);
        assert!(cents("0.001").is_err());
        let balance: BalanceResponse =
            serde_json::from_str(r#"{"disponivel":90071992547409.93}"#).unwrap();
        let balance = balance.convert().unwrap();
        assert_eq!(balance.available, "9007199254740993");
        assert!(balance.credit_limit.is_none());
    }
}
