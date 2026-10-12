-- New tables only. Catalog entries and all snapshots belong to exactly one profile.
CREATE TABLE catalog (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id bigint NOT NULL REFERENCES company(id),
    creator_id bigint NOT NULL REFERENCES app_user(id),
    request_id uuid NOT NULL,
    initial_data jsonb NOT NULL,
    data jsonb NOT NULL CHECK (jsonb_typeof(data) = 'object'),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    archived boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(company_id,request_id),
    UNIQUE(company_id,id)
);
CREATE INDEX catalog_directory ON catalog(company_id,archived,lower(data->>'name'),id);
CREATE TABLE catalog_audit (
    company_id bigint NOT NULL,
    catalog_id bigint NOT NULL,
    revision bigint NOT NULL,
    actor_id bigint NOT NULL REFERENCES app_user(id),
    action text NOT NULL CHECK(action IN ('create','update','archive','restore')),
    data jsonb NOT NULL,
    archived boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(company_id,catalog_id,revision),
    FOREIGN KEY(company_id,catalog_id) REFERENCES catalog(company_id,id)
);
