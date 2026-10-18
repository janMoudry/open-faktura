-- Account-owned display preferences, never financial calculations or authorization.
CREATE TABLE user_dashboard_layout (
 user_id bigint PRIMARY KEY REFERENCES app_user(id) ON DELETE CASCADE,
 revision bigint NOT NULL DEFAULT 1 CHECK (revision>0),
 widgets jsonb NOT NULL CHECK (jsonb_typeof(widgets)='array'),
 updated_at timestamptz NOT NULL DEFAULT now()
);
