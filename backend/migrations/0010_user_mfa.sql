CREATE TABLE user_mfa_credentials (
    company_id uuid NOT NULL,
    user_id uuid NOT NULL,
    encrypted_secret bytea NOT NULL,
    enabled_at timestamptz,
    pending_expires_at timestamptz,
    last_totp_counter bigint NOT NULL DEFAULT -1,
    failed_attempts integer NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (company_id, user_id),
    FOREIGN KEY (company_id, user_id) REFERENCES users(company_id, id),
    CHECK ((enabled_at IS NULL AND pending_expires_at IS NOT NULL) OR (enabled_at IS NOT NULL AND pending_expires_at IS NULL))
);
