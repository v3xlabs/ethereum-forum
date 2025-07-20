use chrono::{DateTime, Utc};
use opentelemetry_http::HttpError;
use poem_openapi::Object;
use post::Post;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query, query_as};
use tracing::info;

use crate::state::AppState;

use super::discourse::topic::DiscourseTopicResponse;

pub mod post;

const POSTS_PER_PAGE: usize = 100;

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct Topic {
    pub discourse_id: String,
    pub topic_id: i32,
    pub title: String,
    pub slug: String,
    pub post_count: i32,
    pub view_count: i32,
    pub like_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_post_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bumped_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm_issue: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object)]
pub struct TopicSummary {
    pub summary_id: i32,
    pub discourse_id: String,
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

impl Topic {
    pub fn from_discourse(discourse_id: &str, topic: &DiscourseTopicResponse) -> Self {
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
            discourse_id: discourse_id.to_string(),
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
        query!("INSERT INTO topics (discourse_id, topic_id, title, slug, post_count, view_count, like_count, image_url, created_at, last_post_at, bumped_at, extra, pm_issue) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) ON CONFLICT (discourse_id, topic_id) DO UPDATE SET discourse_id=$1, topic_id=$2, title=$3, slug=$4, post_count=$5, view_count=$6, like_count=$7, image_url=$8, created_at=$9, last_post_at=$10, bumped_at=$11, extra=$12, pm_issue=$13",
            self.discourse_id,
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

    pub async fn get_by_topic_id(
        discourse_id: &str,
        topic_id: i32,
        state: &AppState,
    ) -> Result<Self, sqlx::Error> {
        let topic = query_as!(
            Self,
            "SELECT * FROM topics WHERE discourse_id = $1 AND topic_id = $2",
            discourse_id,
            topic_id
        )
        .fetch_one(&state.database.pool)
        .await?;
        Ok(topic)
    }

    pub async fn get_first_post(&self, state: &AppState) -> Result<Post, sqlx::Error> {
        let post = query_as!(
            Post,
            "SELECT * FROM posts WHERE discourse_id = $1 AND topic_id = $2 ORDER BY post_number ASC LIMIT 1",
            self.discourse_id,
            self.topic_id
        )
        .fetch_one(&state.database.pool)
        .await?;
        Ok(post)
    }

    pub async fn get_summary_by_topic_id(
        discourse_id: &str,
        topic_id: i32,
        state: &AppState,
    ) -> Result<TopicSummary, HttpError> {
        let summary = query_as!(
            TopicSummary,
            "SELECT * FROM topic_summaries WHERE discourse_id = $1 AND topic_id = $2 ORDER BY based_on DESC LIMIT 1",
            discourse_id,
            topic_id
        )
        .fetch_optional(&state.database.pool)
        .await?;

        let topic = match Topic::get_by_topic_id(discourse_id, topic_id, state).await {
            Ok(topic) => topic,
            Err(_) => {
                return Err(sqlx::Error::RowNotFound)?;
            }
        };

        let summary = match summary {
            Some(s) => s,
            None => {
                return Self::create_new_summary(discourse_id, topic_id, state, &topic).await;
            }
        };

        let based_on = topic
            .last_post_at
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());

        // Check if the existing summary is still current
        if summary.based_on.timestamp() == based_on as i64 {
            return Ok(summary);
        }

        // Check if there's already an ongoing streaming generation for this topic
        if let Some(_ongoing_prompt) = state
            .workshop
            .get_ongoing_summary_prompt(discourse_id, topic_id)
            .await
        {
            // There's already a streaming generation in progress, return the old summary for now
            // The client should check for streaming updates
            return Ok(summary);
        }

        Self::create_new_summary(discourse_id, topic_id, state, &topic).await
    }

    async fn create_new_summary(
        discourse_id: &str,
        topic_id: i32,
        state: &AppState,
        topic: &Topic,
    ) -> Result<TopicSummary, HttpError> {
        info!(
            "Generating new summary for topic {} on {}",
            topic_id, discourse_id
        );

        // Check if there's already an ongoing streaming generation
        if let Some(ongoing_prompt) = state
            .workshop
            .get_ongoing_summary_prompt(discourse_id, topic_id)
            .await
        {
            // Wait for the ongoing prompt to complete
            match ongoing_prompt.await_completion().await {
                Ok(summary_text) => {
                    // The summary should already be saved by the background task, but let's check
                    if let Ok(existing_summary) = query_as!(
                        TopicSummary,
                        "SELECT * FROM topic_summaries WHERE topic_id = $1 ORDER BY based_on DESC LIMIT 1",
                        topic_id
                    ).fetch_optional(&state.database.pool).await {
                        if let Some(summary) = existing_summary {
                            return Ok(summary);
                        }
                    }

                    // Fallback: save the summary ourselves if not already saved
                    let based_on = topic
                        .last_post_at
                        .map(|dt| dt.timestamp())
                        .unwrap_or_else(|| Utc::now().timestamp());

                    let based_on_datetime =
                        DateTime::from_timestamp(based_on as i64, 0).unwrap_or_else(|| Utc::now());

                    let summary = query_as!(
                        TopicSummary,
                        "INSERT INTO topic_summaries (discourse_id, topic_id, based_on, summary_text, created_at) VALUES ($1, $2, $3, $4, NOW()) RETURNING *",
                        discourse_id,
                        topic_id,
                        based_on_datetime,
                        summary_text
                    )
                    .fetch_one(&state.database.pool)
                    .await?;

                    return Ok(summary);
                }
                Err(e) => {
                    info!(
                        "Ongoing prompt failed, falling back to direct generation: {}",
                        e
                    );
                    // Fall through to direct generation
                }
            }
        }

        // No ongoing prompt or it failed, use direct generation (non-streaming)
        let summary =
            crate::modules::workshop::WorkshopService::create_workshop_summary(topic, &state)
                .await?;

        let based_on = topic
            .last_post_at
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());

        let based_on_datetime =
            DateTime::from_timestamp(based_on as i64, 0).unwrap_or_else(|| Utc::now());

        let summary = query_as!(
            TopicSummary,
            "INSERT INTO topic_summaries (discourse_id, topic_id, based_on, summary_text, created_at) VALUES ($1, $2, $3, $4, NOW()) RETURNING *",
            discourse_id,
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
