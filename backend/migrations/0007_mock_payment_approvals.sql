ALTER TABLE mock_payment_intents
    ADD COLUMN approved_at timestamptz,
    ADD COLUMN approved_by uuid,
    ADD COLUMN approval_idempotency_key uuid,
    ADD CONSTRAINT mock_payment_approval_actor_fk
        FOREIGN KEY (company_id, approved_by) REFERENCES users(company_id, id),
    ADD CONSTRAINT mock_payment_approval_fields_check
        CHECK (
            (approved_at IS NULL AND approved_by IS NULL AND approval_idempotency_key IS NULL)
            OR
            (approved_at IS NOT NULL AND approved_by IS NOT NULL AND approval_idempotency_key IS NOT NULL)
        ),
    ADD CONSTRAINT mock_payment_approval_distinct_actor_check
        CHECK (approved_by IS NULL OR approved_by <> creator_id),
    ADD CONSTRAINT mock_payment_approval_idempotency_unique
        UNIQUE (company_id, approved_by, approval_idempotency_key),
    ADD CONSTRAINT mock_payment_approval_requires_review_check
        CHECK (approved_at IS NULL OR review_requested_at IS NOT NULL);

