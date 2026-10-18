CREATE TABLE recurring_rule (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, company_id bigint NOT NULL REFERENCES company(id),
 actor_id bigint NOT NULL REFERENCES app_user(id), revision bigint NOT NULL DEFAULT 1,
 status text NOT NULL DEFAULT 'paused' CHECK(status IN ('paused','active','completed','error')),
 generated integer NOT NULL DEFAULT 0 CHECK(generated>=0), data jsonb NOT NULL,
 next_on date, error_code text, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(company_id,id)
);
CREATE INDEX recurring_due ON recurring_rule(next_on,id) WHERE status='active';
CREATE TABLE recurring_occurrence (
 company_id bigint NOT NULL,rule_id bigint NOT NULL,index integer NOT NULL,scheduled_on date NOT NULL,
 invoice_id bigint NOT NULL,snapshot jsonb NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 mail_request uuid NOT NULL,mail_command jsonb,delivery_id bigint,error_code text,next_attempt timestamptz NOT NULL DEFAULT now(),
 created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(company_id,rule_id,index),UNIQUE(company_id,invoice_id),
 FOREIGN KEY(company_id,rule_id) REFERENCES recurring_rule(company_id,id),
 FOREIGN KEY(company_id,invoice_id) REFERENCES invoice_draft(company_id,id),
 FOREIGN KEY(company_id,delivery_id) REFERENCES invoice_delivery(company_id,id)
);
CREATE TABLE recurring_command (
 company_id bigint NOT NULL,request_id uuid NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 rule_id bigint NOT NULL,command jsonb NOT NULL,PRIMARY KEY(company_id,request_id),
 FOREIGN KEY(company_id,rule_id) REFERENCES recurring_rule(company_id,id)
);
CREATE TABLE recurring_audit (
 company_id bigint NOT NULL,rule_id bigint NOT NULL,revision bigint NOT NULL,actor_id bigint NOT NULL REFERENCES app_user(id),
 action text NOT NULL,snapshot jsonb NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(company_id,rule_id,revision),FOREIGN KEY(company_id,rule_id) REFERENCES recurring_rule(company_id,id)
);
CREATE TRIGGER recurring_audit_immutable BEFORE UPDATE OR DELETE ON recurring_audit FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE TRIGGER recurring_command_immutable BEFORE UPDATE OR DELETE ON recurring_command FOR EACH ROW EXECUTE FUNCTION protect_payment_history();
CREATE INDEX recurring_pending_mail ON recurring_occurrence(next_attempt,created_at) WHERE mail_command IS NOT NULL AND delivery_id IS NULL;
