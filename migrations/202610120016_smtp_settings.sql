CREATE TABLE smtp_settings (
 id BOOLEAN PRIMARY KEY DEFAULT true CHECK(id), revision BIGINT NOT NULL DEFAULT 0,
 mode TEXT NOT NULL DEFAULT 'environment' CHECK(mode IN ('environment','custom','disabled')),
 host TEXT NOT NULL DEFAULT '', port INTEGER NOT NULL DEFAULT 587 CHECK(port BETWEEN 1 AND 65535),
 security TEXT NOT NULL DEFAULT 'starttls' CHECK(security IN ('starttls','tls','plain')),
 sender TEXT NOT NULL DEFAULT '', username TEXT NOT NULL DEFAULT '', password BYTEA,
 updated_by BIGINT REFERENCES app_user(id), updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO smtp_settings(id) VALUES(true);
CREATE TABLE smtp_test (
 request_id UUID PRIMARY KEY, actor_id BIGINT NOT NULL REFERENCES app_user(id),
 revision BIGINT NOT NULL, recipient TEXT NOT NULL, language TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('sending','sent','failed','uncertain')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), finished_at TIMESTAMPTZ,
 error_code TEXT
);
CREATE INDEX smtp_test_recent ON smtp_test(created_at DESC);
