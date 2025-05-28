CREATE TABLE IF NOT EXISTS topic_summaries (
    summary_id TEXT PRIMARY KEY,
    topic_id INT NOT NULL,
    freshness_score INT NOT NULL,
    summary_text TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_topic_summaries_topic_id ON topic_summaries (topic_id);