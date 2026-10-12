-- Existing accounts keep an empty display name until their first update.
ALTER TABLE app_user ADD COLUMN name text NOT NULL DEFAULT '' CHECK (char_length(name) <= 120);
