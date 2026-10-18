-- Additive; old deliveries remain PDF attachments. Deploy backend before frontend.
CREATE TABLE invoice_share (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), request_id uuid NOT NULL,
 revision bigint NOT NULL, expires_days integer NOT NULL CHECK(expires_days BETWEEN 1 AND 365),
 token_hash text NOT NULL UNIQUE, url text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 revoked_at timestamptz, revoked_by bigint REFERENCES app_user(id),
 UNIQUE(company_id,id), UNIQUE(company_id,document_id,id), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,document_id) REFERENCES invoice_draft(company_id,id)
);
CREATE TABLE invoice_view (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL, share_id bigint NOT NULL,
 visit_id uuid NOT NULL, revision bigint NOT NULL,
 viewed_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(share_id,visit_id),
 FOREIGN KEY(company_id,document_id,share_id) REFERENCES invoice_share(company_id,document_id,id)
);
CREATE INDEX invoice_share_history ON invoice_share(company_id,document_id,id DESC);
CREATE INDEX invoice_view_history ON invoice_view(share_id,viewed_at DESC);
ALTER TABLE invoice_delivery ADD COLUMN share_id bigint;
ALTER TABLE invoice_delivery ADD CONSTRAINT invoice_delivery_share_fk
 FOREIGN KEY(company_id,document_id,share_id) REFERENCES invoice_share(company_id,document_id,id);
