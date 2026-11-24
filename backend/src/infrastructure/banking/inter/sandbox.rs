use super::{BankError, InterBank, material};
use crate::recebimentos::{OrderState, SandboxOrder};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx_core::{query::query, query_scalar::query_scalar, row::Row};
use uuid::Uuid;

impl InterBank {
    pub(super) async fn sandbox_order(
        &self,
        actor: Uuid,
        order: SandboxOrder,
    ) -> Result<OrderState, BankError> {
        let (intent, raw, family) = order.fields();
        let cents = crate::api::positive_cents(raw).map_err(|_| BankError::Period)?;
        let decimal = format!("{}.{:02}", cents / 100, cents % 100);
        let fp =
            Sha256::digest(serde_json::to_vec(&order).map_err(|_| BankError::Contract)?).to_vec();
        let mut tx = self.db.begin().await?;
        let inserted=query("INSERT INTO sandbox_collection_intents(company_id,account_id,intent_id,family,fingerprint,cents,state,actor)
                VALUES($1,$2,$3,$4,$5,$6,'INDETERMINADA',$7) ON CONFLICT DO NOTHING")
            .bind(self.company).bind(self.account).bind(intent).bind(family).bind(&fp).bind(cents).bind(actor).execute(&mut *tx).await?.rows_affected()==1;
        if !inserted {
            let row=query("SELECT fingerprint,state,reference FROM sandbox_collection_intents WHERE company_id=$1 AND account_id=$2 AND intent_id=$3")
                .bind(self.company).bind(self.account).bind(intent).fetch_one(&mut *tx).await?;
            if row.get::<Vec<u8>, _>("fingerprint") != fp {
                return Err(BankError::PixConflict);
            }
            let out = OrderState {
                intent_id: intent,
                reference: row.get("reference"),
                state: row.get("state"),
            };
            tx.commit().await?;
            return Ok(out);
        }
        let dir = std::path::PathBuf::from(
            std::env::var("INTER_CREDENTIALS_ROOT").map_err(|_| BankError::Certificate)?,
        )
        .join("sandbox");
        let pix_txid = (family == "PIX").then(|| intent.simple().to_string());
        let body = match order {
            SandboxOrder::Pix { .. } => {
                let key =
                    material(&dir.join("pix_key"), true).map_err(|_| BankError::Certificate)?;
                let key = std::str::from_utf8(&key)
                    .map_err(|_| BankError::Contract)?
                    .trim();
                if key.is_empty() || key.len() > 77 {
                    return Err(BankError::Contract);
                }
                json!({"calendario":{"expiracao":86400},"valor":{"original":decimal},"chave":key})
            }
            SandboxOrder::Billing {
                customer_id,
                due_on,
                ..
            } => {
                if !(250..=9999999999).contains(&cents) || due_on < super::today() {
                    return Err(BankError::Period);
                }
                let name: String =
                    query_scalar("SELECT name FROM customers WHERE company_id=$1 AND id=$2")
                        .bind(self.company)
                        .bind(customer_id)
                        .fetch_optional(&mut *tx)
                        .await?
                        .ok_or(BankError::Account)?;
                if name.chars().count() > 100 {
                    return Err(BankError::Period);
                }
                let fixture = material(&dir.join("pagador_teste.json"), true)
                    .map_err(|_| BankError::Certificate)?;
                #[derive(serde::Deserialize, serde::Serialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Payer {
                    cpf_cnpj: String,
                    tipo_pessoa: String,
                    cep: String,
                    endereco: String,
                    cidade: String,
                    uf: String,
                }
                let p: Payer = serde_json::from_slice(&fixture).map_err(|_| BankError::Contract)?;
                if !matches!(
                    (p.tipo_pessoa.as_str(), p.cpf_cnpj.len()),
                    ("JURIDICA", 14) | ("FISICA", 11)
                ) || !p
                    .cpf_cnpj
                    .bytes()
                    .chain(p.cep.bytes())
                    .all(|b| b.is_ascii_digit())
                    || p.cep.len() != 8
                    || p.uf.len() != 2
                    || p.cidade.is_empty()
                    || p.endereco.is_empty()
                {
                    return Err(BankError::Contract);
                }
                let mut pagador = serde_json::to_value(p).map_err(|_| BankError::Contract)?;
                pagador["nome"] = Value::String(name);
                let amount: serde_json::Number =
                    decimal.parse().map_err(|_| BankError::Contract)?;
                json!({"seuNumero":&intent.simple().to_string()[..15],"dataVencimento":due_on.to_string(),"valorNominal":amount,
                    "numDiasAgenda":0,"pagador":pagador,"formasRecebimento":["BOLETO","PIX"]})
            }
        };
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,'Intenção de emissão em SANDBOX registrada')")
            .bind(self.company).bind(actor).execute(&mut *tx).await?;
        tx.commit().await?;
        // O commit anterior é a barreira contra reenvio após interrupção ou timeout.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            self.client.submit_collection(pix_txid.as_deref(), &body),
        )
        .await;
        let (state, reference) = match result {
            Ok(Ok(id)) => ("ACEITA", Some(id)),
            Ok(Err(BankError::Http(400 | 403 | 422))) => ("REJEITADA", None),
            _ => ("INDETERMINADA", pix_txid),
        };
        let mut tx = self.db.begin().await?;
        if let Some(id) = reference.as_ref().filter(|_| state == "ACEITA") {
            let sql = if family == "PIX" {
                "INSERT INTO pix_charges(id,company_id,account_id,provider_reference,amount_cents,provider_status) VALUES($1,$2,$3,$4,$5,'ATIVA')"
            } else {
                "INSERT INTO billing_charges(id,company_id,account_id,provider_reference,amount_cents,provider_status) VALUES($1,$2,$3,$4,$5,'EM_PROCESSAMENTO')"
            };
            query(sql)
                .bind(Uuid::new_v4())
                .bind(self.company)
                .bind(self.account)
                .bind(id)
                .bind(cents)
                .execute(&mut *tx)
                .await?;
            if family == "V3" {
                crate::recebimentos::enqueue(
                    &mut tx,
                    crate::recebimentos::Scope {
                        company: self.company,
                        account: Some(self.account),
                        mode: self.mode,
                    },
                    "V3",
                    id,
                    &format!("emitida:{id}"),
                )
                .await?;
            }
        }
        query("UPDATE sandbox_collection_intents SET state=$4,reference=$5 WHERE company_id=$1 AND account_id=$2 AND intent_id=$3")
            .bind(self.company).bind(self.account).bind(intent).bind(state).bind(&reference).execute(&mut *tx).await?;
        query("INSERT INTO audit_logs(company_id,actor,action) VALUES($1,$2,$3)")
            .bind(self.company)
            .bind(actor)
            .bind(format!("Resultado da emissão em SANDBOX: {state}"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(OrderState {
            intent_id: intent,
            reference,
            state: state.into(),
        })
    }
}
