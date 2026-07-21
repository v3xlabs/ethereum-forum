-- Restore the staging tier dropped in 0017. Summarizer/digest `note_candidate`
-- calls write here; only the curator promotes entries to llm_memory.
CREATE TABLE llm_memory_staging (
    staging_id SERIAL PRIMARY KEY,
    term TEXT NOT NULL,
    content TEXT NOT NULL,
    source_discourse_id TEXT,
    source_topic_id INT,
    source_post_number INT,
    link_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_memory_staging_created ON llm_memory_staging(created_at DESC);
