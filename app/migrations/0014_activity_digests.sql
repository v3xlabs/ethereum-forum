CREATE TABLE IF NOT EXISTS activity_digests (
    digest_id SERIAL PRIMARY KEY,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    digest_text TEXT NOT NULL,
    topics_included JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_activity_digests_created_at ON activity_digests (created_at DESC);
