CREATE TYPE mock_rec_status AS ENUM ('AGUARDANDO_CONSENTIMENTO','ATIVA','RECUSADA','CANCELADA');
CREATE TYPE mock_rec_charge_status AS ENUM ('CRIADA','LIQUIDADA');

CREATE TABLE mock_pix_recurring_authorizations (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL REFERENCES companies(id),
    creator_id uuid NOT NULL,
    nome_pagador text NOT NULL CHECK (length(trim(nome_pagador)) BETWEEN 1 AND 120),
    objeto text NOT NULL CHECK (length(trim(objeto)) BETWEEN 1 AND 35),
    contrato text NOT NULL CHECK (length(trim(contrato)) BETWEEN 1 AND 35),
    periodicidade text NOT NULL CHECK (periodicidade IN ('SEMANAL','MENSAL','TRIMESTRAL','SEMESTRAL','ANUAL')),
    data_inicial date NOT NULL,
    data_final date,
    valor_centavos bigint NOT NULL CHECK (valor_centavos BETWEEN 1 AND 999999999999),
    status mock_rec_status NOT NULL DEFAULT 'AGUARDANDO_CONSENTIMENTO',
    solicitacao_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (company_id, creator_id) REFERENCES users(company_id,id),
    UNIQUE (company_id,id),
    UNIQUE (company_id,solicitacao_id),
    CHECK (data_final IS NULL OR data_final >= data_inicial)
);

CREATE TABLE mock_pix_recurring_charges (
    id uuid PRIMARY KEY,
    company_id uuid NOT NULL,
    authorization_id uuid NOT NULL,
    competencia date NOT NULL,
    valor_centavos bigint NOT NULL CHECK (valor_centavos BETWEEN 1 AND 999999999999),
    status mock_rec_charge_status NOT NULL DEFAULT 'CRIADA',
    end_to_end_id text,
    created_at timestamptz NOT NULL DEFAULT now(),
    settled_at timestamptz,
    FOREIGN KEY (company_id,authorization_id) REFERENCES mock_pix_recurring_authorizations(company_id,id),
    UNIQUE (company_id,id),
    UNIQUE (company_id,authorization_id,competencia),
    CHECK ((status = 'LIQUIDADA') = (settled_at IS NOT NULL)),
    CHECK ((status = 'LIQUIDADA') = (end_to_end_id IS NOT NULL))
);

CREATE INDEX mock_pix_recurring_company_status
    ON mock_pix_recurring_authorizations(company_id,status,created_at DESC);
CREATE INDEX mock_pix_recurring_charges_listing
    ON mock_pix_recurring_charges(company_id,competencia DESC);
