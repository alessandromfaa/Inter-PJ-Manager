-- Estrutura para as fases seguintes. A API da Fase 1 não escreve nessas tabelas.
CREATE TABLE customers (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 name text NOT NULL CHECK(length(trim(name))>0), UNIQUE(company_id,id)
);
CREATE TABLE suppliers (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 name text NOT NULL CHECK(length(trim(name))>0), UNIQUE(company_id,id)
);
CREATE TABLE financial_categories (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 name text NOT NULL CHECK(length(trim(name))>0), UNIQUE(company_id,id)
);
CREATE TABLE cost_centers (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies,
 name text NOT NULL CHECK(length(trim(name))>0), UNIQUE(company_id,id)
);
CREATE TYPE integration_environment AS ENUM ('MOCK','SANDBOX','PRODUCTION');
CREATE TABLE bank_integrations (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL UNIQUE,
 environment integration_environment NOT NULL, certificate_expires_at timestamptz,
 scopes text[] NOT NULL DEFAULT '{}', FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id)
);
CREATE TABLE bank_transactions (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL,
 provider_reference text NOT NULL, amount_cents bigint NOT NULL, posted_at timestamptz NOT NULL,
 category_id uuid, FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id), UNIQUE(account_id,provider_reference), UNIQUE(company_id,id),
 FOREIGN KEY(company_id,category_id) REFERENCES financial_categories(company_id,id)
);
CREATE INDEX transactions_period ON bank_transactions(company_id,account_id,posted_at);
CREATE TABLE pix_charges (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL,
 provider_reference text NOT NULL, amount_cents bigint NOT NULL CHECK(amount_cents>0),
 provider_status text NOT NULL, updated_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id), UNIQUE(account_id,provider_reference)
);
CREATE TABLE billing_charges (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL,
 provider_reference text NOT NULL, amount_cents bigint NOT NULL CHECK(amount_cents>0),
 provider_status text NOT NULL, updated_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id), UNIQUE(account_id,provider_reference)
);
CREATE TABLE pix_payments (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL,
 provider_reference text NOT NULL, amount_cents bigint NOT NULL CHECK(amount_cents>0),
 provider_status text NOT NULL, updated_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id), UNIQUE(account_id,provider_reference)
);
CREATE TYPE payment_state AS ENUM ('RASCUNHO','EM_REVISAO','APROVADO','EM_ENVIO','INDETERMINADO','CONFIRMADO','REJEITADO','CANCELADO');
CREATE TABLE payment_batches (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL, FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 state payment_state NOT NULL DEFAULT 'RASCUNHO', UNIQUE(company_id,account_id,id)
);
CREATE TABLE payment_requests (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL, creator_id uuid NOT NULL,
 batch_id uuid, amount_cents bigint NOT NULL CHECK(amount_cents>0), idempotency_key uuid NOT NULL,
 state payment_state NOT NULL DEFAULT 'RASCUNHO', FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 FOREIGN KEY(company_id,creator_id) REFERENCES users(company_id,id),
 FOREIGN KEY(company_id,account_id,batch_id) REFERENCES payment_batches(company_id,account_id,id),
 UNIQUE(company_id,idempotency_key), UNIQUE(company_id,id,creator_id)
);
CREATE TABLE payment_approvals (
 company_id uuid NOT NULL, request_id uuid NOT NULL, creator_id uuid NOT NULL, approver_id uuid NOT NULL,
 approved_at timestamptz NOT NULL DEFAULT now(), CHECK(creator_id<>approver_id),
 FOREIGN KEY(company_id,request_id,creator_id) REFERENCES payment_requests(company_id,id,creator_id),
 FOREIGN KEY(company_id,approver_id) REFERENCES users(company_id,id), PRIMARY KEY(request_id,approver_id)
);
CREATE TABLE accounts_receivable (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies, party_id uuid NOT NULL,
 amount_cents bigint NOT NULL CHECK(amount_cents>0), due_date date NOT NULL, settled_at timestamptz,
 category_id uuid, cost_center_id uuid, UNIQUE(company_id,id),
 FOREIGN KEY(company_id,party_id) REFERENCES customers(company_id,id),
 FOREIGN KEY(company_id,category_id) REFERENCES financial_categories(company_id,id),
 FOREIGN KEY(company_id,cost_center_id) REFERENCES cost_centers(company_id,id)
);
CREATE INDEX accounts_receivable_due ON accounts_receivable(company_id,due_date) WHERE settled_at IS NULL;
CREATE TABLE accounts_payable (
 id uuid PRIMARY KEY, company_id uuid NOT NULL REFERENCES companies, party_id uuid NOT NULL,
 amount_cents bigint NOT NULL CHECK(amount_cents>0), due_date date NOT NULL, settled_at timestamptz,
 category_id uuid, cost_center_id uuid, UNIQUE(company_id,id),
 FOREIGN KEY(company_id,party_id) REFERENCES suppliers(company_id,id),
 FOREIGN KEY(company_id,category_id) REFERENCES financial_categories(company_id,id),
 FOREIGN KEY(company_id,cost_center_id) REFERENCES cost_centers(company_id,id)
);
CREATE INDEX accounts_payable_due ON accounts_payable(company_id,due_date) WHERE settled_at IS NULL;
CREATE TABLE reconciliation_records (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, transaction_id uuid NOT NULL UNIQUE,
 payable_id uuid, receivable_id uuid, CHECK(num_nonnulls(payable_id,receivable_id)=1),
 FOREIGN KEY(company_id,transaction_id) REFERENCES bank_transactions(company_id,id),
 FOREIGN KEY(company_id,payable_id) REFERENCES accounts_payable(company_id,id),
 FOREIGN KEY(company_id,receivable_id) REFERENCES accounts_receivable(company_id,id)
);
CREATE TYPE job_state AS ENUM ('AGUARDANDO','PROCESSANDO','CONCLUIDO','FALHOU','ESGOTADO');
CREATE TABLE webhook_events (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL, family text NOT NULL,
 event_key text NOT NULL, encrypted_payload bytea NOT NULL, received_at timestamptz NOT NULL DEFAULT now(),
 state job_state NOT NULL DEFAULT 'AGUARDANDO', attempts int NOT NULL DEFAULT 0 CHECK(attempts>=0),
 FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id), UNIQUE(account_id,family,event_key)
);
CREATE INDEX webhook_queue ON webhook_events(state,received_at);
CREATE TABLE integration_sync_jobs (
 id uuid PRIMARY KEY, company_id uuid NOT NULL, account_id uuid NOT NULL, FOREIGN KEY(company_id,account_id) REFERENCES bank_accounts(company_id,id),
 state job_state NOT NULL DEFAULT 'AGUARDANDO', created_at timestamptz NOT NULL DEFAULT now(), finished_at timestamptz
);
CREATE INDEX sync_history ON integration_sync_jobs(company_id,account_id,created_at DESC);
