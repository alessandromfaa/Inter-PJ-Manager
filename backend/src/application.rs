use crate::{
    domain::{Dashboard, Statement, StatementQuery},
    infrastructure::banking::inter::BankError,
};
use uuid::Uuid;
#[async_trait::async_trait]
pub trait BankProvider: Send + Sync {
    fn environment(&self) -> &'static str {
        "MOCK"
    }
    async fn snapshot(&self, company_id: Uuid) -> Result<Dashboard, BankError>;
    async fn synchronize(&self, _company: Uuid, _actor: Uuid) -> Result<(), BankError> {
        Err(BankError::Mock)
    }
    async fn statement(
        &self,
        _company: Uuid,
        _filter: StatementQuery,
    ) -> Result<Statement, BankError> {
        Err(BankError::Mock)
    }
}
