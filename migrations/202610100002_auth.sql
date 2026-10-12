-- Additive migration: no demo users, no default credentials.
CREATE TABLE installation_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    completed_at timestamptz
);
INSERT INTO installation_state(singleton) VALUES (true);
CREATE TABLE app_user (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    installation_admin boolean NOT NULL DEFAULT false,
    credential_version bigint NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE auth_session (
    token_hash text PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES app_user(id),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX auth_session_user ON auth_session(user_id);
CREATE INDEX auth_session_expiry ON auth_session(expires_at);
CREATE TABLE auth_rate_limit (
    bucket text PRIMARY KEY,
    started_at timestamptz NOT NULL DEFAULT now(),
    attempts integer NOT NULL
);
CREATE TABLE installation_audit (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id bigint REFERENCES app_user(id),
    action text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
