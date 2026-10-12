-- Additive to P02. No seeded firms or memberships. DDL locks only these new tables.
CREATE TABLE company (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    creator_id bigint NOT NULL REFERENCES app_user(id),
    request_id uuid NOT NULL,
    initial_settings jsonb NOT NULL,
    settings jsonb NOT NULL CHECK (jsonb_typeof(settings) = 'object'),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    step integer NOT NULL DEFAULT 0 CHECK (step BETWEEN 0 AND 3),
    completed boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (creator_id, request_id)
);
CREATE TABLE membership (
    company_id bigint NOT NULL REFERENCES company(id),
    user_id bigint NOT NULL REFERENCES app_user(id),
    role text NOT NULL CHECK (role IN ('owner','editor','viewer')),
    PRIMARY KEY (company_id,user_id)
);
CREATE INDEX membership_user ON membership(user_id,company_id);
CREATE UNIQUE INDEX company_single_owner ON membership(company_id) WHERE role='owner';
CREATE TABLE company_tax_profile (
    company_id bigint NOT NULL REFERENCES company(id),
    revision bigint NOT NULL,
    mode text NOT NULL CHECK (mode IN ('non_payer','vat_payer')),
    effective_from date NOT NULL,
    PRIMARY KEY (company_id,revision)
);
CREATE TABLE company_audit (
    company_id bigint NOT NULL REFERENCES company(id),
    revision bigint NOT NULL,
    actor_id bigint NOT NULL REFERENCES app_user(id),
    action text NOT NULL CHECK(action IN ('create','update','complete')),
    settings jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(company_id,revision)
);
