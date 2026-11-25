use crate::{
    domain::{Dashboard, Statement, StatementQuery},
    infrastructure::banking::inter::BankError,
};
use uuid::Uuid;
#[async_trait::async_trait]
pub trait BankProvider: Send + Sync {
    fn environment(&self) -> &'static str;
    async fn recurrence(
        &self,
        _company: Uuid,
        _id: &str,
    ) -> Result<crate::infrastructure::banking::inter::Recurrence, BankError> {
        Err(BankError::Mock)
    }
    async fn pix_charge(
        &self,
        company: Uuid,
        txid: crate::domain::pix::Txid,
    ) -> Result<crate::domain::pix::PixCharge, BankError>;
    fn sandbox_issuance(&self) -> bool {
        false
    }
    async fn issue_sandbox(
        &self,
        _company: Uuid,
        _actor: Uuid,
        _order: crate::recebimentos::SandboxOrder,
    ) -> Result<crate::recebimentos::OrderState, BankError> {
        Err(BankError::Mock)
    }
    fn collection_binding(&self) -> Option<(Uuid, Uuid)> {
        None
    }
    async fn confirm_receipt(
        &self,
        _company: Uuid,
        _family: &str,
        _reference: &str,
    ) -> Result<crate::recebimentos::Confirmation, BankError> {
        Err(BankError::Mock)
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
