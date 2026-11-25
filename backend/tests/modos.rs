use std::process::Command;

#[test]
fn rejeita_ambientes_e_escritas_antes_de_conectar_ao_banco() {
    for (name, value, message) in [
        (
            "INTER_ENV",
            "sandbox",
            "Leitura externa exige ENABLE_BANK_READS=true",
        ),
        (
            "INTER_ENV",
            "production",
            "Leitura externa exige ENABLE_BANK_READS=true",
        ),
        (
            "BANK_PROVIDER",
            "desconhecido",
            "Provedor bancário inválido",
        ),
        (
            "ENABLE_REAL_PAYMENTS",
            "true",
            "Escrita bancária indisponível",
        ),
        (
            "ENABLE_BANK_WRITES",
            "true",
            "Escrita bancária indisponível",
        ),
        ("APP_ENV", "prod", "Ambiente inválido"),
        ("APP_ENV", "production", "Produção exige origem HTTPS"),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_inter-pj-manager"))
            .env_clear()
            .env(name, value)
            .output()
            .unwrap();
        assert!(!run.status.success());
        assert!(String::from_utf8_lossy(&run.stderr).contains(message));
    }
}

#[test]
fn producao_rejeita_origens_invalidas_antes_de_acessar_banco() {
    for origin in [
        "https://",
        "https://usuario@exemplo.com",
        "https://exemplo.com/caminho",
        "https://exemplo.com?x=1",
        "https://exemplo.com/",
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_inter-pj-manager"))
            .env_clear()
            .env("APP_ENV", "production")
            .env("APP_ORIGIN", origin)
            .output()
            .unwrap();
        assert!(!run.status.success());
        assert!(
            String::from_utf8_lossy(&run.stderr).contains("Produção exige origem HTTPS válida")
        );
    }
    let valid = Command::new(env!("CARGO_BIN_EXE_inter-pj-manager"))
        .env_clear()
        .env("APP_ENV", "production")
        .env("APP_ORIGIN", "https://exemplo.com:8443")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&valid.stderr).contains("environment variable not found"));
}
