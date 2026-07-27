ALTER TABLE posts DROP CONSTRAINT posts_pkey;
ALTER TABLE posts ADD PRIMARY KEY (discourse_id, post_id);

ALTER TABLE topics DROP CONSTRAINT topics_pkey;
ALTER TABLE topics ADD PRIMARY KEY (discourse_id, topic_id);

ALTER TABLE posts DROP CONSTRAINT IF EXISTS unique_discourse_post_id;
ALTER TABLE topics DROP CONSTRAINT IF EXISTS unique_discourse_topic_id;
