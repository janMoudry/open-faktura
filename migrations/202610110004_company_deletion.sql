-- Logical deletion preserves audit and future document references. Deploy with new server;
-- older binaries do not filter deleted_at and must not run after this feature is enabled.
ALTER TABLE company ADD COLUMN deleted_at timestamptz;
ALTER TABLE company_audit DROP CONSTRAINT company_audit_action_check;
ALTER TABLE company_audit ADD CONSTRAINT company_audit_action_check CHECK (action IN ('create','update','complete','delete'));
