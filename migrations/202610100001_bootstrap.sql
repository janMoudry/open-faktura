-- Infrastructure only. Accounting entities await their verified specifications.
CREATE TABLE application_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO application_metadata (key, value) VALUES ('schema', 'bootstrap');
