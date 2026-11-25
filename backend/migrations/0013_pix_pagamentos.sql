-- Preparação local: nenhuma rota de envio é habilitada por esta estrutura.
CREATE TABLE pix_payment_intents (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    account_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    approver_id uuid,
    amount_cents bigint NOT NULL CHECK (amount_cents > 0),
    recipient_ciphertext bytea NOT NULL CHECK (octet_length(recipient_ciphertext) > 0),
    idempotency_key uuid NOT NULL,
    state text NOT NULL DEFAULT 'RASCUNHO' CHECK (state IN
        ('RASCUNHO', 'EM_REVISAO', 'APROVADO', 'EM_ENVIO', 'INDETERMINADO',
         'AGUARDANDO_APROVACAO', 'PROCESSADO', 'AGENDADO', 'PAGO', 'REJEITADO', 'CANCELADO')),
    provider_reference uuid,
    bank_status text,
    created_at timestamptz NOT NULL DEFAULT now(),
    approved_at timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, account_id) REFERENCES bank_accounts(company_id, id),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    FOREIGN KEY (company_id, approver_id) REFERENCES users(company_id, id),
    UNIQUE (company_id, idempotency_key),
    UNIQUE (company_id, account_id, provider_reference),
    CHECK (approver_id IS NULL OR approver_id <> creator_id),
    CHECK ((approver_id IS NULL) = (approved_at IS NULL)),
    CHECK (state NOT IN ('APROVADO', 'EM_ENVIO', 'INDETERMINADO',
        'AGUARDANDO_APROVACAO', 'PROCESSADO', 'AGENDADO', 'PAGO') OR approved_at IS NOT NULL)
);

CREATE INDEX pix_payment_intents_listing ON pix_payment_intents(company_id, account_id, created_at DESC);
