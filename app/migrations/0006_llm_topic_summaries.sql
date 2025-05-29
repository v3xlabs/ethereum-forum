CREATE TABLE IF NOT EXISTS topic_summaries (
    summary_id SERIAL PRIMARY KEY,
    topic_id INT NOT NULL,
    based_on TIMESTAMP WITH TIME ZONE NOT NULL,
    summary_text TEXT NOT NULL
);

ALTER TABLE topic_summaries ADD CONSTRAINT unique_topic_summaries_topic_id UNIQUE (topic_id);