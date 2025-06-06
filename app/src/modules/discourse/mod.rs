use std::{collections::HashSet, sync::Arc, time::Duration};

use crate::{
    models::{
        discourse::{
            latest::DiscourseLatestResponse,
            topic::DiscourseTopicResponse,
            user::{DiscourseUserProfile, DiscourseUserSummaryResponse},
        },
        topics::{Post, Topic},
    },
    state::AppState,
};
use anyhow::{Error, Result};
use async_std::{
    channel::{Receiver, Sender},
    sync::Mutex,
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use moka::future::Cache;
use poem_openapi::types::{ParseFromJSON, ToJSON, Type};
use serde::{Deserialize, Serialize};
use strip_tags::strip_tags;
use tracing::{error, info};

pub async fn fetch_latest_topics() -> Result<DiscourseLatestResponse, Error> {
    let url = "https://ethereum-magicians.org/latest.json";
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseLatestResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub async fn fetch_topic(topic_id: TopicId, page: u32) -> Result<DiscourseTopicResponse, Error> {
    let url = format!(
        "https://ethereum-magicians.org/t/{}.json?page={}",
        topic_id, page
    );
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseTopicResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub type TopicId = i32;

#[derive(Debug, Serialize, Deserialize)]
pub struct ForumSearchDocument {
    pub entity_type: String,
    pub topic_id: Option<i32>,
    pub post_id: Option<i32>,
    pub post_number: Option<i32>,
    pub user_id: Option<i32>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub pm_issue: Option<i32>,
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

pub struct DiscourseService {
    topic_tx: Sender<DiscourseTopicIndexRequest>,
    topic_lock: Arc<Mutex<HashSet<(TopicId, u32)>>>,
    topic_rx: Receiver<DiscourseTopicIndexRequest>,
    user_profile_cache: Cache<String, LResult<DiscourseUserProfile>>,
    user_summary_cache: Cache<String, LResult<DiscourseUserSummaryResponse>>,
}

impl Default for DiscourseService {
    fn default() -> Self {
        let (topic_tx, topic_rx) = async_std::channel::unbounded();
        Self {
            topic_tx,
            topic_lock: Arc::new(Mutex::new(HashSet::new())),
            topic_rx,
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
}

impl DiscourseService {
    pub async fn run(&self, state: AppState) {
        // spawn task fetch periodically
        let state2 = state.clone();
        let _ = async_std::task::spawn(async move {
            state2.discourse.fetch_periodically().await;
        });

        while let Ok(request) = self.topic_rx.recv().await {
            // self.topic_lock.lock().await.insert(request.topic_id);
            info!("Received request: {:?}", request);

            if let Ok(topic) = fetch_topic(request.topic_id, request.page).await {
                let existing_topic = Topic::get_by_topic_id(topic.id, &state).await.ok();
                let existing_messages = if let Some(existing) = &existing_topic {
                    Post::count_by_topic_id(existing.topic_id, &state)
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
                    state
                        .discourse
                        .enqueue(request.topic_id, request.page + 1)
                        .await;
                }

                if request.page == 1 {
                    let topic = Topic::from_discourse(&topic);

                    match topic.upsert(&state).await {
                        Ok(_) => {
                            info!("Upserted topic: {:?}", topic.topic_id);

                            if let Some(meili) = &state.meili {
                                let meili_doc = ForumSearchDocument {
                                    entity_type: "topic".to_string(),
                                    topic_id: Some(topic.topic_id),
                                    post_id: None,
                                    post_number: None,
                                    user_id: None,
                                    title: Some(topic.title.clone()),
                                    slug: Some(topic.slug.clone()),
                                    pm_issue: topic.pm_issue,
                                    cooked: None,
                                    entity_id: format!("topic_{}", topic.topic_id),
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

                // found topic
                let mut meili_docs = Vec::new();
                for post in topic.post_stream.posts {
                    let post = Post::from_discourse(post);
                    match post.upsert(&state).await {
                        Ok(_) => {
                            info!("Upserted post: {:?}", post.post_id);

                            if state.meili.is_some() {
                                meili_docs.push(ForumSearchDocument {
                                    entity_type: "post".to_string(),
                                    topic_id: Some(post.topic_id),
                                    post_id: Some(post.post_id),
                                    post_number: Some(post.post_number),
                                    user_id: Some(post.user_id),
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
    }

    pub async fn enqueue(&self, topic_id: TopicId, page: u32) {
        let mut set = self.topic_lock.lock().await;
        let key = (topic_id, page);
        if set.insert(key) {
            // only send if newly inserted
            let _ = self
                .topic_tx
                .send(DiscourseTopicIndexRequest { topic_id, page })
                .await;
        } else {
            info!("Topic {:?} is already enqueued, skipping", topic_id);
        }
    }

    pub async fn fetch_latest(&self) -> anyhow::Result<()> {
        // fetch discourse topics
        let topics = fetch_latest_topics().await?;

        for topic in topics.topic_list.topics {
            info!("Topic ({}): {:?}", topic.id, topic.title);
            self.enqueue(topic.id, 1).await;
            info!("Queued");
        }

        Ok(())
    }

    // trigger once on startup and then at exactly every round 30 minute mark cron style
    pub async fn fetch_periodically(&self) {
        loop {
            match self.fetch_latest().await {
                Ok(_) => {
                    info!("Fetched latest topics");
                }
                Err(e) => {
                    error!("Error fetching latest topics: {:?}", e);
                }
            }

            let now = Utc::now();
            let next = now.duration_round_up(TimeDelta::minutes(30)).unwrap();

            info!("Next fetch at: {:?}", next);

            let duration = next.signed_duration_since(now);
            async_std::task::sleep(Duration::from_secs(duration.num_seconds() as u64)).await;
        }
    }
}

impl DiscourseService {
    pub async fn fetch_discourse_user(username: &str) -> anyhow::Result<DiscourseUserProfile> {
        let url = format!("https://ethereum-magicians.org/u/{}.json", username);
        let response = reqwest::get(url).await?;
        let body = response.text().await?;
        let parsed: DiscourseUserProfile = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn fetch_discourse_user_cached(
        &self,
        username: &str,
    ) -> LResult<DiscourseUserProfile> {
        let username = username.to_string();
        self.user_profile_cache
            .get_with(username.clone(), async move {
                match DiscourseService::fetch_discourse_user(&username).await {
                    Ok(user) => LResult::Success(user),
                    Err(e) => LResult::Failed(e.to_string()),
                }
            })
            .await
    }

    pub async fn fetch_discourse_user_summary(
        username: &str,
    ) -> Result<DiscourseUserSummaryResponse> {
        let url = format!("https://ethereum-magicians.org/u/{}/summary.json", username);
        let response = reqwest::get(url).await?;
        let body = response.text().await?;
        let parsed: DiscourseUserSummaryResponse = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn fetch_discourse_user_summary_cached(
        &self,
        username: &str,
    ) -> LResult<DiscourseUserSummaryResponse> {
        let username = username.to_string();
        self.user_summary_cache
            .get_with(username.clone(), async move {
                match DiscourseService::fetch_discourse_user_summary(&username).await {
                    Ok(user) => LResult::Success(user),
                    Err(e) => LResult::Failed(e.to_string()),
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_fetch_latest_topics() {
        let result = fetch_latest_topics().await.unwrap();
        // assert!(result.topic_list.topics.len() > 0);

        println!("Active Users: {:?}", result.users.len());
    }
}
