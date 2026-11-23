use anyhow::{Result, ensure};
use std::env;
pub fn var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.into())
}
pub fn validate() -> Result<bool> {
    ensure!(
        var("BANK_PROVIDER", "inter") == "inter" && var("INTER_ENV", "mock") == "mock",
        "Esta versão suporta somente Inter em MOCK"
    );
    for name in ["ENABLE_BANK_WRITES", "ENABLE_REAL_PAYMENTS"] {
        ensure!(
            var(name, "false") == "false",
            "Escrita bancária indisponível"
        );
    }
    match var("APP_ENV", "development").as_str() {
        "development" => Ok(false),
        "production" => Ok(true),
        _ => anyhow::bail!("Ambiente inválido"),
    }
}
