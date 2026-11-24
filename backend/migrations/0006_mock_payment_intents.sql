CREATE TABLE mock_payment_intents (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    recipient text NOT NULL CHECK (recipient IN ('FORNECEDOR_DEMO', 'SERVICOS_DEMO')),
    amount_cents bigint NOT NULL CHECK (amount_cents BETWEEN 1 AND 999999999999),
    created_at timestamptz NOT NULL DEFAULT now(),
    review_requested_at timestamptz,
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    UNIQUE (company_id, creator_id, idempotency_key)
);

CREATE INDEX mock_payment_intents_listing ON mock_payment_intents(company_id, created_at DESC, id DESC);
