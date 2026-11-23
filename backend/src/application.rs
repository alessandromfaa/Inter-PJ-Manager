use crate::domain::Snapshot;
use uuid::Uuid;

/// Leitura do painel dentro da empresa autenticada. Operações de escrita
/// terão contratos próprios, com os estados assíncronos de cada família.
#[async_trait::async_trait]
pub trait BankProvider: Send + Sync {
    async fn snapshot(&self, company_id: Uuid) -> anyhow::Result<Snapshot>;
}
