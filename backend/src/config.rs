use anyhow::{Result, ensure};
use std::env;
pub fn var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.into())
}
pub fn validate() -> Result<bool> {
    ensure!(
        var("BANK_PROVIDER", "inter") == "inter",
        "Provedor bancário inválido"
    );
    let mode = var("INTER_ENV", "mock");
    ensure!(
        matches!(mode.as_str(), "mock" | "sandbox" | "production"),
        "Ambiente Inter inválido"
    );
    if mode != "mock" {
        ensure!(
            var("ENABLE_BANK_READS", "false") == "true",
            "Leitura externa exige ENABLE_BANK_READS=true"
        );
        ensure!(
            mode != "production" || var("APP_ENV", "development") == "production",
            "Leitura de produção exige APP_ENV=production"
        );
    }
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
