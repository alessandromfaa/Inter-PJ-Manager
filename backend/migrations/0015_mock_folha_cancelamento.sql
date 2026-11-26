ALTER TABLE mock_payroll_entries
    ADD COLUMN cancelled_at timestamptz,
    ADD COLUMN cancelled_by uuid,
    ADD CONSTRAINT mock_payroll_entries_company_id_key UNIQUE (company_id, id),
    ADD CONSTRAINT mock_payroll_cancel_actor_fk FOREIGN KEY (company_id, cancelled_by) REFERENCES users(company_id, id),
    ADD CONSTRAINT mock_payroll_cancel_pair CHECK ((cancelled_at IS NULL) = (cancelled_by IS NULL));

-- Não apaga nem altera lançamentos existentes: duplicatas impedem a migration
-- e exigem análise antes da aplicação.
CREATE UNIQUE INDEX mock_payroll_one_active_absence_per_day
    ON mock_payroll_entries(company_id, employee_id, occurred_on)
    WHERE cancelled_at IS NULL AND kind IN ('FALTA_JUSTIFICADA', 'FALTA_INJUSTIFICADA');

CREATE TABLE mock_payroll_entry_cancellations (
    company_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    entry_id uuid NOT NULL,
    PRIMARY KEY (company_id, actor_id, idempotency_key),
    FOREIGN KEY (company_id, actor_id) REFERENCES users(company_id, id),
    FOREIGN KEY (company_id, entry_id) REFERENCES mock_payroll_entries(company_id, id)
);
