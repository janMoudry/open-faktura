-- Explicit taxpayer decisions over source evidence; not a tax return or inferred entitlement.
CREATE TABLE tax_evidence_classification (
 company_id bigint NOT NULL REFERENCES company(id),
 kind text NOT NULL CHECK(kind IN ('invoice','credit_note','tax_receipt','retail','received','purchase_receipt','cash')),
 source_id bigint NOT NULL,
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 observation jsonb NOT NULL,
 data jsonb NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id),
 updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,kind,source_id)
);
CREATE TABLE tax_evidence_classification_audit (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id), kind text NOT NULL, source_id bigint NOT NULL,
 revision bigint NOT NULL, actor_id bigint NOT NULL REFERENCES app_user(id),
 observation jsonb NOT NULL, data jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE tax_evidence_classification_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL, result jsonb NOT NULL,
 PRIMARY KEY(company_id,request_id)
);
