CREATE TABLE llm_memory (
    entry_id SERIAL PRIMARY KEY,
    term TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    sources JSONB DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE llm_memory_staging (
    staging_id SERIAL PRIMARY KEY,
    term TEXT NOT NULL,
    content TEXT NOT NULL,
    source_discourse_id TEXT,
    source_topic_id INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE llm_memory_snapshots (
    snapshot_id SERIAL PRIMARY KEY,
    version INT NOT NULL,
    memory_snapshot JSONB NOT NULL,
    curator_run_id UUID,
    summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_memory_term ON llm_memory(term);
CREATE INDEX idx_llm_memory_staging_created ON llm_memory_staging(created_at DESC);
CREATE INDEX idx_llm_memory_snapshots_version ON llm_memory_snapshots(version DESC);
