use futures::StreamExt;
use futures::stream::BoxStream;
use poem::{Result, web::Data};
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::EventStream;
use poem_openapi::{Enum, Object, OpenApi, payload::Json};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::models::digest::ActivityDigest;
use crate::models::topics::{Topic, TopicSummary, post::Post};
use crate::modules::llm::streams::{StreamEvent, ToolCallUpdate};
use crate::modules::llm::summary::{self, SummaryError};
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TopicApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct PostsResponse {
    pub posts: Vec<Post>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Enum)]
#[oai(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SummaryStartStatus {
    Existing,
    Started,
    Ongoing,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct SummaryStartResponse {
    pub status: SummaryStartStatus,
    pub topic_id: i32,
    pub discourse_id: String,
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct StreamingResponse {
    pub content: String,
    pub is_complete: bool,
    /// Discard all previously received content; generation restarted.
    pub is_reset: bool,
    pub error: Option<String>,
    pub tool_activity: Option<String>,
    pub tool_call: Option<ToolCallUpdate>,
}

impl StreamingResponse {
    fn empty() -> Self {
        Self {
            content: String::new(),
            is_complete: false,
            is_reset: false,
            error: None,
            tool_activity: None,
            tool_call: None,
        }
    }
}

fn summary_error_status(error: &SummaryError) -> StatusCode {
    match error {
        SummaryError::TopicNotFound => StatusCode::NOT_FOUND,
        SummaryError::Unconfigured => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[OpenApi]
impl TopicApi {
    /// /topics
    ///
    /// List topics by latest activity
    #[oai(path = "/topics", method = "get", tag = "ApiTags::Topic")]
    async fn list(&self, state: Data<&AppState>) -> Result<Json<Vec<Topic>>> {
        let topics = Topic::get_by_latest_post_at(&state).await.map_err(|e| {
            tracing::error!("Error getting topics: {:?}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(topics))
    }

    /// /topics/trending
    ///
    /// List trending topics
    #[oai(path = "/topics/trending", method = "get", tag = "ApiTags::Topic")]
    async fn trending(&self, state: Data<&AppState>) -> Result<Json<Vec<Topic>>> {
        let topics = Topic::get_by_trending(&state).await.map_err(|e| {
            tracing::error!("Error getting trending topics: {:?}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(topics))
    }

    /// /t/:discourse_id/:topic_id
    ///
    /// Get information about a topic
    #[oai(
        path = "/t/:discourse_id/:topic_id",
        method = "get",
        operation_id = "get_topic",
        tag = "ApiTags::Topic"
    )]
    async fn get_topic(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<Topic>> {
        let discourse_id = discourse_id.0;
        let topic = Topic::get_by_topic_id(&discourse_id, topic_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(topic))
    }

    /// /t/:discourse_id/:topic_id
    ///
    /// Force refresh a topic
    #[oai(
        path = "/t/:discourse_id/:topic_id",
        method = "post",
        operation_id = "refresh_topic",
        tag = "ApiTags::Topic"
    )]
    async fn refresh_topic(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<serde_json::Value>> {
        info!("Refreshing topic: {} on {}", topic_id.0, discourse_id.0);
        state.discourse.enqueue(&discourse_id, topic_id.0, 1).await;

        Ok(Json(serde_json::json!({})))
    }

    /// /t/:discourse_id/:topic_id/posts
    ///
    /// Get all posts for a topic
    /// This endpoint is paginated, and uses ?page=1 as the first page
    #[oai(
        path = "/t/:discourse_id/:topic_id/posts",
        method = "get",
        operation_id = "get_posts",
        tag = "ApiTags::Topic"
    )]
    async fn get_posts(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
        #[oai(style = "simple")] page: Query<i32>,
        #[oai(style = "simple")] size: Query<Option<i32>>,
    ) -> Result<Json<PostsResponse>> {
        let discourse_id = discourse_id.0;
        let topic_id = topic_id.0;
        let page = page.0;

        let (posts, has_more) =
            Post::find_by_topic_id(&discourse_id, topic_id, page, size.0, &state)
                .await
                .map_err(|e| {
                    tracing::error!("Error finding posts: {:?}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

        Ok(Json(PostsResponse { posts, has_more }))
    }

    /// /t/:discourse_id/:topic_id/summary
    ///
    /// Get summaries from topic
    #[oai(
        path = "/t/:discourse_id/:topic_id/summary",
        method = "get",
        operation_id = "get_summary",
        tag = "ApiTags::Topic"
    )]
    async fn get_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<TopicSummary>> {
        let topic_id = topic_id.0;

        let summary = summary::get_or_generate_summary(&discourse_id, topic_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic summary: {:?}", e);
                poem::Error::from_status(summary_error_status(&e))
            })?;

        Ok(Json(summary))
    }

    /// /t/:discourse_id/:topic_id/summary/cached
    ///
    /// Latest cached summary, read-only: never triggers generation, 404 when none exists
    #[oai(
        path = "/t/:discourse_id/:topic_id/summary/cached",
        method = "get",
        operation_id = "get_cached_summary",
        tag = "ApiTags::Topic"
    )]
    async fn get_cached_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<TopicSummary>> {
        let topic = Topic::get_by_topic_id(&discourse_id, topic_id.0, &state)
            .await
            .map_err(|_| poem::Error::from_status(StatusCode::NOT_FOUND))?;

        summary::latest_cached_summary(&topic, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error reading cached summary: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?
            .map(Json)
            .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))
    }

    /// /t/:discourse_id/:topic_id/summary/stream
    ///
    /// Start summary generation for a topic (or coalesce onto an ongoing one)
    #[oai(
        path = "/t/:discourse_id/:topic_id/summary/stream",
        method = "post",
        operation_id = "start_summary_stream",
        tag = "ApiTags::Topic"
    )]
    async fn start_summary_stream(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
        #[oai(name = "force")] force: Query<Option<bool>>,
        #[oai(name = "X-Admin-Key")] admin_key: poem_openapi::param::Header<Option<String>>,
    ) -> Result<Json<SummaryStartResponse>> {
        let discourse_id = discourse_id.0;
        let topic_id = topic_id.0;

        let topic = Topic::get_by_topic_id(&discourse_id, topic_id, &state)
            .await
            .map_err(|_| poem::Error::from_status(StatusCode::NOT_FOUND))?;

        // An in-flight run always wins over the cache so every viewer attaches
        // to the live summarizer. A finished stream in its grace period does
        // NOT count — coalescing onto it would replay old output and swallow
        // forced regenerations.
        if let Some(llm) = state.llm.as_ref()
            && let Some(stream) = llm
                .streams
                .get(&summary::summary_key(&discourse_id, topic_id))
                .await
            && !stream.is_done().await
        {
            return Ok(Json(SummaryStartResponse {
                status: SummaryStartStatus::Ongoing,
                topic_id,
                discourse_id,
                summary: None,
            }));
        }

        let force = force.0.unwrap_or(false);
        if force {
            crate::server::admin::AdminApi::verify_admin_key(admin_key.0)?;
        } else {
            let cached = summary::fresh_cached_summary(&topic, &state)
                .await
                .map_err(|e| {
                    tracing::error!("Error checking cached summary: {:?}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

            if let Some(cached) = cached {
                return Ok(Json(SummaryStartResponse {
                    status: SummaryStartStatus::Existing,
                    topic_id,
                    discourse_id,
                    summary: Some(cached.summary_text),
                }));
            }
        }

        if state.llm.is_none() {
            return Err(poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE));
        }

        let (_stream, created) = summary::start_topic_summary(&topic, &state, force)
            .await
            .map_err(|e| {
                tracing::error!("Error starting summary generation: {:?}", e);
                poem::Error::from_status(summary_error_status(&e))
            })?;

        Ok(Json(SummaryStartResponse {
            status: if created {
                SummaryStartStatus::Started
            } else {
                SummaryStartStatus::Ongoing
            },
            topic_id,
            discourse_id,
            summary: None,
        }))
    }

    /// /t/:discourse_id/:topic_id/summary/stream
    ///
    /// SSE stream of an ongoing (or just finished) summary generation
    #[oai(
        path = "/t/:discourse_id/:topic_id/summary/stream",
        method = "get",
        operation_id = "get_summary_stream",
        tag = "ApiTags::Topic"
    )]
    async fn get_summary_stream(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<EventStream<BoxStream<'static, StreamingResponse>>> {
        let key = summary::summary_key(&discourse_id, topic_id.0);
        let stream = match state.llm.as_ref() {
            Some(llm) => llm.streams.get(&key).await,
            None => None,
        }
        .ok_or_else(|| poem::Error::from_status(StatusCode::NOT_FOUND))?;

        let events = stream
            .subscribe()
            .await
            .map(|event| match event {
                StreamEvent::Chunk(content) => StreamingResponse {
                    content,
                    ..StreamingResponse::empty()
                },
                StreamEvent::ToolActivity(activity) => StreamingResponse {
                    tool_activity: Some(activity),
                    ..StreamingResponse::empty()
                },
                StreamEvent::ToolCall(update) => StreamingResponse {
                    tool_call: Some(update),
                    ..StreamingResponse::empty()
                },
                StreamEvent::Reset => StreamingResponse {
                    is_reset: true,
                    ..StreamingResponse::empty()
                },
                StreamEvent::Done(Ok(())) => StreamingResponse {
                    is_complete: true,
                    ..StreamingResponse::empty()
                },
                StreamEvent::Done(Err(error)) => StreamingResponse {
                    is_complete: true,
                    error: Some(error),
                    ..StreamingResponse::empty()
                },
            })
            .boxed();

        Ok(EventStream::new(events))
    }

    /// /digest
    ///
    /// Get the latest forum activity digest
    #[oai(
        path = "/digest",
        method = "get",
        operation_id = "get_digest",
        tag = "ApiTags::Topic"
    )]
    async fn get_digest(&self, state: Data<&AppState>) -> Result<Json<ActivityDigest>> {
        match ActivityDigest::get_latest(&state).await {
            Ok(Some(digest)) => Ok(Json(digest)),
            Ok(None) => Err(poem::Error::from_status(StatusCode::NOT_FOUND)),
            Err(e) => {
                tracing::error!("Error getting activity digest: {:?}", e);
                Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))
            }
        }
    }
}
