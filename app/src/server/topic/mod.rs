use poem::{Result, web::Data};
use poem_openapi::param::{Path, Query};
use poem_openapi::{Object, OpenApi, payload::Json};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::models::topics::{Post, Summary, Topic};
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TopicApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct PostsResponse {
    pub posts: Vec<Post>,
    pub has_more: bool,
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

    /// /t/:topic_id
    ///
    /// Get information about a topic
    #[oai(
        path = "/t/:topic_id",
        method = "get",
        operation_id = "get_topic",
        tag = "ApiTags::Topic"
    )]
    async fn get_topic(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<Topic>> {
        let topic = Topic::get_by_topic_id(topic_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(topic))
    }

    /// /t/:topic_id
    ///
    /// Force refresh a topic
    #[oai(
        path = "/t/:topic_id",
        method = "post",
        operation_id = "refresh_topic",
        tag = "ApiTags::Topic"
    )]
    async fn refresh_topic(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<serde_json::Value>> {
        info!("Refreshing topic: {:?}", topic_id.0);
        state.discourse.enqueue(topic_id.0, 1).await;

        Ok(Json(serde_json::json!({})))
    }

    /// /t/:topic_id/posts
    ///
    /// Get all posts for a topic
    /// This endpoint is paginated, and uses ?page=1 as the first page
    #[oai(
        path = "/t/:topic_id/posts",
        method = "get",
        operation_id = "get_posts",
        tag = "ApiTags::Topic"
    )]
    async fn get_posts(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
        #[oai(style = "simple")] page: Query<i32>,
        #[oai(style = "simple")] size: Query<Option<i32>>,
    ) -> Result<Json<PostsResponse>> {
        let topic_id = topic_id.0;
        let page = page.0;

        let (posts, has_more) = Post::find_by_topic_id(topic_id, page, size.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error finding posts: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(PostsResponse { posts, has_more }))
    }

    /// /t/:topic_id/summary
    ///
    /// Get summaries from topic
    #[oai(
        path = "/t/:topic_id/summary",
        method = "get",
        operation_id = "get_summary",
        tag = "ApiTags::Topic"
    )]
    async fn get_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<Summary>> {
        let topic_id = topic_id.0;

        let summary = Topic::get_summary_by_topic_id(topic_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic summary: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        println!("Summary for topic {}: {:?}", topic_id, summary);

        Ok(Json(summary))
    }
}
