-- Legacy issuer is intentionally unknown, not copied from today's company settings.
ALTER TABLE retail_posting ADD COLUMN issuer jsonb;
CREATE TABLE retail_posting_pdf (
 company_id bigint NOT NULL, posting_id bigint NOT NULL,
 revision bigint NOT NULL, language text NOT NULL CHECK(language IN ('cs','en')),
 content bytea NOT NULL,
 PRIMARY KEY(company_id,posting_id,revision,language),
 FOREIGN KEY(company_id,posting_id) REFERENCES retail_posting(company_id,id)
);
