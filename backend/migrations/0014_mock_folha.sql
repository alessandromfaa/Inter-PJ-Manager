-- Estruturas para dados sintéticos; não habilitam pagamentos nem integração externa.
CREATE TABLE mock_payroll_employees (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL REFERENCES companies(id),
    creator_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    name text NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 120),
    salary_cents bigint NOT NULL CHECK (salary_cents BETWEEN 1 AND 999999999999),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    UNIQUE (company_id, id),
    UNIQUE (company_id, creator_id, idempotency_key)
);

CREATE TABLE mock_payroll_entries (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    employee_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('FALTA_INJUSTIFICADA', 'FALTA_JUSTIFICADA', 'ADIANTAMENTO', 'PRODUCAO')),
    occurred_on date NOT NULL,
    amount_cents bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, employee_id) REFERENCES mock_payroll_employees(company_id, id),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    UNIQUE (company_id, creator_id, idempotency_key),
    CHECK ((kind IN ('ADIANTAMENTO', 'PRODUCAO') AND amount_cents IS NOT NULL AND amount_cents BETWEEN 1 AND 999999999999)
        OR (kind IN ('FALTA_INJUSTIFICADA', 'FALTA_JUSTIFICADA') AND amount_cents IS NULL))
);

CREATE INDEX mock_payroll_entries_period ON mock_payroll_entries(company_id, employee_id, occurred_on);
