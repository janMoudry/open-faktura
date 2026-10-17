CREATE TABLE sales_terminal (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 company_id bigint NOT NULL REFERENCES company(id),
 code text NOT NULL CHECK (code ~ '^[A-Z0-9_-]{1,32}$'),
 name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
 archived boolean NOT NULL DEFAULT false,
 revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
 UNIQUE(company_id,id), UNIQUE(company_id,code)
);
