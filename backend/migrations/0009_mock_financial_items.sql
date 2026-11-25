CREATE TYPE mock_financial_kind AS ENUM ('PAGAR', 'RECEBER');
CREATE TYPE mock_financial_state AS ENUM ('ABERTO', 'LIQUIDADO', 'CANCELADO');
CREATE TABLE mock_financial_categories (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL REFERENCES companies(id),
    name text NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 100),
    UNIQUE (company_id, id),
    UNIQUE (company_id, name)
);
CREATE TABLE mock_cost_centers (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL REFERENCES companies(id),
    name text NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 100),
    UNIQUE (company_id, id),
    UNIQUE (company_id, name)
);

CREATE TABLE mock_financial_items (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    kind mock_financial_kind NOT NULL,
    party_name text NOT NULL CHECK (length(trim(party_name)) BETWEEN 1 AND 160),
    description text NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 240),
    amount_cents bigint NOT NULL CHECK (amount_cents BETWEEN 1 AND 999999999999),
    due_on date NOT NULL,
    category_id uuid,
    cost_center_id uuid,
    state mock_financial_state NOT NULL DEFAULT 'ABERTO',
    created_at timestamptz NOT NULL DEFAULT now(),
    settled_at timestamptz,
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    FOREIGN KEY (company_id, category_id) REFERENCES mock_financial_categories(company_id, id),
    FOREIGN KEY (company_id, cost_center_id) REFERENCES mock_cost_centers(company_id, id),
    UNIQUE (company_id, id),
    CHECK ((state = 'LIQUIDADO') = (settled_at IS NOT NULL))
);

CREATE INDEX mock_financial_items_listing
    ON mock_financial_items(company_id, due_on, id);

CREATE TABLE mock_financial_reminders (
    company_id uuid NOT NULL,
    item_id uuid NOT NULL,
    reminder_date date NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (company_id, item_id, reminder_date),
    FOREIGN KEY (company_id, item_id) REFERENCES mock_financial_items(company_id, id)
);

CREATE TABLE mock_financial_recurring_rules (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    kind mock_financial_kind NOT NULL,
    party_name text NOT NULL CHECK (length(trim(party_name)) BETWEEN 1 AND 160),
    description text NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 240),
    amount_cents bigint NOT NULL CHECK (amount_cents BETWEEN 1 AND 999999999999),
    due_day smallint NOT NULL CHECK (due_day BETWEEN 1 AND 31),
    category_id uuid,
    cost_center_id uuid,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id, id),
    FOREIGN KEY (company_id, category_id) REFERENCES mock_financial_categories(company_id, id),
    FOREIGN KEY (company_id, cost_center_id) REFERENCES mock_cost_centers(company_id, id),
    UNIQUE (company_id, id)
);

CREATE TABLE mock_financial_recurring_occurrences (
    company_id uuid NOT NULL,
    rule_id uuid NOT NULL,
    period date NOT NULL,
    item_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (company_id, rule_id, period),
    UNIQUE (company_id, item_id),
    FOREIGN KEY (company_id, rule_id) REFERENCES mock_financial_recurring_rules(company_id, id),
    FOREIGN KEY (company_id, item_id) REFERENCES mock_financial_items(company_id, id)
);
