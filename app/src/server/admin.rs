use crate::models::digest::ActivityDigest;
use crate::models::llm::{LlmMemory, LlmMemorySnapshot, LlmMemoryStaging, LlmRun};
use crate::models::topics::{Topic, post::Post};
use crate::modules::discourse::{DiscourseService, ForumSearchDocument};
use crate::modules::llm::curator::{self, CuratorOutput};
use crate::modules::llm::digest::{self, DigestError};
use crate::modules::llm::summary;
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
use uuid::Uuid;

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
pub struct SystemPromptResponse {
    pub summary_prompt: String,
    pub digest_prompt: String,
    pub curator_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct SystemPromptUpdate {
    pub prompt_type: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmMemoryEntry {
    pub entry_id: i32,
    pub term: String,
    pub content: String,
    pub sources: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmRunResponse {
    pub run_id: Uuid,
    pub run_type: String,
    pub discourse_id: Option<String>,
    pub topic_id: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub model_used: Option<String>,
    pub tool_calls: Option<i32>,
    pub tool_rounds: Option<i32>,
    pub duration_ms: Option<i32>,
    pub outcome: String,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminMetricsResponse {
    pub total_runs: i64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub avg_total_tokens: f64,
    pub runs_by_type: serde_json::Value,
    pub runs_by_day: serde_json::Value,
    pub total_tokens_all_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct CuratorTriggerResponse {
    pub success: bool,
    pub message: String,
    pub output: Option<CuratorOutput>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmUsageDay {
    pub date: String,
    pub run_type: String,
    pub model_used: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub runs: i64,
    pub failures: i64,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmUsageResponse {
    pub days: Vec<LlmUsageDay>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmModelStats {
    pub model: String,
    pub runs: i64,
    pub avg_prompt_tokens: f64,
    pub avg_completion_tokens: f64,
    pub avg_total_tokens: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LlmModelStatsResponse {
    pub models: Vec<LlmModelStats>,
}

impl AdminApi {
    pub(crate) fn verify_admin_key(api_key: Option<String>) -> Result<()> {
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

        let user_mapping = build_user_mapping_from_posts(&posts);
        info!("Built user mapping for {} users", user_mapping.len());

        const BATCH_SIZE: usize = 100;
        let post_batches = posts.chunks(BATCH_SIZE);

        for batch in post_batches {
            let mut post_docs = Vec::new();

            for post in batch {
                let username = user_mapping
                    .get(&post.user_id)
                    .cloned()
                    .or_else(|| None);

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
    #[oai(path = "/admin/stats", method = "get", tag = "ApiTags::Admin")]
    async fn get_stats(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<AdminStatsResponse>> {
        Self::verify_admin_key(admin_key.0)?;

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

    /// /admin/digest
    #[oai(path = "/admin/digest", method = "post", tag = "ApiTags::Admin")]
    async fn generate_digest(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<ActivityDigest>> {
        Self::verify_admin_key(admin_key.0)?;

        let digest = digest::generate_and_store(&state).await.map_err(|e| {
            error!("Failed to generate activity digest: {}", e);
            match e {
                DigestError::Unconfigured => {
                    poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
                }
                _ => poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            }
        })?;

        Ok(Json(digest))
    }

    /// /admin/topic_summary
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
                    info!(
                        "Successfully deleted topic summary for topic_id {}",
                        topic_id.0
                    );
                    Ok(())
                } else {
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

    /// /admin/digest
    #[oai(path = "/admin/digest", method = "delete", tag = "ApiTags::Admin")]
    async fn delete_digest(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<()> {
        Self::verify_admin_key(admin_key.0)?;

        let deleted = crate::models::llm::LlmRun::delete_latest_digest(&state)
            .await
            .map_err(|e| {
                error!("Failed to delete digest: {}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        if deleted == 0 {
            return Err(poem::Error::from_string(
                "No activity digest to delete".to_string(),
                StatusCode::NOT_FOUND,
            ));
        }

        info!("Deleted latest activity digest");
        Ok(())
    }

    // --- New Admin Endpoints ---

    /// /admin/llm/system-prompt
    #[oai(path = "/admin/llm/system-prompt", method = "get", tag = "ApiTags::Admin")]
    async fn get_system_prompt(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<SystemPromptResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        Ok(Json(SystemPromptResponse {
            summary_prompt: summary::SUMMARY_PROMPT.to_string(),
            digest_prompt: digest::DIGEST_PROMPT.to_string(),
            curator_prompt: curator::CURATOR_PROMPT.to_string(),
        }))
    }

    /// /admin/llm/memory
    #[oai(path = "/admin/llm/memory", method = "get", tag = "ApiTags::Admin")]
    async fn get_memory(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<Vec<LlmMemory>>> {
        Self::verify_admin_key(admin_key.0)?;

        let entries = LlmMemory::get_all(&state).await.map_err(|e| {
            error!("Failed to fetch memory: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(entries))
    }

    /// /admin/llm/memory
    #[oai(path = "/admin/llm/memory", method = "post", tag = "ApiTags::Admin")]
    async fn upsert_memory(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        body: Json<LlmMemoryEntry>,
    ) -> Result<Json<LlmMemory>> {
        Self::verify_admin_key(admin_key.0)?;

        let entry = LlmMemory::upsert(
            &body.term,
            &body.content,
            &body.sources.clone().unwrap_or(serde_json::json!([])),
            &state,
        )
        .await
        .map_err(|e| {
            error!("Failed to upsert memory: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(entry))
    }

    /// /admin/llm/memory/:entry_id
    #[oai(
        path = "/admin/llm/memory/:entry_id",
        method = "delete",
        tag = "ApiTags::Admin"
    )]
    async fn delete_memory(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(style = "simple")] entry_id: poem_openapi::param::Path<i32>,
    ) -> Result<()> {
        Self::verify_admin_key(admin_key.0)?;

        let deleted = LlmMemory::delete(entry_id.0, &state).await.map_err(|e| {
            error!("Failed to delete memory: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        if !deleted {
            return Err(poem::Error::from_string(
                format!("Memory entry {} not found", entry_id.0),
                StatusCode::NOT_FOUND,
            ));
        }

        Ok(())
    }

    /// /admin/llm/memory/staging
    #[oai(path = "/admin/llm/memory/staging", method = "get", tag = "ApiTags::Admin")]
    async fn get_staging(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(name = "limit")] limit: poem_openapi::param::Query<Option<i64>>,
    ) -> Result<Json<Vec<LlmMemoryStaging>>> {
        Self::verify_admin_key(admin_key.0)?;

        let limit = limit.0.unwrap_or(100).min(500);
        let entries = LlmMemoryStaging::recent(limit, &state).await.map_err(|e| {
            error!("Failed to fetch staged candidates: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(entries))
    }


    #[oai(path = "/admin/llm/runs", method = "get", tag = "ApiTags::Admin")]
    async fn get_runs(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(name = "run_type")] run_type: poem_openapi::param::Query<Option<String>>,
        #[oai(name = "limit")] limit: poem_openapi::param::Query<Option<i64>>,
        #[oai(name = "offset")] offset: poem_openapi::param::Query<Option<i64>>,
    ) -> Result<Json<Vec<LlmRun>>> {
        Self::verify_admin_key(admin_key.0)?;

        let limit = limit.0.unwrap_or(50).min(200);
        let offset = offset.0.unwrap_or(0);

        let runs = if let Some(rtype) = &run_type.0 {
            sqlx::query_as!(
                LlmRun,
                "SELECT * FROM llm_runs WHERE run_type = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                rtype,
                limit,
                offset
            )
            .fetch_all(&state.database.pool)
            .await
        } else {
            sqlx::query_as!(
                LlmRun,
                "SELECT * FROM llm_runs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                limit,
                offset
            )
            .fetch_all(&state.database.pool)
            .await
        }
        .map_err(|e| {
            error!("Failed to fetch runs: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(runs))
    }

    /// /admin/llm/runs/:run_id
    #[oai(
        path = "/admin/llm/runs/:run_id",
        method = "get",
        tag = "ApiTags::Admin"
    )]
    async fn get_run(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(style = "simple")] run_id: poem_openapi::param::Path<Uuid>,
    ) -> Result<Json<LlmRun>> {
        Self::verify_admin_key(admin_key.0)?;

        let run = sqlx::query_as!(
            LlmRun,
            "SELECT * FROM llm_runs WHERE run_id = $1",
            run_id.0
        )
        .fetch_optional(&state.database.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch run: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))?;

        Ok(Json(run))
    }

    /// /admin/llm/metrics
    #[oai(path = "/admin/llm/metrics", method = "get", tag = "ApiTags::Admin")]
    async fn get_metrics(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<AdminMetricsResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        let pool = &state.database.pool;

        let total_runs: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM llm_runs")
            .fetch_one(pool)
            .await
            .unwrap_or(Some(0))
            .unwrap_or(0);

        let success_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM llm_runs WHERE outcome = 'success'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        let avg_duration: f64 = sqlx::query_scalar!(
            "SELECT COALESCE(AVG(duration_ms::float8), 0.0) FROM llm_runs WHERE duration_ms IS NOT NULL"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0.0))
        .unwrap_or(0.0);

        let avg_tokens: f64 = sqlx::query_scalar!(
            "SELECT COALESCE(AVG(total_tokens::float8), 0.0) FROM llm_runs WHERE total_tokens IS NOT NULL"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0.0))
        .unwrap_or(0.0);

        let total_tokens_all_time: i64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(total_tokens), 0) FROM llm_runs"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        let success_rate = if total_runs > 0 {
            success_count as f64 / total_runs as f64 * 100.0
        } else {
            0.0
        };

        Ok(Json(AdminMetricsResponse {
            total_runs,
            success_rate,
            avg_duration_ms: avg_duration,
            avg_total_tokens: avg_tokens,
            runs_by_type: serde_json::Value::Null,
            runs_by_day: serde_json::Value::Null,
            total_tokens_all_time,
        }))
    }

    /// /admin/llm/usage
    #[oai(path = "/admin/llm/usage", method = "get", tag = "ApiTags::Admin")]
    async fn get_usage(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(name = "days")] days: poem_openapi::param::Query<Option<i32>>,
    ) -> Result<Json<LlmUsageResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        let days = days.0.unwrap_or(30).clamp(1, 180);

        let rows = sqlx::query!(
            r#"SELECT to_char(date_trunc('day', created_at), 'YYYY-MM-DD') AS "date!",
                      run_type,
                      COALESCE(model_used, 'unknown') AS "model_used!",
                      COALESCE(SUM(prompt_tokens), 0)::bigint AS "prompt_tokens!",
                      COALESCE(SUM(completion_tokens), 0)::bigint AS "completion_tokens!",
                      COALESCE(SUM(total_tokens), 0)::bigint AS "total_tokens!",
                      COUNT(*)::bigint AS "runs!",
                      (COUNT(*) FILTER (WHERE outcome != 'success'))::bigint AS "failures!"
               FROM llm_runs
               WHERE created_at >= NOW() - make_interval(days => $1)
               GROUP BY 1, 2, 3
               ORDER BY 1 ASC, 2 ASC, 3 ASC"#,
            days
        )
        .fetch_all(&state.database.pool)
        .await
        .map_err(|e| {
            error!("Failed to aggregate llm usage: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(LlmUsageResponse {
            days: rows
                .into_iter()
                .map(|row| LlmUsageDay {
                    date: row.date,
                    run_type: row.run_type,
                    model_used: row.model_used,
                    prompt_tokens: row.prompt_tokens,
                    completion_tokens: row.completion_tokens,
                    total_tokens: row.total_tokens,
                    runs: row.runs,
                    failures: row.failures,
                })
                .collect(),
        }))
    }

    /// /admin/llm/per-model
    #[oai(path = "/admin/llm/per-model", method = "get", tag = "ApiTags::Admin")]
    async fn get_per_model_stats(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
        #[oai(name = "days")] days: poem_openapi::param::Query<Option<i32>>,
    ) -> Result<Json<LlmModelStatsResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        let days = days.0.unwrap_or(30).clamp(1, 365);

        let rows = sqlx::query!(
            r#"SELECT COALESCE(model_used, 'unknown') AS "model!",
                      COUNT(*)::bigint AS "runs!",
                      COALESCE(AVG(prompt_tokens::float8), 0.0) AS "avg_prompt_tokens!",
                      COALESCE(AVG(completion_tokens::float8), 0.0) AS "avg_completion_tokens!",
                      COALESCE(AVG(total_tokens::float8), 0.0) AS "avg_total_tokens!",
                      COALESCE(SUM(total_tokens::bigint), 0)::bigint AS "total_tokens!"
               FROM llm_runs
               WHERE created_at >= NOW() - make_interval(days => $1)
                 AND outcome != 'running'
               GROUP BY 1
               ORDER BY COALESCE(SUM(total_tokens), 0) DESC"#,
            days
        )
        .fetch_all(&state.database.pool)
        .await
        .map_err(|e| {
            error!("Failed to aggregate per-model stats: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(LlmModelStatsResponse {
            models: rows
                .into_iter()
                .map(|row| LlmModelStats {
                    model: row.model,
                    runs: row.runs,
                    avg_prompt_tokens: row.avg_prompt_tokens,
                    avg_completion_tokens: row.avg_completion_tokens,
                    avg_total_tokens: row.avg_total_tokens,
                    total_tokens: row.total_tokens,
                })
                .collect(),
        }))
    }

    /// /admin/llm/snapshots
    #[oai(path = "/admin/llm/snapshots", method = "get", tag = "ApiTags::Admin")]
    async fn get_snapshots(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<Vec<LlmMemorySnapshot>>> {
        Self::verify_admin_key(admin_key.0)?;

        let snapshots = sqlx::query_as!(
            LlmMemorySnapshot,
            "SELECT * FROM llm_memory_snapshots ORDER BY version DESC LIMIT 10"
        )
        .fetch_all(&state.database.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch snapshots: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(snapshots))
    }

    /// /admin/llm/curator/trigger
    #[oai(path = "/admin/llm/curator/trigger", method = "post", tag = "ApiTags::Admin")]
    async fn trigger_curator(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<CuratorTriggerResponse>> {
        Self::verify_admin_key(admin_key.0)?;

        if state.llm.is_none() {
            return Ok(Json(CuratorTriggerResponse {
                success: false,
                message: "LLM is not configured".to_string(),
                output: None,
            }));
        }

        match curator::run_curator(&state).await {
            Ok(output) => Ok(Json(CuratorTriggerResponse {
                success: true,
                message: format!(
                    "Curator run complete: {} memory updates",
                    output.memory_updates.len(),
                ),
                output: Some(output),
            })),
            Err(e) => Ok(Json(CuratorTriggerResponse {
                success: false,
                message: format!("Curator run failed: {e}"),
                output: None,
            })),
        }
    }

    /// /admin/llm/curator/last
    #[oai(path = "/admin/llm/curator/last", method = "get", tag = "ApiTags::Admin")]
    async fn get_last_curator_run(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-Admin-Key")] admin_key: Header<Option<String>>,
    ) -> Result<Json<Option<LlmRun>>> {
        Self::verify_admin_key(admin_key.0)?;

        let run = sqlx::query_as!(
            LlmRun,
            "SELECT * FROM llm_runs WHERE run_type = 'curator' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&state.database.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch last curator run: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(run))
    }
}

fn build_user_mapping_from_posts(posts: &[Post]) -> std::collections::HashMap<i32, String> {
    let mut user_map = std::collections::HashMap::new();

    for post in posts {
        if let Some(extra) = &post.extra {
            if let Some(username) = extra.get("username").and_then(|u| u.as_str()) {
                user_map.insert(post.user_id, username.to_string());
            }
        }
    }

    user_map
}
