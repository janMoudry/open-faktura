-- Additive; old deliveries remain PDF attachments. Deploy backend before frontend.
CREATE TABLE quote_share (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), request_id uuid NOT NULL,
 revision bigint NOT NULL, expires_days integer NOT NULL CHECK(expires_days BETWEEN 1 AND 365),
 token_hash text NOT NULL UNIQUE, url text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 revoked_at timestamptz, revoked_by bigint REFERENCES app_user(id),
 UNIQUE(company_id,id), UNIQUE(company_id,document_id,id), UNIQUE(company_id,request_id),
 FOREIGN KEY(company_id,document_id) REFERENCES sales_document(company_id,id)
);
CREATE TABLE quote_view (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL, document_id bigint NOT NULL, share_id bigint NOT NULL,
 visit_id uuid NOT NULL, revision bigint NOT NULL,
 viewed_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(share_id,visit_id),
 FOREIGN KEY(company_id,document_id,share_id) REFERENCES quote_share(company_id,document_id,id)
);
CREATE INDEX quote_share_history ON quote_share(company_id,document_id,id DESC);
CREATE INDEX quote_view_history ON quote_view(share_id,viewed_at DESC);
ALTER TABLE quote_delivery ADD COLUMN share_id bigint;
ALTER TABLE quote_delivery ADD CONSTRAINT quote_delivery_share_fk
 FOREIGN KEY(company_id,document_id,share_id) REFERENCES quote_share(company_id,document_id,id);
