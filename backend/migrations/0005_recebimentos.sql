CREATE TABLE mock_billing_requests (
 company_id uuid NOT NULL REFERENCES companies, code uuid NOT NULL, customer_id uuid NOT NULL,
 cents bigint NOT NULL CHECK(cents BETWEEN 1 AND 999999999999), due_on date NOT NULL,
 status text NOT NULL DEFAULT 'EM_PROCESSAMENTO' CHECK(status IN ('EM_PROCESSAMENTO','A_RECEBER','RECEBIDO','CANCELADO')),
 channel text CHECK(channel IN ('PIX','BOLETO')), created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,code), FOREIGN KEY(company_id,customer_id) REFERENCES customers(company_id,id),
 CHECK ((status='RECEBIDO') = (channel IS NOT NULL))
);
CREATE TABLE mock_pix_receipts (
 company_id uuid NOT NULL, e2e text NOT NULL CHECK(e2e ~ '^[a-zA-Z0-9]{32}$'), txid text NOT NULL,
 cents bigint NOT NULL CHECK(cents>0), received_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,e2e), UNIQUE(company_id,txid),
 FOREIGN KEY(company_id,txid) REFERENCES mock_pix_charges(company_id,txid)
);
-- Guarda somente as referências necessárias à consulta posterior, sem o payload do pagador.
CREATE TABLE collection_events (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 environment integration_environment NOT NULL, account_id uuid, family text NOT NULL CHECK(family IN ('PIX','V3')),
 reference text NOT NULL, event_key text NOT NULL, state job_state NOT NULL DEFAULT 'AGUARDANDO',
 attempts int NOT NULL DEFAULT 0, next_at timestamptz NOT NULL DEFAULT now(), created_at timestamptz NOT NULL DEFAULT now(),
 error_code text, CHECK((environment='MOCK')=(account_id IS NULL)),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 UNIQUE NULLS NOT DISTINCT(company_id, environment, account_id, family,event_key)
);
CREATE INDEX collection_event_work ON collection_events(environment,next_at,id) WHERE state='AGUARDANDO';
CREATE INDEX collection_event_history ON collection_events(company_id,environment,id DESC);
CREATE TABLE collection_results (
 company_id uuid NOT NULL REFERENCES companies, environment integration_environment NOT NULL,
 account_id uuid, family text NOT NULL CHECK(family IN ('PIX','V3')), reference text NOT NULL,
 txid text, status text NOT NULL, gross_cents bigint NOT NULL CHECK(gross_cents>=0),
 received_cents bigint NOT NULL CHECK(received_cents>=0), channel text CHECK(channel IN ('BOLETO','PIX')),
 reconciliation text NOT NULL CHECK(reconciliation IN ('CONCILIADO','PENDENTE','DIVERGENTE')),
 checked_at timestamptz NOT NULL DEFAULT now(), CHECK((environment='MOCK')=(account_id IS NULL)),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 UNIQUE NULLS NOT DISTINCT(company_id, environment, account_id, family, reference)
);
CREATE INDEX collection_results_txid ON collection_results(company_id,txid) WHERE txid IS NOT NULL;
CREATE TABLE sandbox_collection_intents (
 company_id uuid NOT NULL, account_id uuid NOT NULL, intent_id uuid NOT NULL, family text NOT NULL CHECK(family IN ('PIX','V3')),
 fingerprint bytea NOT NULL, cents bigint NOT NULL CHECK(cents>0), reference text,
 state text NOT NULL CHECK(state IN ('INDETERMINADA','ACEITA','REJEITADA')),
 actor uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,account_id,intent_id), FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 FOREIGN KEY(company_id,actor) REFERENCES users(company_id,id)
);
