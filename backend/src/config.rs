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
    let sandbox_writes = var("ENABLE_SANDBOX_COLLECTIONS", "false");
    ensure!(
        matches!(sandbox_writes.as_str(), "true" | "false"),
        "Opção de homologação inválida"
    );
    if sandbox_writes == "true" {
        ensure!(
            mode == "sandbox" && var("ENABLE_BANK_WRITES", "false") == "true",
            "Emissão de homologação exige sandbox e escrita explícita"
        );
    } else {
        ensure!(
            var("ENABLE_BANK_WRITES", "false") == "false",
            "Escrita bancária indisponível"
        );
    }
    ensure!(
        var("ENABLE_REAL_PAYMENTS", "false") == "false",
        "Escrita bancária indisponível"
    );
    match var("APP_ENV", "development").as_str() {
        "development" => Ok(false),
        "production" => Ok(true),
        _ => anyhow::bail!("Ambiente inválido"),
    }
}
