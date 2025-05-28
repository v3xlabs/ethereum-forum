use std::{collections::HashSet, sync::Arc, time::Duration};

use crate::{
    models::{
        discourse::{latest::DiscourseLatestResponse, topic::DiscourseTopicResponse},
        topics::{Post, Topic},
    },
    state::AppState,
};
use anyhow::Error;
use async_std::{
    channel::{Receiver, Sender},
    sync::Mutex,
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use tracing::{error, info};

pub async fn fetch_latest_topics() -> Result<DiscourseLatestResponse, Error> {
    let url = "https://ethereum-magicians.org/latest.json";
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseLatestResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub async fn fetch_topic(topic_id: TopicId, page: u32) -> Result<DiscourseTopicResponse, Error> {
    let url = format!("https://ethereum-magicians.org/t/{topic_id}.json?page={page}");
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    let parsed: DiscourseTopicResponse = serde_json::from_str(&body)?;
    Ok(parsed)
}

pub type TopicId = i32;

#[derive(Debug)]
pub struct DiscourseTopicIndexRequest {
    pub topic_id: TopicId,
    pub page: u32,
}

pub struct DiscourseService {
    topic_tx: Sender<DiscourseTopicIndexRequest>,
    topic_lock: Arc<Mutex<HashSet<(TopicId, u32)>>>,
    topic_rx: Receiver<DiscourseTopicIndexRequest>,
}

impl Default for DiscourseService {
    fn default() -> Self {
        let (topic_tx, topic_rx) = async_std::channel::unbounded();
        Self {
            topic_tx,
            topic_lock: Arc::new(Mutex::new(HashSet::new())),
            topic_rx,
        }
    }
}

impl DiscourseService {
    pub async fn run(&self, state: AppState) {
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

                if worth_fetching_more {
                    info!(
                        "Topic {:?} ({} -> {}) is worth fetching more, fetching",
                        topic.id, existing_messages, topic.posts_count
                    );
                } else {
                    info!(
                        "Topic {:?} is up to date ({} -> {}) skipping",
                        topic.id, existing_messages, topic.posts_count
                    );
                    self.topic_lock
                        .lock()
                        .await
                        .remove(&(request.topic_id, request.page));
                    continue;
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
                        Ok(()) => {
                            info!("Upserted topic: {:?}", topic.topic_id);
                        }
                        Err(e) => error!("Error upserting topic: {:?}", e),
                    }
                }

                // found topic
                for post in topic.post_stream.posts {
                    let post = Post::from_discourse(post);
                    match post.upsert(&state).await {
                        Ok(()) => {
                            info!("Upserted post: {:?}", post.post_id);
                        }
                        Err(e) => error!("Error upserting post: {:?}", e),
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
                Ok(()) => {
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
