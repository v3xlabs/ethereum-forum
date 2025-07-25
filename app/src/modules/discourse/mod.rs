use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    models::{
        discourse::{
            latest::DiscourseLatestResponse,
            topic::DiscourseTopicResponse,
            user::{DiscourseUserProfile, DiscourseUserSummaryResponse},
        },
        topics::{Topic, post::Post},
    },
    state::AppState,
};
use anyhow::{Error, Result};
use async_std::{
    channel::{Receiver, Sender},
    sync::Mutex,
};
use chrono::{DateTime, Utc};
use moka::future::Cache;
use poem_openapi::types::{ParseFromJSON, ToJSON, Type};
use serde::{Deserialize, Serialize};
use strip_tags::strip_tags;
use tracing::{error, info};

pub async fn fetch_latest_topics(discourse_url: &str) -> Result<DiscourseLatestResponse, Error> {
    let url = format!("{}/latest.json", discourse_url);
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseLatestResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub async fn fetch_latest_topics_paginated(
    discourse_url: &str,
    more_topics_url: Option<&str>,
) -> Result<DiscourseLatestResponse, Error> {
    let url = if let Some(more_url) = more_topics_url {
        if more_url.starts_with("/latest?") {
            let page_param = &more_url[8..]; // Remove "/latest?"
            format!("{}/latest.json?{}", discourse_url, page_param)
        } else if more_url.starts_with("/latest") {
            format!("{}/latest.json", discourse_url)
        } else {
            format!("{}{}", discourse_url, more_url)
        }
    } else {
        format!("{}/latest.json", discourse_url)
    };

    info!("Fetching URL: {}", url);

    let response = reqwest::get(&url).await?;
    let status = response.status();

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "HTTP error {}: Failed to fetch {}",
            status,
            url
        ));
    }

    let body: String = response.text().await?;

    let parsed: DiscourseLatestResponse = serde_json::from_str(&body).map_err(|e| {
        anyhow::anyhow!(
            "JSON parsing error for {}: {}. Body starts with: {}",
            url,
            e,
            &body[..std::cmp::min(200, body.len())]
        )
    })?;

    Ok(parsed)
}

