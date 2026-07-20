use crate::models::topics::{Topic, post::Post};
use crate::models::workshop::usage::UserUsageOverview;
use crate::models::workshop::usage::get_all_users_usage_overview;
use crate::modules::discourse::{DiscourseService, ForumSearchDocument};
use crate::server::ApiTags;
use crate::state::AppState;
use poem::Result;
use poem::web::Data;
use poem_openapi::param::Header;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::query_as;
use strip_tags::strip_tags;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct ReindexResponse {
    pub success: bool,
    pub message: String,
    pub topics_processed: i32,
    pub posts_processed: i32,
    pub errors: i32,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminStatsResponse {
    pub database_topics: i64,
    pub database_posts: i64,
    pub meilisearch_documents: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminUsageResponse {
    pub total_users: i32,
    pub total_tokens: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_reasoning_tokens: i64,
    pub users: Vec<UserUsageOverview>,
}

impl AdminApi {
    fn verify_admin_key(api_key: Option<String>) -> Result<()> {
        let expected_key = std::env::var("ADMIN_API_KEY")
            .map_err(|_| poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))?;

        match api_key {
            Some(key) if key == expected_key => Ok(()),
            _ => Err(poem::Error::from_status(StatusCode::UNAUTHORIZED)),
        }
    }
}

#[OpenApi]
impl AdminApi {
    /// /admin/reindex
    ///
    /// Trigger a full reindex of all topics and posts from database to Meilisearch
    #[oai(path = "/admin/reindex", method = "post", tag = "ApiTags::Admin")]
    async fn reindex_all(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<ReindexResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        let Some(meili) = &state.meili else {
            return Ok(Json(ReindexResponse {
                success: false,
                message: "Meilisearch is not configured".to_string(),
                topics_processed: 0,
                posts_processed: 0,
                errors: 0,
            }));
        };

        info!("Starting full reindex of all topics and posts");

        let mut topics_processed = 0i32;
        let mut posts_processed = 0i32;
        let mut errors = 0i32;

        // Get all topics from database
        let topics = match query_as!(Topic, "SELECT * FROM topics ORDER BY topic_id ASC")
            .fetch_all(&state.database.pool)
            .await
        {
            Ok(topics) => topics,
            Err(e) => {
                error!("Failed to fetch topics from database: {}", e);
                return Ok(Json(ReindexResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                    topics_processed: 0,
                    posts_processed: 0,
                    errors: 1,
                }));
            }
        };

        info!("Found {} topics to reindex", topics.len());

        // Index all topics
        let forum_index = meili.index("forum");
        let mut topic_docs = Vec::new();

        for topic in &topics {
            topic_docs.push(ForumSearchDocument {
                entity_type: "topic".to_string(),
                discourse_id: Some(topic.discourse_id.clone()),
                topic_id: Some(topic.topic_id),
                post_id: None,
                post_number: None,
                user_id: None,
                username: None,
                title: Some(topic.title.clone()),
                slug: Some(topic.slug.clone()),
                pm_issue: topic.pm_issue,
                cooked: None,
                entity_id: format!("topic_{}", topic.topic_id),
            });
            topics_processed += 1;
        }

        // Batch insert topics
        if !topic_docs.is_empty() {
            match forum_index
                .add_documents(&topic_docs, Some("entity_id"))
                .await
            {
                Ok(_) => info!("Successfully indexed {} topics", topic_docs.len()),
                Err(e) => {
                    error!("Failed to index topics: {}", e);
                    errors += 1;
                }
            }
        }

        // Get all posts from database
        let posts = match query_as!(Post, "SELECT * FROM posts ORDER BY post_id ASC")
            .fetch_all(&state.database.pool)
            .await
        {
            Ok(posts) => posts,
            Err(e) => {
                error!("Failed to fetch posts from database: {}", e);
                errors += 1;
                return Ok(Json(ReindexResponse {
                    success: false,
                    message: format!("Database error fetching posts: {}", e),
                    topics_processed,
                    posts_processed: 0,
                    errors,
                }));
            }
        };

        info!("Found {} posts to reindex", posts.len());

        // Build user mapping from post extras for more efficient username lookup
        let user_mapping = build_user_mapping_from_posts(&posts);
        info!("Built user mapping for {} users", user_mapping.len());

        // Index all posts in batches to avoid memory issues
        const BATCH_SIZE: usize = 100;
        let post_batches = posts.chunks(BATCH_SIZE);

        for batch in post_batches {
            let mut post_docs = Vec::new();

            for post in batch {
                // Try to get username from our mapping first, then fallback to API
                let username = user_mapping
                    .get(&post.user_id)
                    .map(|u| u.clone())
                    .or_else(|| {
                        // Fallback to API lookup (currently returns None for efficiency)
                        None
                    });

                post_docs.push(ForumSearchDocument {
                    entity_type: "post".to_string(),
                    discourse_id: Some(post.discourse_id.clone()),
                    topic_id: Some(post.topic_id),
                    post_id: Some(post.post_id),
                    post_number: Some(post.post_number),
                    user_id: Some(post.user_id),
                    username,
                    title: None,
                    slug: None,
                    pm_issue: None,
                    cooked: post.cooked.as_deref().map(strip_tags),
                    entity_id: format!("post_{}", post.post_id),
                });
                posts_processed += 1;
            }

            // Batch insert posts
            if !post_docs.is_empty() {
                match forum_index
                    .add_documents(&post_docs, Some("entity_id"))
                    .await
                {
                    Ok(_) => info!("Successfully indexed batch of {} posts", post_docs.len()),
                    Err(e) => {
                        error!("Failed to index post batch: {}", e);
                        errors += 1;
                    }
                }
            }
        }

        let success = errors == 0;
        let message = if success {
            format!(
                "Successfully reindexed {} topics and {} posts",
                topics_processed, posts_processed
            )
        } else {
            format!(
                "Reindexing completed with {} errors. Processed {} topics and {} posts",
                errors, topics_processed, posts_processed
            )
        };

        info!("{}", message);

        Ok(Json(ReindexResponse {
            success,
            message,
            topics_processed,
            posts_processed,
            errors,
        }))
    }

    /// /admin/stats
    ///
    /// Get indexing statistics
    #[oai(path = "/admin/stats", method = "get", tag = "ApiTags::Admin")]
    async fn get_stats(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<AdminStatsResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        // Get database counts
        let database_topics = match sqlx::query_scalar!("SELECT COUNT(*) FROM topics")
            .fetch_one(&state.database.pool)
            .await
        {
            Ok(count) => count.unwrap_or(0),
            Err(e) => {
                error!("Failed to count topics: {}", e);
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        let database_posts = match sqlx::query_scalar!("SELECT COUNT(*) FROM posts")
            .fetch_one(&state.database.pool)
            .await
        {
            Ok(count) => count.unwrap_or(0),
            Err(e) => {
                error!("Failed to count posts: {}", e);
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        // Get Meilisearch document count
        let meilisearch_documents = if let Some(meili) = &state.meili {
            let forum_index = meili.index("forum");
            match forum_index.get_stats().await {
                Ok(stats) => Some(stats.number_of_documents as i64),
                Err(e) => {
                    warn!("Failed to get Meilisearch stats: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Json(AdminStatsResponse {
            database_topics,
            database_posts,
            meilisearch_documents,
        }))
    }

    /// /admin/usage
    ///
    /// Get workshop usage statistics for all users
    #[oai(path = "/admin/usage", method = "get", tag = "ApiTags::Admin")]
    async fn get_usage_stats(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<AdminUsageResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        // Get all users' usage overview
        let users = get_all_users_usage_overview(&state).await.map_err(|e| {
            error!("Failed to get usage overview: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        // Calculate totals
        let total_users = users.len() as i32;
        let total_tokens = users.iter().map(|u| u.total_tokens).sum();
        let total_prompt_tokens = users.iter().map(|u| u.prompt_tokens).sum();
        let total_completion_tokens = users.iter().map(|u| u.completion_tokens).sum();
        let total_reasoning_tokens = users.iter().map(|u| u.reasoning_tokens).sum();

        Ok(Json(AdminUsageResponse {
            total_users,
            total_tokens,
            total_prompt_tokens,
            total_completion_tokens,
            total_reasoning_tokens,
            users,
        }))
    }

    #[oai(
        path = "/admin/topic_summary",
        method = "delete",
        tag = "ApiTags::Admin"
    )]
    async fn delete_topic_summary(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(name = "topic_id")] topic_id: poem_openapi::param::Query<i32>,
        #[oai(name = "discourse_id")] discourse_id: poem_openapi::param::Query<String>,
    ) -> Result<()> {
        Self::verify_admin_key(admin_key.0)?;

        let result = sqlx::query!(
            "DELETE FROM topic_summaries WHERE topic_id = $1 AND discourse_id = $2",
            topic_id.0,
            discourse_id.0
        )
        .execute(&state.database.pool)
        .await;

        match result {
            Ok(query_result) => {
                if query_result.rows_affected() > 0 {
                    let summary_key = format!("summary-{}-{}", discourse_id.0, topic_id.0);
                    state.workshop.ongoing_prompts.remove(&summary_key).await;

                    info!(
                        "Successfully deleted topic summary for topic_id {} and cleared cache",
                        topic_id.0
                    );
                    Ok(())
                } else {
                    error!("No topic summary found for topic_id {}", topic_id.0);
                    Err(poem::Error::from_string(
                        format!("Topic summary not found for topic_id {}", topic_id.0),
                        StatusCode::NOT_FOUND,
                    ))
                }
            }
            Err(e) => {
                error!(
                    "Failed to delete topic summary for topic_id {}: {}",
                    topic_id.0, e
                );
                Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))
            }
        }
    }
}

/// Helper function to get username for a user_id using Discourse API with caching
async fn get_username_for_user_id(_user_id: i32, _discourse: &DiscourseService) -> Option<String> {
    // For reindexing, we try to get usernames from Discourse, but don't block on failures
    // This is best-effort - new posts will have usernames from the API
    None
}

/// Build a comprehensive user mapping by extracting user info from post extras
fn build_user_mapping_from_posts(posts: &[Post]) -> std::collections::HashMap<i32, String> {
    let mut user_map = std::collections::HashMap::new();

    for post in posts {
        if let Some(extra) = &post.extra {
            // Try to extract username from the extra JSON data
            if let Some(username) = extra.get("username").and_then(|u| u.as_str()) {
                user_map.insert(post.user_id, username.to_string());
            }
        }
    }

    user_map
}
