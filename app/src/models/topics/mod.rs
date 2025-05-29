use chrono::{DateTime, Utc};
use poem_openapi::Object;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query, query_as, query_scalar};
use tracing::info;

use crate::state::AppState;

use super::discourse::topic::{DiscourseTopicPost, DiscourseTopicResponse};

const POSTS_PER_PAGE: usize = 100;

#[derive(Debug, Serialize, Deserialize, FromRow, Object)]
pub struct Topic {
    pub topic_id: i32,
    pub title: String,
    pub slug: String,
    pub post_count: i32,
    pub view_count: i32,
    pub like_count: i32,
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_post_at: Option<DateTime<Utc>>,
    pub bumped_at: Option<DateTime<Utc>>,
    pub pm_issue: Option<i32>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object)]
pub struct TopicSummary {
    pub summary_id: i32,
    pub topic_id: i32,
    pub based_on: DateTime<Utc>,
    pub summary_text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i32,
    pub username: String,
    pub name: String,
    pub avatar_template: Option<String>,
    pub trust_level: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object)]
pub struct Post {
    pub post_id: i32,
    pub topic_id: i32,
    pub user_id: i32,
    pub post_number: i32,
    pub updated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub cooked: Option<String>,
    pub post_url: Option<String>,
    pub extra: Option<serde_json::Value>,
}

impl Post {
    pub fn from_discourse(post: DiscourseTopicPost) -> Self {
        Self {
            post_id: post.id,
            topic_id: post.topic_id,
            user_id: post.user_id,
            post_number: post.post_number,
            updated_at: Some(post.updated_at),
            created_at: Some(post.created_at),
            cooked: Some(post.cooked),
            post_url: post.post_url,
            extra: Some(post.extra),
        }
    }

    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!("INSERT INTO posts (post_id, topic_id, user_id, post_number, updated_at, cooked, post_url, extra) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (post_id) DO UPDATE SET post_id=$1, topic_id=$2, user_id=$3, post_number=$4, updated_at = $5, cooked = $6, post_url = $7, extra = $8",
            self.post_id,
            self.topic_id,
            self.user_id,
            self.post_number,
            self.updated_at,
            self.cooked,
            self.post_url,
            self.extra,
        )
        .execute(&state.database.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_topic_id(
        topic_id: i32,
        page: i32,
        size: Option<i32>,
        state: &AppState,
    ) -> Result<(Vec<Self>, bool), sqlx::Error> {
        let size = size.unwrap_or(POSTS_PER_PAGE as i32);
        let offset = (page - 1) * size;
        let posts = query_as!(
            Self,
            "SELECT * FROM posts WHERE topic_id = $1 ORDER BY post_number ASC LIMIT $2 OFFSET $3",
            topic_id,
            (size + 1) as i64,
            offset as i64
        )
        .fetch_all(&state.database.pool)
        .await?;

        let has_more = posts.len() == size as usize + 1;
        let posts = posts.into_iter().take(size as usize).collect();

        Ok((posts, has_more))
    }

    pub async fn count_by_topic_id(topic_id: i32, state: &AppState) -> Result<i32, sqlx::Error> {
        let count = query_scalar!("SELECT COUNT(*) FROM posts WHERE topic_id = $1", topic_id)
            .fetch_one(&state.database.pool)
            .await?;

        Ok(count.unwrap_or_default() as i32)
    }
}

impl Topic {
    pub fn from_discourse(topic: &DiscourseTopicResponse) -> Self {
        let mut pm_issue = None;

        if let Some(category_id) = topic.extra.get("category_id") {
            let category_id = category_id.as_i64().unwrap();

            // If "Protocol Calls"
            if category_id == 63 {
                info!("Found category_id: {}", category_id);
                // Get first post
                if let Some(first_post) = topic.post_stream.posts.first() {
                    info!("Found first post: {}", first_post.cooked);
                    if let Some(found_pm_issue) = try_extract_pm_issue(&first_post.cooked) {
                        info!("Found pm_issue: {}", found_pm_issue);
                        pm_issue = Some(found_pm_issue);
                    }
                }
            }
        }

        Self {
            topic_id: topic.id,
            title: topic.title.clone(),
            slug: topic.slug.clone(),
            like_count: topic.like_count,
            post_count: topic.posts_count,
            image_url: topic.image_url.clone(),
            last_post_at: Some(topic.last_posted_at),
            bumped_at: None,
            extra: Some(topic.extra.clone()),
            created_at: topic.created_at,
            view_count: topic.views,
            pm_issue,
        }
    }

    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!("INSERT INTO topics (topic_id, title, slug, post_count, view_count, like_count, image_url, created_at, last_post_at, bumped_at, extra, pm_issue) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (topic_id) DO UPDATE SET topic_id=$1, title=$2, slug=$3, post_count=$4, view_count=$5, like_count=$6, image_url=$7, created_at=$8, last_post_at=$9, bumped_at=$10, extra=$11, pm_issue=$12",
            self.topic_id,
            self.title,
            self.slug,
            self.post_count,
            self.view_count,
            self.like_count,
            self.image_url,
            self.created_at,
            self.last_post_at,
            self.bumped_at,
            self.extra,
            self.pm_issue,
        )
        .execute(&state.database.pool)
        .await?;
        Ok(())
    }

