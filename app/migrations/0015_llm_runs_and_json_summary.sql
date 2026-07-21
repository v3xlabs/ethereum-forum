ALTER TABLE topic_summaries
    ADD COLUMN based_on_post_number INT DEFAULT 0,
    ADD COLUMN summary_json JSONB DEFAULT NULL;

CREATE TABLE llm_runs (
    run_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_type TEXT NOT NULL CHECK (run_type IN ('summary', 'digest', 'curator')),
    discourse_id TEXT,
    topic_id INT,
    prompt_tokens INT DEFAULT 0,
    completion_tokens INT DEFAULT 0,
    total_tokens INT DEFAULT 0,
    reasoning_tokens INT DEFAULT 0,
    model_used TEXT,
    tool_calls INT DEFAULT 0,
    tool_rounds INT DEFAULT 0,
    duration_ms INT DEFAULT 0,
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'failure', 'truncated', 'cached')),
    error TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_runs_created_at ON llm_runs(created_at DESC);
CREATE INDEX idx_llm_runs_type ON llm_runs(run_type);
CREATE INDEX idx_llm_runs_topic ON llm_runs(discourse_id, topic_id);
