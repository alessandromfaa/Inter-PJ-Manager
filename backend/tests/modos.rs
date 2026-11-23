use std::process::Command;

#[test]
fn rejeita_ambientes_e_escritas_antes_de_conectar_ao_banco() {
    for (name, value, message) in [
        ("INTER_ENV", "sandbox", "somente Inter em MOCK"),
        ("INTER_ENV", "production", "somente Inter em MOCK"),
        ("BANK_PROVIDER", "desconhecido", "somente Inter em MOCK"),
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
