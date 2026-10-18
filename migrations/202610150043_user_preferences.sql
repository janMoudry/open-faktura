CREATE TABLE user_preference (
 user_id bigint PRIMARY KEY REFERENCES app_user(id) ON DELETE CASCADE,
 revision bigint NOT NULL DEFAULT 1,
 theme text NOT NULL DEFAULT 'system' CHECK(theme IN('system','light','dark')),
 updated_at timestamptz NOT NULL DEFAULT now()
);
