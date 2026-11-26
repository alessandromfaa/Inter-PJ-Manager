-- Salário inicial permanece no cadastro; alterações valem a partir do pagamento informado.
CREATE TABLE mock_payroll_salary_changes (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    employee_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    effective_on date NOT NULL CHECK (extract(day FROM effective_on) IN (5, 20)),
    salary_cents bigint NOT NULL CHECK (salary_cents BETWEEN 1 AND 999999999999),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, employee_id) REFERENCES mock_payroll_employees(company_id, id),
    FOREIGN KEY (company_id, actor_id) REFERENCES users(company_id, id),
    UNIQUE (company_id, employee_id, effective_on),
    UNIQUE (company_id, actor_id, idempotency_key)
);