    pub async fn get_by_latest_post_at(state: &AppState) -> Result<Vec<Self>, sqlx::Error> {
        let topics = query_as!(
            Self,
            "SELECT * FROM topics ORDER BY last_post_at DESC LIMIT 20"
        )
        .fetch_all(&state.database.pool)
        .await?;

        Ok(topics)
    }

    // order by views and require that last_post_at is within 14 days
    pub async fn get_by_trending(state: &AppState) -> Result<Vec<Self>, sqlx::Error> {
        let topics = query_as!(
            Self,
            "SELECT * FROM topics WHERE last_post_at > NOW() - INTERVAL '14 days' ORDER BY view_count DESC LIMIT 20"
        )
        .fetch_all(&state.database.pool)
        .await?;

        Ok(topics)
    }

    pub async fn get_by_topic_id(topic_id: i32, state: &AppState) -> Result<Self, sqlx::Error> {
        let topic = query_as!(Self, "SELECT * FROM topics WHERE topic_id = $1", topic_id)
            .fetch_one(&state.database.pool)
            .await?;
        Ok(topic)
    }

    pub async fn get_first_post(&self, state: &AppState) -> Result<Post, sqlx::Error> {
        let post = query_as!(
            Post,
            "SELECT * FROM posts WHERE topic_id = $1 ORDER BY post_number ASC LIMIT 1",
            self.topic_id
        )
        .fetch_one(&state.database.pool)
        .await?;
        Ok(post)
    }

    pub async fn get_summary_by_topic_id(
        topic_id: i32,
        state: &AppState,
    ) -> Result<TopicSummary, sqlx::Error> {
        let summary = query_as!(
            TopicSummary,
            "SELECT * FROM topic_summaries WHERE topic_id = $1 ORDER BY based_on DESC LIMIT 1",
            topic_id
        )
        .fetch_optional(&state.database.pool)
        .await?;

        let topic = match Topic::get_by_topic_id(topic_id, state).await {
            Ok(topic) => topic,
            Err(_) => {
                return Err(sqlx::Error::RowNotFound);
            }
        };

        let summary = match summary {
            Some(s) => s,
            None => {
                return Self::create_new_summary(topic_id, state, &topic).await;
            }
        };

        let based_on = topic
            .last_post_at
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());

        if summary.based_on.timestamp() == based_on as i64 {
            return Ok(summary);
        }

        Self::create_new_summary(topic_id, state, &topic).await
    }

    async fn create_new_summary(
        topic_id: i32,
        state: &AppState,
        topic: &Topic,
    ) -> Result<TopicSummary, sqlx::Error> {
        let summary = "This is a mock summary".to_string();

        let based_on = topic
            .last_post_at
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());

        let based_on_datetime =
            DateTime::from_timestamp(based_on as i64, 0).unwrap_or_else(|| Utc::now());

        let summary = query_as!(
            TopicSummary,
            "INSERT INTO topic_summaries (topic_id, based_on, summary_text, created_at) VALUES ($1, $2, $3, NOW()) RETURNING *",
            topic_id,
            based_on_datetime,
            summary
            )
            .fetch_one(&state.database.pool)
            .await?;

        info!(
            "Created new summary for topic_id: {} with summary_id: {}",
            topic_id, summary.summary_id
        );

        Ok(summary)
    }
}

// Match for <a href=\"https://github.com/ethereum/pm/issues/1518\">GitHub Issue</a>
fn try_extract_pm_issue(cooked: &str) -> Option<i32> {
    let re = Regex::new(r#"https://github\.com/ethereum/pm/issues/(\d+)"#).unwrap();
    let caps = re.captures(cooked);

    caps.map(|caps| caps.get(1).unwrap().as_str().parse().unwrap())
}
