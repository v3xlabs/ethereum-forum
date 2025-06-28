CREATE TABLE IF NOT EXISTS github_issues (
    repository_url TEXT NOT NULL,
    id TEXT NOT NULL,
    number INT NOT NULL,
    title TEXT NOT NULL,
    state TEXT NOT NULL,
    "user" JSONB NOT NULL, -- "user" is a reserved keyword in PostgreSQL
    labels JSONB NOT NULL DEFAULT '[]',
    locked BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (repository_url, id)
);

CREATE TABLE IF NOT EXISTS github_issue_comments (
    repository_url TEXT NOT NULL,
    issue_id TEXT NOT NULL,
    id TEXT NOT NULL,
    "user" JSONB NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (repository_url, issue_id, id),
    FOREIGN KEY (repository_url, issue_id) REFERENCES github_issues(repository_url, id) ON DELETE CASCADE
);
