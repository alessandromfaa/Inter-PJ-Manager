ALTER TABLE bank_integrations ADD COLUMN binding_digest bytea UNIQUE;
ALTER TABLE bank_transactions ADD COLUMN inter_kind text,
 ADD COLUMN inter_current boolean NOT NULL DEFAULT true;
CREATE TABLE inter_read_state (
 account_id uuid PRIMARY KEY REFERENCES bank_integrations(account_id),
 imported_from date NOT NULL, imported_until date NOT NULL,
 balance_json text NOT NULL, synchronized_at timestamptz NOT NULL DEFAULT now(),
 CHECK(imported_from<=imported_until)
);
ALTER TABLE integration_sync_jobs ADD COLUMN error_code text;
