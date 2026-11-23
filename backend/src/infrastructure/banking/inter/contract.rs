use super::BankError;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use time::Date;

fn cents(raw: &str) -> Result<i64, BankError> {
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
    pub tipo_transacao: String,
    tipo_operacao: Direction,
    valor: String,
}
#[derive(Deserialize)]
enum Direction {
    C,
    D,
}
impl Transaction {
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
