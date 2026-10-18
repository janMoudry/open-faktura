CREATE TABLE retail_series(company_id bigint NOT NULL REFERENCES company(id),year integer NOT NULL,next_value bigint NOT NULL CHECK(next_value>0),PRIMARY KEY(company_id,year));
CREATE TABLE retail_document(
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,company_id bigint NOT NULL REFERENCES company(id),
 number text NOT NULL,revision bigint NOT NULL DEFAULT 1,data jsonb NOT NULL,totals jsonb NOT NULL,allocations jsonb NOT NULL,
 terminal_id bigint,terminal jsonb,cancelled boolean NOT NULL DEFAULT false,pdf bytea,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id),UNIQUE(company_id,number),FOREIGN KEY(company_id,terminal_id) REFERENCES sales_terminal(company_id,id)
);
CREATE INDEX retail_document_list ON retail_document(company_id,id DESC);
CREATE TABLE retail_command(company_id bigint NOT NULL REFERENCES company(id),request_id uuid NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),input jsonb NOT NULL,result jsonb NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,request_id));
CREATE TABLE retail_audit(id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,company_id bigint NOT NULL,document_id bigint NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),revision bigint NOT NULL,reason text NOT NULL,snapshot jsonb NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),FOREIGN KEY(company_id,document_id) REFERENCES retail_document(company_id,id));