pub async fn fetch_topic(
    discourse_url: &str,
    topic_id: TopicId,
    page: u32,
) -> Result<DiscourseTopicResponse, Error> {
    let url = format!("{}/t/{}.json?page={}", discourse_url, topic_id, page);
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseTopicResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub type TopicId = i32;

#[derive(Debug, Serialize, Deserialize)]
pub struct ForumSearchDocument {
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discourse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm_issue: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooked: Option<String>,
    pub entity_id: String,
}

#[derive(Debug)]
pub struct DiscourseTopicIndexRequest {
    pub topic_id: TopicId,
    pub page: u32,
}

#[derive(Debug, Clone, poem_openapi::Union)]
pub enum LResult<T: Send + Sync + Type + ToJSON + ParseFromJSON> {
    Failed(String),
    Success(T),
}

#[derive(Debug, Clone)]
pub struct DiscourseConfig {
    pub discourse_id: String,
    pub url: String,
    pub scrape_interval: String,
}

/// Main service that manages multiple discourse instances
pub struct DiscourseService {
    indexers: HashMap<String, Arc<DiscourseIndexer>>,
    user_profile_cache: Cache<String, LResult<DiscourseUserProfile>>,
    user_summary_cache: Cache<String, LResult<DiscourseUserSummaryResponse>>,
}

impl DiscourseService {
    pub fn new(configs: Vec<DiscourseConfig>) -> Self {
        let mut indexers = HashMap::new();

        for config in configs {
            let indexer = Arc::new(DiscourseIndexer::new(config.clone()));
            indexers.insert(config.discourse_id.clone(), indexer);
        }

        Self {
            indexers,
            user_profile_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60 * 60)) // 1 hour TTL
                .build(),
            user_summary_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60 * 60)) // 1 hour TTL
                .build(),
        }
    }

    pub async fn start_all_indexers(&self, state: AppState) {
        for (discourse_id, indexer) in &self.indexers {
            let indexer_clone = Arc::clone(indexer);
            let state_clone = state.clone();
            let discourse_id_clone = discourse_id.clone();

            async_std::task::spawn(async move {
                indexer_clone.run(state_clone).await;
            });

            info!("Started indexer for discourse: {}", discourse_id_clone);
        }
    }

    pub async fn enqueue(
        &self,
        discourse_id: &str,
        topic_id: TopicId,
        page: u32,
    ) -> Result<(), Error> {
        if let Some(indexer) = self.indexers.get(discourse_id) {
            indexer.enqueue(topic_id, page).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Discourse instance '{}' not found",
                discourse_id
            ))
        }
    }

    pub fn get_discourse_url(&self, discourse_id: &str) -> Option<String> {
        self.indexers
            .get(discourse_id)
            .map(|indexer| indexer.config.url.clone())
    }

    pub async fn fetch_discourse_user_cached(
        &self,
        discourse_id: &str,
        username: &str,
    ) -> Result<LResult<DiscourseUserProfile>, Error> {
        let discourse_url = self
            .get_discourse_url(discourse_id)
            .ok_or_else(|| anyhow::anyhow!("Discourse instance '{}' not found", discourse_id))?;

        let cache_key = format!("{}:{}", discourse_id, username);
        let username = username.to_string();

        Ok(self
            .user_profile_cache
            .get_with(cache_key, async move {
                match Self::fetch_discourse_user(&discourse_url, &username).await {
                    Ok(user) => LResult::Success(user),
                    Err(e) => LResult::Failed(e.to_string()),
                }
            })
            .await)
    }

    pub async fn fetch_discourse_user_summary_cached(
        &self,
        discourse_id: &str,
        username: &str,
    ) -> Result<LResult<DiscourseUserSummaryResponse>, Error> {
        let discourse_url = self
            .get_discourse_url(discourse_id)
            .ok_or_else(|| anyhow::anyhow!("Discourse instance '{}' not found", discourse_id))?;

        let cache_key = format!("{}:{}", discourse_id, username);
        let username = username.to_string();

        Ok(self
            .user_summary_cache
            .get_with(cache_key, async move {
                match Self::fetch_discourse_user_summary(&discourse_url, &username).await {
                    Ok(user) => LResult::Success(user),
                    Err(e) => LResult::Failed(e.to_string()),
                }
            })
            .await)
    }

    pub async fn fetch_discourse_user(
        discourse_url: &str,
        username: &str,
    ) -> anyhow::Result<DiscourseUserProfile> {
        let url = format!("{}/u/{}.json", discourse_url, username);
        let response = reqwest::get(url).await?;
        let body = response.text().await?;
        let parsed: DiscourseUserProfile = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn fetch_discourse_user_summary(
        discourse_url: &str,
        username: &str,
    ) -> Result<DiscourseUserSummaryResponse> {
        let url = format!("{}/u/{}/summary.json", discourse_url, username);
        let response = reqwest::get(url).await?;

        // Check if the response is a 404 (profile hidden or user not found)
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Return an empty summary response for hidden profiles
            return Ok(DiscourseUserSummaryResponse {
                topics: None,
                badges: None,
                badge_types: None,
                users: None,
                user_summary: None,
            });
        }

        let body = response.text().await?;
        let parsed: DiscourseUserSummaryResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }
}

/// Individual indexer for a single discourse instance
pub struct DiscourseIndexer {
    config: DiscourseConfig,
    topic_tx: Sender<DiscourseTopicIndexRequest>,
    topic_lock: Arc<Mutex<HashSet<(TopicId, u32)>>>,
    topic_rx: Receiver<DiscourseTopicIndexRequest>,
}

impl DiscourseIndexer {
    pub fn new(config: DiscourseConfig) -> Self {
        let (topic_tx, topic_rx) = async_std::channel::unbounded();
        Self {
            config,
            topic_tx,
            topic_lock: Arc::new(Mutex::new(HashSet::new())),
            topic_rx,
        }
    }

    pub async fn run(self: Arc<Self>, state: AppState) {
        let state_clone = state.clone();
        let indexer_clone = Arc::clone(&self);
        let topic_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM topics WHERE discourse_id = $1",
            self.config.discourse_id
        )
        .fetch_one(&state.database.pool)
        .await
        .map(|row| row.count.unwrap_or(0))
        .unwrap_or(0);

        if topic_count == 0 {
            async_std::task::spawn(async move {
                indexer_clone.fetch_all_topics(&state_clone).await;
            });
        } else {
            info!(
                "Skipping full topics fetch because topics already exist for {}",
                self.config.discourse_id
            );
        }

        info!(
            "Started indexer for {}, awaiting requests",
            self.config.discourse_id
        );

