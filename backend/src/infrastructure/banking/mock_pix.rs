use super::inter::BankError;
use crate::domain::pix::{MockPaymentCode, PixCharge, PixStatus, Txid};
use crc::{CRC_16_IBM_3740, Crc};
use sqlx_core::{query::query, row::Row};
use sqlx_postgres::{PgPool, PgRow};
use uuid::Uuid;

// A URL é sintética e não contém uma chave Pix ou um destino de pagamento.
fn copy_paste(location: Uuid) -> String {
    let url = format!("pix.invalid/{}", location.simple());
    let merchant = format!("0014br.gov.bcb.pix25{:02}{url}", url.len());
    let payload = format!(
        "00020101021226{:02}{merchant}5204000053039865802BR5912EMPRESA MOCK6009FORTALEZA62070503***6304",
        merchant.len()
    );
    let crc = Crc::<u16>::new(&CRC_16_IBM_3740).checksum(payload.as_bytes());
    format!("{payload}{crc:04X}")
}

fn from_row(txid: Txid, row: &PgRow) -> PixCharge {
    PixCharge {
        source: "MOCK",
        txid,
        status: if row.get("settled") {
            PixStatus::Concluida
        } else {
            PixStatus::Ativa
        },
        revision: 0,
        original_cents: row.get::<i64, _>("original_cents").to_string(),
        mock_code: (!row.get::<bool, _>("settled")).then(|| MockPaymentCode {
            copy_paste: row.get("copy_paste"),
            expires_at: row.get("expires_at"),
        }),
    }
}

const SELECT_CHARGE: &str =
    "SELECT original_cents, copy_paste, created_at + interval '24 hours' AS expires_at
     , EXISTS(SELECT 1 FROM collection_results r WHERE r.environment='MOCK' AND r.family='PIX' AND r.company_id=c.company_id
        AND r.txid=c.txid AND r.reconciliation='CONCILIADO') AS settled
     FROM mock_pix_charges c WHERE company_id=$1 AND txid=$2";

pub async fn find(db: &PgPool, company: Uuid, txid: Txid) -> Result<PixCharge, BankError> {
    let row = query(SELECT_CHARGE)
        .bind(company)
        .bind(txid.as_str())
        .fetch_optional(db)
        .await?
        .ok_or(BankError::PixNotFound)?;
    Ok(from_row(txid, &row))
}

pub async fn issue(
    db: &PgPool,
    company: Uuid,
    actor: Uuid,
    intent: Uuid,
    cents: i64,
) -> Result<(PixCharge, bool), BankError> {
    let txid = Txid::for_mock_intent(intent);
    let mut tx = db.begin().await?;
    let inserted = query(
        "INSERT INTO mock_pix_charges(company_id,txid,original_cents,copy_paste,creator_id)
         VALUES($1,$2,$3,$4,$5) ON CONFLICT(company_id,txid) DO NOTHING",
    )
    .bind(company)
    .bind(txid.as_str())
    .bind(cents)
    .bind(copy_paste(Uuid::new_v4()))
    .bind(actor)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;

    // Em READ COMMITTED, esta segunda instrução enxerga também a inserção concorrente concluída.
    let row = query(SELECT_CHARGE)
        .bind(company)
        .bind(txid.as_str())
        .fetch_one(&mut *tx)
        .await?;
    if row.get::<i64, _>("original_cents") != cents {
        return Err(BankError::PixConflict);
    }
    if inserted {
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Cobrança Pix emitida em MOCK')")
            .bind(company).bind(actor).execute(&mut *tx).await?;
    }
    let charge = from_row(txid, &row);
    tx.commit().await?;
    Ok((charge, inserted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn br_code_dinamico_ficticio_com_crc_e_campos_conferidos() {
        assert_eq!(
            copy_paste(Uuid::nil()),
            "00020101021226660014br.gov.bcb.pix2544pix.invalid/000000000000000000000000000000005204000053039865802BR5912EMPRESA MOCK6009FORTALEZA62070503***6304FC55"
        );
    }
}
