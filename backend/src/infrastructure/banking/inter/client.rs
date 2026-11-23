use super::{
    BankError,
    contract::{Balance, BalanceResponse, Page, Transaction},
};
use reqwest::{Client, RequestBuilder, StatusCode, header};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};
use time::{Date, Duration as CalendarDuration};
use tokio::{
    sync::Mutex,
    time::{Instant, sleep, sleep_until},
};
use zeroize::Zeroizing;

pub struct ClientInter {
    pub(super) http: Client,
    pub(super) base: String,
    account: String,
    client_id: String,
    secret: Zeroizing<String>,
    token: Mutex<Option<Token>>,
    oauth: Pacer,
    bank: Pacer,
}
struct Pacer {
    next: Mutex<Instant>,
    spacing: Duration,
}
impl Pacer {
    fn new(spacing: Duration) -> Self {
        Self {
            next: Mutex::new(Instant::now()),
            spacing,
        }
    }
    async fn wait(&self) {
        let mut next = self.next.lock().await;
        sleep_until(*next).await;
        *next = Instant::now() + self.spacing;
    }
    async fn defer(&self, delay: Duration) {
        let mut next = self.next.lock().await;
        *next = (*next).max(Instant::now() + delay);
    }
}
struct Token {
    raw: Zeroizing<String>,
    renew_at: Instant,
}
#[derive(Deserialize)]
struct Grant {
    access_token: String,
    expires_in: u64,
    token_type: String,
    scope: Option<String>,
}
impl ClientInter {
    pub(super) fn new(
        http: Client,
        base: String,
        account: String,
        client_id: String,
        secret: Zeroizing<String>,
    ) -> Self {
        Self {
            http,
            base,
            account,
            client_id,
            secret,
            token: Mutex::new(None),
            oauth: Pacer::new(Duration::from_secs(12)),
            bank: Pacer::new(Duration::from_secs(6)),
        }
    }
    #[cfg(test)]
    pub(super) fn fast_mock(mut self) -> Self {
        self.oauth = Pacer::new(Duration::ZERO);
        self.bank = Pacer::new(Duration::ZERO);
        self
    }
    async fn request<T: DeserializeOwned>(
        &self,
        pacer: &Pacer,
        f: impl Fn() -> RequestBuilder,
    ) -> Result<(T, Instant), BankError> {
        for attempt in 0..2 {
            pacer.wait().await;
            let start = Instant::now();
            let mut response = f().send().await.map_err(BankError::transport)?;
            let status = response.status();
            tracing::info!(
                status = status.as_u16(),
                elapsed_ms = start.elapsed().as_millis(),
                "Resposta da integração bancária"
            );
            if status == StatusCode::TOO_MANY_REQUESTS {
                let retry = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| {
                        s.parse::<u64>().ok().or_else(|| {
                            httpdate::parse_http_date(s).ok().map(|t| {
                                t.duration_since(SystemTime::now())
                                    .unwrap_or_default()
                                    .as_secs()
                                    .saturating_add(1)
                            })
                        })
                    })
                    .unwrap_or(2);
                pacer.defer(Duration::from_secs(retry.min(86400))).await;
                if attempt == 0 && retry <= 30 {
                    sleep(Duration::from_millis(
                        100 + u64::from(uuid::Uuid::new_v4().as_bytes()[0]),
                    ))
                    .await;
                    continue;
                }
                return Err(BankError::Limit);
            }
            if status != StatusCode::OK {
                return Err(BankError::Http(status.as_u16()));
            }
            let mut body = Zeroizing::new(Vec::new());
            while let Some(part) = response.chunk().await.map_err(BankError::transport)? {
                if body.len() + part.len() > 4 * 1024 * 1024 {
                    return Err(BankError::Contract);
                }
                body.extend_from_slice(&part);
            }
            return serde_json::from_slice(&body)
                .map(|value| (value, start))
                .map_err(|_| BankError::Contract);
        }
        Err(BankError::Limit)
    }
    async fn bearer(&self) -> Result<Zeroizing<String>, BankError> {
        let mut cache = self.token.lock().await;
        if let Some(token) = cache.as_ref().filter(|t| t.renew_at > Instant::now()) {
            return Ok(token.raw.clone());
        }
        let (grant, start): (Grant, Instant) = self
            .request(&self.oauth, || {
                self.http
                    .post(format!("{}/oauth/v2/token", self.base))
                    .form(&[
                        ("grant_type", "client_credentials"),
                        ("scope", "extrato.read"),
                        ("client_id", self.client_id.as_str()),
                        ("client_secret", self.secret.as_str()),
                    ])
            })
            .await?;
        if grant.expires_in == 0
            || grant.access_token.is_empty()
            || !grant.token_type.eq_ignore_ascii_case("bearer")
            || grant
                .scope
                .as_deref()
                .is_some_and(|s| s.split_whitespace().collect::<Vec<_>>() != ["extrato.read"])
        {
            return Err(BankError::Contract);
        }
        let lifetime = Duration::from_secs(grant.expires_in);
        let renew_at = start
            .checked_add(lifetime - (lifetime / 10).min(Duration::from_secs(60)))
            .ok_or(BankError::Contract)?;
        let raw = Zeroizing::new(grant.access_token);
        *cache = Some(Token {
            raw: raw.clone(),
            renew_at,
        });
        Ok(raw)
    }
    async fn read<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, BankError> {
        for attempt in 0..2 {
            let token = self.bearer().await?;
            let result = self
                .request(&self.bank, || {
                    self.http
                        .get(format!("{}{path}", self.base))
                        .bearer_auth(token.as_str())
                        .header("x-conta-corrente", &self.account)
                        .query(params)
                })
                .await;
            if matches!(result, Err(BankError::Http(401))) {
                let mut cache = self.token.lock().await;
                if cache.as_ref().is_some_and(|t| t.raw == token) {
                    *cache = None
                }
                if attempt == 0 {
                    continue;
                }
            }
            return result.map(|(value, _)| value);
        }
        Err(BankError::Http(401))
    }
    pub async fn balance(&self) -> Result<Balance, BankError> {
        self.read::<BalanceResponse>("/banking/v2/saldo", &[])
            .await?
            .convert()
    }
    pub async fn statement(&self, begin: Date, end: Date) -> Result<Vec<Transaction>, BankError> {
        let mut windows = vec![(begin, end)];
        let mut records = Vec::new();
        let mut ids = HashSet::new();
        while let Some((from, to)) = windows.pop() {
            let mut seen = 0;
            let mut expected = None;
            for index in 0..100 {
                let page: Page = self
                    .read(
                        "/banking/v2/extrato/completo",
                        &[
                            ("dataInicio", from.to_string()),
                            ("dataFim", to.to_string()),
                            ("pagina", index.to_string()),
                            ("tamanhoPagina", "500".into()),
                        ],
                    )
                    .await?;
                if index == 0 && page.total_elementos > 10000 {
                    if from == to {
                        return Err(BankError::Volume);
                    }
                    let middle = from + CalendarDuration::days((to - from).whole_days() / 2);
                    windows.push((middle + CalendarDuration::days(1), to));
                    windows.push((from, middle));
                    break;
                }
                if *expected.get_or_insert(page.total_elementos) != page.total_elementos
                    || page.numero_de_elementos != page.transacoes.len()
                {
                    return Err(BankError::Contract);
                }
                seen += page.numero_de_elementos;
                if seen > page.total_elementos
                    || page.ultima_pagina != (seen == page.total_elementos)
                    || (!page.ultima_pagina && page.numero_de_elementos == 0)
                {
                    return Err(BankError::Contract);
                }
                for item in page.transacoes {
                    if item.id_transacao.is_empty()
                        || item.id_transacao.len() > 256
                        || item.tipo_transacao.len() > 128
                        || !(from..=to).contains(&item.data_transacao)
                        || !ids.insert(item.id_transacao.clone())
                    {
                        return Err(BankError::Contract);
                    }
                    item.signed_cents()?;
                    records.push(item);
                    if records.len() > 100000 {
                        return Err(BankError::Volume);
                    }
                }
                if page.ultima_pagina {
                    break;
                }
                if index == 99 {
                    return Err(BankError::Volume);
                }
            }
        }
        Ok(records)
    }
}