        // Process topic indexing requests
        while let Ok(request) = self.topic_rx.recv().await {
            info!(
                "Processing request for {}: {:?}",
                self.config.discourse_id, request
            );

            if let Ok(topic) = fetch_topic(&self.config.url, request.topic_id, request.page).await {
                let existing_topic =
                    Topic::get_by_topic_id(&self.config.discourse_id, topic.id, &state)
                        .await
                        .ok();
                let existing_messages = if let Some(existing) = &existing_topic {
                    Post::count_by_topic_id(&self.config.discourse_id, existing.topic_id, &state)
                        .await
                        .unwrap_or(0)
                } else {
                    0
                };

                let worth_fetching_more = existing_messages != topic.posts_count || {
                    let existing = existing_topic.unwrap();
                    let zero = DateTime::<Utc>::MIN_UTC;
                    let existing_time = existing.last_post_at.unwrap_or(zero);

                    existing.post_count != topic.posts_count
                        || existing_time < topic.last_posted_at
                        || existing_messages < topic.posts_count
                };

                if !worth_fetching_more {
                    info!(
                        "Topic {:?} is up to date ({} -> {}) skipping",
                        topic.id, existing_messages, topic.posts_count
                    );
                    self.topic_lock
                        .lock()
                        .await
                        .remove(&(request.topic_id, request.page));
                    continue;
                } else {
                    info!(
                        "Topic {:?} ({} -> {}) is worth fetching more, fetching",
                        topic.id, existing_messages, topic.posts_count
                    );
                }

                if !topic.post_stream.posts.is_empty() {
                    self.enqueue(request.topic_id, request.page + 1).await;
                }

                if request.page == 1 {
                    let topic_model = Topic::from_discourse(&self.config.discourse_id, &topic);

                    match topic_model.upsert(&state).await {
                        Ok(_) => {
                            info!("Upserted topic: {:?}", topic_model.topic_id);

                            if let Some(meili) = &state.meili {
                                let meili_doc = ForumSearchDocument {
                                    entity_type: "topic".to_string(),
                                    discourse_id: Some(self.config.discourse_id.clone()),
                                    topic_id: Some(topic_model.topic_id),
                                    post_id: None,
                                    post_number: None,
                                    user_id: None,
                                    username: None,
                                    title: Some(topic_model.title.clone()),
                                    slug: Some(topic_model.slug.clone()),
                                    pm_issue: topic_model.pm_issue,
                                    cooked: None,
                                    entity_id: format!("topic_{}", topic_model.topic_id),
                                };

                                let forum = meili.index("forum");

                                if let Err(e) = forum
                                    .add_documents(&[meili_doc], Some("entity_id"))
                                    .await
                                    .map_err(|e| {
                                        sqlx::Error::Io(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            e.to_string(),
                                        ))
                                    })
                                {
                                    error!("Error upserting topic to Meilisearch: {:?}", e);
                                }
                            }
                        }
                        Err(e) => error!("Error upserting topic: {:?}", e),
                    }
                }

                // Process posts
                let mut meili_docs = Vec::new();
                for discourse_post in topic.post_stream.posts {
                    let username = discourse_post.username.clone();
                    let post = Post::from_discourse(&self.config.discourse_id, discourse_post);
                    match post.upsert(&state).await {
                        Ok(_) => {
                            info!("Upserted post: {:?}", post.post_id);

                            if state.meili.is_some() {
                                meili_docs.push(ForumSearchDocument {
                                    entity_type: "post".to_string(),
                                    discourse_id: Some(self.config.discourse_id.clone()),
                                    topic_id: Some(post.topic_id),
                                    post_id: Some(post.post_id),
                                    post_number: Some(post.post_number),
                                    user_id: Some(post.user_id),
                                    username: Some(username),
                                    title: None,
                                    slug: None,
                                    pm_issue: None,
                                    cooked: post.cooked.as_deref().map(strip_tags),
                                    entity_id: format!("post_{}", post.post_id),
                                });
                            }
                        }
                        Err(e) => error!("Error upserting post: {:?}", e),
                    }
                }

                if let Some(meili) = &state.meili {
                    if !meili_docs.is_empty() {
                        let forum = meili.index("forum");
                        if let Err(e) = forum
                            .add_documents(&meili_docs, Some("entity_id"))
                            .await
                            .map_err(|e| {
                                sqlx::Error::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                ))
                            })
                        {
                            error!("Error bulk upserting posts to Meilisearch: {:?}", e);
                        }
                    }
                }
            }

            self.topic_lock
                .lock()
                .await
                .remove(&(request.topic_id, request.page));
        }

        error!("Indexer for {} stopped", self.config.discourse_id);
    }

    pub async fn enqueue(&self, topic_id: TopicId, page: u32) {
        info!(
            "Enqueuing topic {:?} page {} for {}",
            topic_id, page, self.config.discourse_id
        );
        let mut set = self.topic_lock.lock().await;
        let key = (topic_id, page);
        if set.insert(key) {
            let _ = self
                .topic_tx
                .send(DiscourseTopicIndexRequest { topic_id, page })
                .await;
            info!(
                "Enqueued topic {:?} page {} for {}",
                topic_id, page, self.config.discourse_id
            );
        } else {
            info!(
                "Topic {:?} page {} is already enqueued for {}, skipping",
                topic_id, page, self.config.discourse_id
            );
        }
    }

    pub async fn fetch_all_topics(&self, state: &AppState) {
        info!(
            "Starting complete topic fetch for {}",
            self.config.discourse_id
        );
        let mut more_topics_url: Option<String> = None;
        let mut page_count = 0;
        let mut total_topics = 0;
        let mut topics_queued = 0;

        loop {
            match fetch_latest_topics_paginated(&self.config.url, more_topics_url.as_deref()).await
            {
                Ok(response) => {
                    page_count += 1;
                    let topics_in_page = response.topic_list.topics.len();
                    total_topics += topics_in_page;

                    info!(
                        "Fetched page {} for {} with {} topics (total: {})",
                        page_count, self.config.discourse_id, topics_in_page, total_topics
                    );

                    if let Some(ref next_url) = response.topic_list.more_topics_url {
                        info!(
                            "Next page URL for {}: {}",
                            self.config.discourse_id, next_url
                        );
                    } else {
                        info!("No more pages for {}", self.config.discourse_id);
                    }

                    for topic in response.topic_list.topics {
                        let should_queue = match Topic::get_by_topic_id(
                            &self.config.discourse_id,
                            topic.id,
                            state,
                        )
                        .await
                        {
                            Ok(existing_topic) => {
                                let existing_posts = Post::count_by_topic_id(
                                    &self.config.discourse_id,
                                    topic.id,
                                    state,
                                )
                                .await
                                .unwrap_or(0);

                                existing_posts != topic.posts_count
                                    || existing_topic.post_count != topic.posts_count
                            }
                            Err(_) => true,
                        };

                        if should_queue {
                            info!(
                                "Topic ({}) for {}: {:?} - queuing for indexing",
                                topic.id, self.config.discourse_id, topic.title
                            );
                            self.enqueue(topic.id, 1).await;
                            topics_queued += 1;
                        } else {
                            info!(
                                "Topic ({}) for {}: {:?} - already up to date, skipping",
                                topic.id, self.config.discourse_id, topic.title
                            );
                        }
                    }

                    info!(
                        "Page {} complete: {} topics queued out of {} on this page",
                        page_count, topics_queued, topics_in_page
                    );

                    if let Some(next_url) = response.topic_list.more_topics_url {
                        more_topics_url = Some(next_url);

                        info!(
                            "Waiting 2 seconds before next page for {}",
                            self.config.discourse_id
                        );
                        async_std::task::sleep(Duration::from_secs(2)).await;
                    } else {
                        info!(
                            "Completed topic fetch for {} - {} pages, {} total topics, {} queued for indexing",
                            self.config.discourse_id, page_count, total_topics, topics_queued
                        );
                        break;
                    }
                }
                Err(e) => {
                    error!(
                        "Error fetching topics page {} for {}: {:?}",
                        page_count + 1,
                        self.config.discourse_id,
                        e
                    );

                    info!(
                        "Waiting 5 seconds before retry for {}",
                        self.config.discourse_id
                    );
                    async_std::task::sleep(Duration::from_secs(5)).await;

                    break;
                }
            }
        }
    }

    pub async fn fetch_latest(&self, _state: &AppState) -> anyhow::Result<()> {
        let topics = fetch_latest_topics(&self.config.url).await?;

        for topic in topics.topic_list.topics {
            info!(
                "Topic ({}) for {}: {:?}",
                topic.id, self.config.discourse_id, topic.title
            );
            self.enqueue(topic.id, 1).await;
            info!("Queued for {}", self.config.discourse_id);
        }

        Ok(())
    }
}

/// Helper function to create discourse configs from TOML-like structure
pub fn create_discourse_configs() -> Vec<DiscourseConfig> {
    vec![
        DiscourseConfig {
            discourse_id: "magicians".to_string(),
            url: "https://ethereum-magicians.org".to_string(),
            scrape_interval: "30m".to_string(),
        },
        DiscourseConfig {
            discourse_id: "research".to_string(),
            url: "https://ethresear.ch".to_string(),
            scrape_interval: "30m".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_fetch_latest_topics() {
        let result = fetch_latest_topics("https://ethereum-magicians.org")
            .await
            .unwrap();
        // assert!(result.topic_list.topics.len() > 0);

        println!("Active Users: {:?}", result.users.len());
    }
}
