CREATE TABLE IF NOT EXISTS github_issues (
    repository_url TEXT NOT NULL,
    id TEXT NOT NULL PRIMARY KEY,
    number INT NOT NULL,
    title TEXT NOT NULL,
    state TEXT NOT NULL,
    "user" JSONB NOT NULL, -- "user" is a reserved keyword in PostgreSQL
    labels TEXT NOT NULL,
    locked BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
);
