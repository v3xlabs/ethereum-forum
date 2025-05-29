CREATE TABLE IF NOT EXISTS topic_summaries (
    summary_id SERIAL PRIMARY KEY,
    topic_id INT NOT NULL,
    based_on TIMESTAMP WITH TIME ZONE NOT NULL,
    summary_text TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_topic_summaries_topic_id ON topic_summaries (topic_id);
