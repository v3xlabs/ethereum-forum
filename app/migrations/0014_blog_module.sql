CREATE TABLE IF NOT EXISTS blog_posts (
    post_guid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    content_description TEXT NOT NULL,
    pubDate TIMESTAMP NOT NULL,
    category TEXT NOT NULL,
    image_url TEXT NOT NULL
)
