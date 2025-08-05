use crate::state::AppState;
use async_std::task::sleep;
use chrono::DateTime;
use reqwest::Client;
use rss::Channel;
use scraper::{Html, Selector};
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct BlogService;

impl BlogService {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn start_blog_service(self: Arc<Self>, state: AppState) {
        tracing::info!("Blog service started");

        loop {
            self.timer_tick(&state).await;

            sleep(Duration::from_secs(15 * 60)).await;
        }
    }

    async fn timer_tick(&self, state: &AppState) {
        let client = Client::new();
        let resp = match client
            .get("https://blog.ethereum.org/en/feed.xml")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to fetch feed: {:?}", e);
                return;
            }
        };

        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to read response bytes: {:?}", e);
                return;
            }
        };

        let channel = match Channel::read_from(&bytes[..]) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to parse RSS feed: {:?}", e);
                return;
            }
        };

        for item in channel.items() {
            let guid = item.guid().map(|g| g.value()).unwrap_or_default();
            let result = sqlx::query!(
                "SELECT post_guid FROM blog_posts WHERE post_guid = $1",
                guid
            )
            .fetch_optional(&state.database.pool)
            .await;

            match result {
                Ok(Some(_)) => {
                    tracing::debug!("Blog post already exists: {}", guid);
                }
                Ok(None) => {
                    tracing::info!(
                        "New blog post found: {}",
                        item.title().unwrap_or("untitled")
                    );

                    let link = item.link().unwrap();
                    let html_resp = match reqwest::get(link).await {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("Failed to fetch blog post HTML: {:?}", e);
                            return;
                        }
                    };

                    let html_bytes = match html_resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!("Failed to read blog post HTML bytes: {:?}", e);
                            return;
                        }
                    };

                    let markdown_content = {
                        let html =
                            Html::parse_document(std::str::from_utf8(&html_bytes).unwrap_or(""));
                        let article_selector = Selector::parse("article").unwrap();
                        let article_html =
                            if let Some(article) = html.select(&article_selector).next() {
                                article.html()
                            } else {
                                tracing::warn!("No <article> tag found in blog post: {}", link);
                                continue;
                            };

                        let mut clean_html = article_html;

                        clean_html = regex::Regex::new(r"<style[^>]*>[\s\S]*?</style>")
                            .unwrap()
                            .replace_all(&clean_html, "")
                            .to_string();

                        clean_html = regex::Regex::new(r"<script[^>]*>[\s\S]*?</script>")
                            .unwrap()
                            .replace_all(&clean_html, "")
                            .to_string();

                        clean_html = regex::Regex::new(r#"\s+class="[^"]*""#)
                            .unwrap()
                            .replace_all(&clean_html, "")
                            .to_string();

                        clean_html = regex::Regex::new(r#"\s+style="[^"]*""#)
                            .unwrap()
                            .replace_all(&clean_html, "")
                            .to_string();

                        html2md::parse_html(&clean_html)
                    };

                    let image_url = {
                        let image_selector =
                            Selector::parse("main > div > div > span > img").unwrap();
                        let html_str = std::str::from_utf8(&html_bytes).unwrap_or("");
                        let image_element = Html::parse_document(html_str)
                            .select(&image_selector)
                            .next()
                            .and_then(|img| img.value().attr("src"))
                            .map(|src| src.to_string());
                        image_element.unwrap_or_default()
                    };

                    tracing::info!(
                        "Extracted {} characters of markdown content",
                        markdown_content.len()
                    );

                    if let Err(e) = sqlx::query!(
                        "INSERT INTO blog_posts (post_guid, title, content, content_description, pubDate, category, image_url) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                        guid,
                        item.title().unwrap_or("untitled"),
                        markdown_content,
                        item.description().unwrap_or(""),
                        {
                            let pub_date_str = item.pub_date().unwrap_or("1970-01-01T00:00:00Z");
                            DateTime::parse_from_rfc2822(pub_date_str)
                                .or_else(|_| DateTime::parse_from_rfc3339(pub_date_str))
                                .map(|dt| dt.naive_utc())
                                .unwrap_or_else(|_| DateTime::from_timestamp(0, 0).unwrap().naive_utc())
                        },
                        item.categories().get(0).map(|c| c.name()).unwrap_or("Uncategorized"),
                        image_url
                    )
                    .execute(&state.database.pool)
                    .await
                    {
                        tracing::error!("Failed to insert blog post: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Database error: {:?}", e);
                }
            }
        }
    }
}
