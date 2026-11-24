-- Dados fictícios isolados das cobranças vinculadas a contas bancárias.
CREATE TABLE mock_pix_charges (
    company_id uuid NOT NULL REFERENCES companies,
    txid text NOT NULL CHECK (txid ~ '^M[0-9a-f]{32}$'),
    original_cents bigint NOT NULL CHECK (original_cents BETWEEN 1 AND 999999999999),
    copy_paste text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    creator_id uuid NOT NULL,
    PRIMARY KEY (company_id, txid),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id)
);
