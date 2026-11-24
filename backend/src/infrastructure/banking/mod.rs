pub mod inter;
pub mod mock_pix;
use crate::domain::Dashboard;
use crate::{
    application::BankProvider,
    domain::{Entry, PeriodSummary, Snapshot},
};
use uuid::Uuid;
pub struct MockBank(pub sqlx_postgres::PgPool);
#[async_trait::async_trait]
impl BankProvider for MockBank {
    fn environment(&self) -> &'static str {
        "MOCK"
    }
    async fn pix_charge(
        &self,
        company: Uuid,
        txid: crate::domain::pix::Txid,
    ) -> Result<crate::domain::pix::PixCharge, inter::BankError> {
        use crate::domain::pix::{PixCharge, PixStatus};
        let (status, amount) = match txid.as_str() {
            "DEMO000000000000000000000001" => (PixStatus::Ativa, "212000"),
            "DEMO000000000000000000000002" => (PixStatus::Concluida, "425000"),
            _ => return mock_pix::find(&self.0, company, txid).await,
        };
        Ok(PixCharge {
            source: "MOCK",
            txid,
            status,
            revision: 0,
            original_cents: amount.into(),
            mock_code: None,
        })
    }

    async fn snapshot(&self, _company_id: Uuid) -> Result<Dashboard, inter::BankError> {
        let entries = [
            ("2026-11-23", "Estúdio Aurora", "Serviços", 425000),
            ("2026-11-20", "Locação do escritório", "Estrutura", -185000),
            ("2026-11-18", "Projeto Horizonte", "Consultoria", 790000),
            ("2026-11-13", "Plataforma de gestão", "Tecnologia", -24900),
            ("2026-11-10", "Oficina Norte", "Serviços", 318000),
            ("2026-11-04", "Fornecedor Atlântico", "Materiais", -136200),
        ]
        .into_iter()
        .map(|(date, description, category, cents)| Entry {
            date: date.into(),
            description: description.into(),
            category: category.into(),
            cents,
        })
        .collect::<Vec<_>>();
        let incoming = entries.iter().map(|e| e.cents.max(0)).sum::<i64>();
        let outgoing = entries.iter().map(|e| -e.cents.min(0)).sum::<i64>();
        let opening = 3475000;
        Ok(Dashboard::Mock(Snapshot {
            source: "MOCK",
            as_of: "2026-11-23".into(),
            opening,
            available: opening + incoming - outgoing,
            incoming,
            outgoing,
            // A cobrança liquidada corresponde ao crédito de Estúdio Aurora.
            // Valores pendentes e agendados ficam fora do saldo disponível.
            summary: PeriodSummary {
                settled_charge_cents: 425000,
                pending_charge_cents: 212000,
                scheduled_payment_cents: 87000,
                unreconciled_count: 1,
            },
            entries,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saldo_fecha_em_centavos() {
        let pool = sqlx_postgres::PgPoolOptions::new()
            .connect_lazy_with(sqlx_postgres::PgConnectOptions::new());
        let Dashboard::Mock(s) = MockBank(pool).snapshot(Uuid::new_v4()).await.unwrap() else {
            panic!("Esperado MOCK")
        };
        assert_eq!(
            s.available,
            s.opening + s.entries.iter().map(|e| e.cents).sum::<i64>()
        );
        assert_eq!(s.source, "MOCK");
    }
}
