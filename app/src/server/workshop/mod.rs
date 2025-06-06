use crate::models::topics::Topic;
use crate::models::workshop::{WorkshopChat, WorkshopMessage};
use crate::modules::workshop::prompts::SUMMARY_MODEL;
use crate::modules::workshop::WorkshopService;
use crate::server::ApiTags;
use crate::server::auth::AuthUser;
use crate::state::AppState;
use crate::metrics;
use async_std::task;
use futures::{StreamExt, stream::BoxStream};
use poem::Request;
use poem::Result;
use poem::web::Data;
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::{EventStream, Json};
use poem_openapi::{Object, OpenApi};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopChatInput {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopChatPayload {
    pub chat_id: Uuid,
    pub chat: WorkshopChat,
    pub messages: Vec<WorkshopMessage>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct StreamingResponse {
    pub content: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

#[OpenApi]
impl WorkshopApi {
    /// /ws/t/:topic_id/summary/to-chat
    ///
    /// Create a new chat from a topic summary
    #[oai(
        path = "/ws/t/:topic_id/summary/to-chat",
        method = "post",
        tag = "ApiTags::Workshop"
    )]
    async fn create_chat_from_summary(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<WorkshopMessage>> {
        let user_id = auth_user.0.user.user_id;
        let user_prompt = format!("Summarize ethereum.forum topic #{}", topic_id.0);

        let message =
            WorkshopMessage::create_user_message(None, None, user_id, user_prompt, &state)
                .await
                .map_err(|e| {
                    tracing::error!("Error creating message: {:?}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

        let summary = Topic::get_summary_by_topic_id(topic_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic summary: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        let message2 = WorkshopMessage::create_system_response(
            &message.chat_id,
            Some(message.message_id),
            summary.summary_text,
            &state,
        )
        .await
        .map_err(|e| {
            tracing::error!("Error creating message: {:?}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        Ok(Json(message2))
    }

    /// /ws/chat
    ///
    /// Get all chats
    #[oai(path = "/ws/chat", method = "get", tag = "ApiTags::Workshop")]
    async fn get_chats(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
    ) -> Result<Json<Vec<WorkshopChat>>> {
        let user_id = auth_user.0.user.user_id;
        let chats = WorkshopChat::find_by_user_id(user_id, &state.0)
            .await
            .map_err(|e| {
                tracing::error!("Error finding chats: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(chats))
    }

    /// /ws/chat/:chat_id
    ///
    /// Get a chat
    #[oai(path = "/ws/chat/:chat_id", method = "get", tag = "ApiTags::Workshop")]
    async fn get_chat(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        #[oai(style = "simple")] chat_id: Path<Uuid>,
    ) -> Result<Json<WorkshopChatPayload>> {
        let user_id = auth_user.0.user.user_id;

        // First verify that the chat belongs to the authenticated user
        let chat = WorkshopChat::find_by_id(*chat_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error finding chat: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Check if the chat belongs to the authenticated user
        if chat.user_id != user_id {
            tracing::warn!(
                "User {} attempted to access chat {} owned by {}",
                user_id,
                *chat_id,
                chat.user_id
            );
            return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
        }

        let messages = WorkshopMessage::get_messages_by_chat_id(&chat_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error finding messages: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(WorkshopChatPayload {
            chat_id: *chat_id,
            chat,
            messages,
        }))
    }

    /// /ws/chat/:chat_id
    ///
    /// Send a message
    /// Specify parent_message as query param to send a reply
    #[oai(path = "/ws/chat/:chat_id", method = "post", tag = "ApiTags::Workshop")]
    async fn send_message(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        payload: Json<WorkshopChatInput>,
        #[oai(style = "simple")] chat_id: Path<String>,
        #[oai(style = "simple")] parent_message: Query<Option<Uuid>>,
    ) -> Result<Json<WorkshopMessage>> {
        let user_id = auth_user.0.user.user_id;
        let message = payload.message.clone();

        let chat_id = if chat_id.eq("new") {
            None
        } else {
            let parsed_chat_id = Uuid::parse_str(&chat_id).map_err(|e| {
                tracing::error!("Error parsing chat id: {:?}", e);
                poem::Error::from_status(StatusCode::BAD_REQUEST)
            })?;

            // If chat_id is provided, verify that it belongs to the authenticated user
            let chat = WorkshopChat::find_by_id(parsed_chat_id, &state)
                .await
                .map_err(|e| {
                    tracing::error!("Error finding chat: {:?}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

            if chat.user_id != user_id {
                tracing::warn!(
                    "User {} attempted to send message to chat {} owned by {}",
                    user_id,
                    parsed_chat_id,
                    chat.user_id
                );
                return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
            }

            Some(parsed_chat_id)
        };

        let message = WorkshopMessage::create_user_message(
            chat_id,
            *parent_message,
            user_id,
            message.to_string(),
            &state,
        )
        .await
        .map_err(|e| {
            tracing::error!("Error sending message: {:?}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        WorkshopChat::update_last_message(&message.chat_id, &message.message_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error updating chat: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Start processing the next message (this will create an OngoingPrompt)
        let (_ongoing_prompt, created_message) =
            WorkshopService::process_next_message(message.chat_id, message.message_id, &state)
                .await
                .map_err(|e| {
                    tracing::error!("Error processing next message: {:?}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

        // Return the system response message that's being generated
        // Add debug logging to help with troubleshooting
        tracing::info!(
            "Created system message with ID: {} for chat: {}",
            created_message.message_id,
            created_message.chat_id
        );

        Ok(Json(created_message))
    }

    /// /ws/chat/:chat_id/:message_id/stream
    ///
    /// Get SSE stream for message generation
    #[oai(
        path = "/ws/chat/:chat_id/:message_id/stream",
        method = "get",
        tag = "ApiTags::Workshop"
    )]
    async fn stream_message_response(
        &self,
        req: &Request,
        state: Data<&AppState>,
        #[oai(style = "simple")] chat_id: Path<Uuid>,
        #[oai(style = "simple")] message_id: Path<Uuid>,
        #[oai(style = "simple")] token: Query<Option<String>>,
    ) -> Result<EventStream<BoxStream<'static, StreamingResponse>>> {
        // Handle authentication - either via SecurityScheme or query parameter
        let authenticated_user = if let Some(token_str) = token.0 {
            // Manual token validation for EventSource compatibility
            let sso_service = state.sso.as_ref().ok_or_else(|| {
                tracing::error!("SSO service not configured");
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

            let claims = sso_service.validate_jwt_token(&token_str).map_err(|e| {
                tracing::warn!("Invalid JWT token in query parameter: {}", e);
                poem::Error::from_status(StatusCode::UNAUTHORIZED)
            })?;

            let now = chrono::Utc::now().timestamp();
            if claims.exp <= now {
                tracing::warn!("Expired JWT token in query parameter");
                return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
            }

            let user_id = Uuid::parse_str(&claims.sub)
                .map_err(|_| poem::Error::from_status(StatusCode::UNAUTHORIZED))?;

            let user = crate::models::user::User::find_by_id(&state.database.pool, user_id)
                .await
                .map_err(|e| {
                    tracing::error!("Database error looking up user: {}", e);
                    poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                })?
                .ok_or_else(|| {
                    tracing::warn!("User not found: {}", user_id);
                    poem::Error::from_status(StatusCode::UNAUTHORIZED)
                })?;

            crate::server::auth::AuthenticatedUser { user, claims }
        } else {
            // Try to extract from Authorization header using our helper
            match crate::server::auth::extract_user_from_request(req).await {
                Ok(Some(user)) => user,
                Ok(None) => return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED)),
                Err(_) => return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED)),
            }
        };

        let user_id = authenticated_user.user.user_id;

        // Verify that the chat belongs to the authenticated user
        let chat = WorkshopChat::find_by_id(*chat_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error finding chat: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        if chat.user_id != user_id {
            tracing::warn!(
                "User {} attempted to stream from chat {} owned by {}",
                user_id,
                *chat_id,
                chat.user_id
            );
            return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
        }

        tracing::info!(
            "Stream request for chat: {}, message: {}",
            *chat_id,
            *message_id
        );

        // List all ongoing prompts for debugging
        let all_keys = state.workshop.ongoing_prompts.list_keys().await;
        tracing::info!("Available ongoing prompt keys: {:?}", all_keys);

        // Try to get the ongoing prompt
        let ongoing_prompt = state
            .workshop
            .get_ongoing_prompt(*chat_id, *message_id)
            .await
            .ok_or_else(|| {
                tracing::error!(
                    "No ongoing prompt found for chat {} message {}",
                    *chat_id,
                    *message_id
                );
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?;

        tracing::info!("Found ongoing prompt, starting stream");

        // Get the stream
        let stream = ongoing_prompt.get_stream().await;

        // Convert to streaming response events
        let response_stream = stream
            .map(|result| match result {
                Ok(delta) => {
                    let content = delta
                        .choices
                        .first()
                        .and_then(|c| c.delta.content.as_ref())
                        .cloned()
                        .unwrap_or_default();

                    StreamingResponse {
                        content,
                        is_complete: false,
                        error: None,
                    }
                }
                Err(err) => {
                    tracing::error!("Stream error: {}", err);
                    StreamingResponse {
                        content: String::new(),
                        is_complete: true,
                        error: Some(err),
                    }
                }
            })
            .boxed();

        Ok(EventStream::new(response_stream))
    }

    /// /ws/t/:topic_id/summary/stream
    ///
    /// Trigger summary generation and start streaming (or coalesce if already running)
    /// Endpoint does not require authentication
    #[oai(
        path = "/ws/t/:topic_id/summary/stream",
        method = "post",
        tag = "ApiTags::Workshop"
    )]
    async fn start_topic_summary_stream(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<serde_json::Value>> {
        let topic = Topic::get_by_topic_id(topic_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting topic: {:?}", e);
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?;

        // First check if we already have a recent summary
        if let Ok(existing_summary) = sqlx::query_as!(
            crate::models::topics::TopicSummary,
            "SELECT * FROM topic_summaries WHERE topic_id = $1 ORDER BY based_on DESC LIMIT 1",
            topic_id.0
        )
        .fetch_optional(&state.database.pool)
        .await
        {
            if let Some(summary) = existing_summary {
                let based_on = topic
                    .last_post_at
                    .map(|dt| dt.timestamp())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp());

                // If summary is current, return existing
                if summary.based_on.timestamp() == based_on as i64 {
                    return Ok(Json(serde_json::json!({
                        "status": "existing",
                        "topic_id": topic_id.0,
                        "summary": summary.summary_text
                    })));
                }
            }
        }

        // Check if there's already an ongoing stream
        if let Some(_existing_prompt) = state.workshop.get_ongoing_summary_prompt(topic_id.0).await
        {
            return Ok(Json(serde_json::json!({
                "status": "ongoing",
                "topic_id": topic_id.0
            })));
        }

        // Start the summary generation (or get existing ongoing prompt)
        let _ongoing_prompt = WorkshopService::create_workshop_summary_streaming(&topic, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error starting summary generation: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Spawn a task to handle completion and update the topic summary
        let topic_clone = topic.clone();
        let state_clone = state.clone();

        task::spawn(async move {
            if let Some(ongoing_prompt) = state_clone
                .workshop
                .get_ongoing_summary_prompt(topic_clone.topic_id)
                .await
            {
                match ongoing_prompt.await_completion().await {
                    Ok(content) => {
                        if let Some(u) = ongoing_prompt.get_usage().await {
                            metrics::record_openai_usage(None, SUMMARY_MODEL, &u);
                        }
                        // Update the topic summary in the database
                        let based_on = topic_clone
                            .last_post_at
                            .map(|dt| dt.timestamp())
                            .unwrap_or_else(|| chrono::Utc::now().timestamp());

                        let based_on_datetime =
                            chrono::DateTime::from_timestamp(based_on as i64, 0)
                                .unwrap_or_else(|| chrono::Utc::now());

                        if let Err(e) = sqlx::query!(
                            "INSERT INTO topic_summaries (topic_id, based_on, summary_text, created_at) VALUES ($1, $2, $3, NOW())",
                            topic_clone.topic_id,
                            based_on_datetime,
                            content
                        )
                        .execute(&state_clone.database.pool)
                        .await {
                            tracing::error!("Error saving topic summary: {:?}", e);
                        } else {
                            tracing::info!("Saved new summary for topic_id: {}", topic_clone.topic_id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error in summary completion: {:?}", e);
                    }
                }
            }
        });

        Ok(Json(serde_json::json!({
            "status": "started",
            "topic_id": topic_id.0
        })))
    }

    /// /ws/t/:topic_id/summary/stream
    ///
    /// Get SSE stream for topic summary generation
    /// Endpoint does not require authentication
    #[oai(
        path = "/ws/t/:topic_id/summary/stream",
        method = "get",
        tag = "ApiTags::Workshop"
    )]
    async fn stream_topic_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<EventStream<BoxStream<'static, StreamingResponse>>> {
        tracing::info!("Summary stream request for topic: {}", topic_id.0);

        // Try to get the ongoing summary prompt
        let ongoing_prompt = state
            .workshop
            .get_ongoing_summary_prompt(topic_id.0)
            .await
            .ok_or_else(|| {
                tracing::error!("No ongoing summary prompt found for topic {}", topic_id.0);
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?;

        tracing::info!("Found ongoing summary prompt, starting stream");

        // Get the stream
        let stream = ongoing_prompt.get_stream().await;

        // Convert to streaming response events
        let response_stream = stream
            .map(|result| match result {
                Ok(delta) => {
                    let content = delta
                        .choices
                        .first()
                        .and_then(|c| c.delta.content.as_ref())
                        .cloned()
                        .unwrap_or_default();

                    StreamingResponse {
                        content,
                        is_complete: false,
                        error: None,
                    }
                }
                Err(err) => {
                    tracing::error!("Summary stream error: {}", err);
                    StreamingResponse {
                        content: String::new(),
                        is_complete: true,
                        error: Some(err),
                    }
                }
            })
            .boxed();

        Ok(EventStream::new(response_stream))
    }
}
