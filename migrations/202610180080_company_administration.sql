-- P20 administration. Stop old API before upgrade; history is retained.
ALTER TABLE company_audit DROP CONSTRAINT company_audit_action_check;
ALTER TABLE company_audit ADD CONSTRAINT company_audit_action_check CHECK(action IN
 ('create','update','complete','delete','restore','member_add','member_role','member_remove','transfer','leave'));
CREATE TABLE company_admin_command (
 company_id bigint NOT NULL REFERENCES company(id), request_id uuid NOT NULL,
 actor_id bigint NOT NULL REFERENCES app_user(id), input jsonb NOT NULL, result jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(company_id,request_id)
);
