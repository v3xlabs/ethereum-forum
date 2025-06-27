use crate::models::topics::Topic;
use crate::models::workshop::snapshot::{CreateChatSnapshotPayload, WorkshopSnapshotResponse};
use crate::models::workshop::usage::{get_user_daily_usage, get_user_usage_by_model, get_user_usage_stats};
use crate::models::workshop::{
    chat::WorkshopChat,
    message::WorkshopMessage,
    snapshot::WorkshopSnapshot,
    usage::{DailyUsage, ModelUsage, UserUsageOverview, UserUsageStats},
};
use crate::modules::workshop::WorkshopService;
use crate::modules::workshop::prompts::{
    StreamingEntryType as PromptsStreamingEntryType, ToolCallEntry as PromptsToolCallEntry,
    ToolCallStatus as PromptsToolCallStatus,
};
use crate::server::ApiTags;
use crate::server::auth::AuthUser;
use crate::state::AppState;
use async_std::task;
use futures::{StreamExt, stream::BoxStream};
use poem::Request;
use poem::Result;
use poem::web::Data;
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::{EventStream, Json};
use poem_openapi::{Enum, Object, OpenApi};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopChatInput {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WorkshopChatPayload {
    pub chat_id: Uuid,
    pub chat: WorkshopChat,
    pub messages: Vec<WorkshopMessage>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct UserUsageResponse {
    pub stats: UserUsageStats,
    pub by_model: Vec<ModelUsage>,
    pub daily_usage: Vec<DailyUsage>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminUsageResponse {
    pub total_users: i32,
    pub total_tokens: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub users: Vec<UserUsageOverview>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct StreamingResponse {
    pub content: String,
    pub is_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: StreamingEntryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallEntry>,
}

#[derive(Debug, Serialize, Deserialize, Enum)]
#[serde(rename_all = "snake_case")]
pub enum StreamingEntryType {
    Content,
    ToolCallStart,
    ToolCallResult,
    ToolCallError,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct ToolCallEntry {
    pub tool_name: String,
    pub tool_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub status: ToolCallStatus,
}

#[derive(Debug, Serialize, Deserialize, Enum)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Starting,
    Executing,
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AvailableModel {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AvailableModelsResponse {
    pub models: Vec<AvailableModel>,
    pub default_model: String,
}

// Conversion functions
fn convert_entry_type(entry_type: PromptsStreamingEntryType) -> StreamingEntryType {
    match entry_type {
        PromptsStreamingEntryType::Content => StreamingEntryType::Content,
        PromptsStreamingEntryType::ToolCallStart => StreamingEntryType::ToolCallStart,
        PromptsStreamingEntryType::ToolCallResult => StreamingEntryType::ToolCallResult,
        PromptsStreamingEntryType::ToolCallError => StreamingEntryType::ToolCallError,
    }
}

fn convert_tool_call_entry(entry: PromptsToolCallEntry) -> ToolCallEntry {
    ToolCallEntry {
        tool_name: entry.tool_name,
        tool_id: entry.tool_id,
        arguments: entry.arguments,
        result: entry.result,
        status: convert_tool_call_status(entry.status),
    }
}

fn convert_tool_call_status(status: PromptsToolCallStatus) -> ToolCallStatus {
    match status {
        PromptsToolCallStatus::Starting => ToolCallStatus::Starting,
        PromptsToolCallStatus::Executing => ToolCallStatus::Executing,
        PromptsToolCallStatus::Success => ToolCallStatus::Success,
        PromptsToolCallStatus::Error => ToolCallStatus::Error,
    }
}

#[OpenApi]
impl WorkshopApi {
    /// /ws/t/:discourse_id/:topic_id/summary/to-chat
    ///
    /// Create a new chat from a topic summary
    #[oai(
        path = "/ws/t/:discourse_id/:topic_id/summary/to-chat",
        method = "post",
        tag = "ApiTags::Workshop"
    )]
    async fn create_chat_from_summary(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        #[oai(style = "simple")] discourse_id: Path<String>,
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

        let summary = Topic::get_summary_by_topic_id(&discourse_id, topic_id.0, &state)
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

    /// /ws/models
    ///
    /// Get available models for the user
    #[oai(path = "/ws/models", method = "get", tag = "ApiTags::Workshop")]
    async fn get_available_models(
        &self,
        _state: Data<&AppState>,
        _auth_user: AuthUser,
    ) -> Result<Json<AvailableModelsResponse>> {
        // For now, return a hardcoded list of available models
        // In the future, this could be dynamically fetched from the LLM provider
        // or based on user permissions/subscription level
        let models = vec![
            AvailableModel {
                id: "google/gemini-2.5-flash-preview-05-20".to_string(),
                name: "Gemini 2.5 Flash Preview".to_string(),
                provider: "Google".to_string(),
                is_default: true,
            },
            AvailableModel {
                id: "google/gemini-2.0-flash-001".to_string(),
                name: "Gemini 2.0 Flash".to_string(),
                provider: "Google".to_string(),
                is_default: false,
            },
            AvailableModel {
                id: "google/gemini-2.5-pro-preview".to_string(),
                name: "Gemini 2.5 Pro Preview".to_string(),
                provider: "Google".to_string(),
                is_default: false,
            },
            AvailableModel {
                id: "anthropic/claude-sonnet-4".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider: "Anthropic".to_string(),
                is_default: false,
            },
            AvailableModel {
                id: "openai/gpt-4o-mini".to_string(),
                name: "OpenAI o4 Mini".to_string(),
                provider: "OpenAI".to_string(),
                is_default: false,
            },
            AvailableModel {
                id: "mistralai/mistral-nemo".to_string(),
                name: "Mistral Nemo".to_string(),
                provider: "Mistral AI".to_string(),
                is_default: false,
            },
            // AvailableModel {
            //     id: "mistralai/mistral-7b-instruct:free".to_string(),
            //     name: "Mistral 7B Instruct (Free)".to_string(),
            //     provider: "Mistral AI".to_string(),
            //     is_default: false,
            // },
            // AvailableModel {
            //     id: "deepseek/deepseek-r1-0528:free".to_string(),
            //     name: "DeepSeek R1 0528".to_string(),
            //     provider: "DeepSeek".to_string(),
            //     is_default: false,
            // },
        ];

        Ok(Json(AvailableModelsResponse {
            default_model: "google/gemini-2.5-flash-preview-05-20".to_string(),
            models,
        }))
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

        // Extract the model from the payload, or use default if not specified
        let model = payload.model.clone();

        // Start processing the next message (this will create an OngoingPrompt)
        let (_ongoing_prompt, created_message) = WorkshopService::process_next_message_with_model(
            message.chat_id,
            message.message_id,
            model,
            &state,
        )
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
                Ok(entry) => StreamingResponse {
                    content: entry.content,
                    is_complete: false,
                    error: None,
                    entry_type: convert_entry_type(entry.entry_type),
                    tool_call: entry.tool_call.map(convert_tool_call_entry),
                },
                Err(err) => {
                    tracing::error!("Stream error: {}", err);
                    StreamingResponse {
                        content: String::new(),
                        is_complete: true,
                        error: Some(err),
                        entry_type: StreamingEntryType::ToolCallError,
                        tool_call: None,
                    }
                }
            })
            .boxed();

        Ok(EventStream::new(response_stream))
    }

    /// /ws/t/:discourse_id/:topic_id/summary/stream
    ///
    /// Trigger summary generation and start streaming (or coalesce if already running)
    /// Endpoint does not require authentication
    #[oai(
        path = "/ws/t/:discourse_id/:topic_id/summary/stream",
        method = "post",
        tag = "ApiTags::Workshop"
    )]
    async fn start_topic_summary_stream(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<Json<serde_json::Value>> {
        let topic = Topic::get_by_topic_id(&discourse_id, topic_id.0, &state)
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
        if let Some(_existing_prompt) = state
            .workshop
            .get_ongoing_summary_prompt(&discourse_id, topic_id.0)
            .await
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
                .get_ongoing_summary_prompt(&discourse_id, topic_clone.topic_id)
                .await
            {
                match ongoing_prompt.await_completion().await {
                    Ok(content) => {
                        // Update the topic summary in the database
                        let based_on = topic_clone
                            .last_post_at
                            .map(|dt| dt.timestamp())
                            .unwrap_or_else(|| chrono::Utc::now().timestamp());

                        let based_on_datetime =
                            chrono::DateTime::from_timestamp(based_on as i64, 0)
                                .unwrap_or_else(|| chrono::Utc::now());

                        if let Err(e) = sqlx::query!(
                            "INSERT INTO topic_summaries (discourse_id, topic_id, based_on, summary_text, created_at) VALUES ($1, $2, $3, $4, NOW())",
                            topic_clone.discourse_id,
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

    /// /ws/t/:discourse_id/:topic_id/summary/stream
    ///
    /// Get SSE stream for topic summary generation
    /// Endpoint does not require authentication
    #[oai(
        path = "/ws/t/:discourse_id/:topic_id/summary/stream",
        method = "get",
        tag = "ApiTags::Workshop"
    )]
    async fn stream_topic_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> Result<EventStream<BoxStream<'static, StreamingResponse>>> {
        tracing::info!(
            "Summary stream request for topic: {} on {}",
            topic_id.0,
            discourse_id.0
        );

        // Try to get the ongoing summary prompt
        let ongoing_prompt = state
            .workshop
            .get_ongoing_summary_prompt(&discourse_id, topic_id.0)
            .await
            .ok_or_else(|| {
                tracing::error!(
                    "No ongoing summary prompt found for topic {} on {}",
                    topic_id.0,
                    discourse_id.0
                );
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?;

        tracing::info!("Found ongoing summary prompt, starting stream");

        // Get the stream
        let stream = ongoing_prompt.get_stream().await;

        // Convert to streaming response events
        let response_stream = stream
            .map(|result| match result {
                Ok(entry) => StreamingResponse {
                    content: entry.content,
                    is_complete: false,
                    error: None,
                    entry_type: convert_entry_type(entry.entry_type),
                    tool_call: entry.tool_call.map(convert_tool_call_entry),
                },
                Err(err) => {
                    tracing::error!("Summary stream error: {}", err);
                    StreamingResponse {
                        content: String::new(),
                        is_complete: true,
                        error: Some(err),
                        entry_type: StreamingEntryType::ToolCallError,
                        tool_call: None,
                    }
                }
            })
            .boxed();

        Ok(EventStream::new(response_stream))
    }

    /// /ws/usage
    ///
    /// Get current user's usage statistics
    #[oai(path = "/ws/usage", method = "get", tag = "ApiTags::Workshop")]
    async fn get_user_usage(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        #[oai(style = "simple")] days: Query<Option<i32>>,
    ) -> Result<Json<UserUsageResponse>> {
        let user_id = auth_user.0.user.user_id;
        let days = days.0.unwrap_or(30); // Default to 30 days

        // Get overall stats
        let stats = get_user_usage_stats(user_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting user usage stats: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Get usage by model
        let by_model = get_user_usage_by_model(user_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting user usage by model: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Get daily usage
        let daily_usage = get_user_daily_usage(user_id, days, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting user daily usage: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(UserUsageResponse {
            stats,
            by_model,
            daily_usage,
        }))
    }

    /// /ws/share
    /// 
    /// Creates a new chat snapshot
    #[oai(path = "/ws/share", method = "post", tag = "ApiTags::Workshop")]
    async fn create_chat_snapshot(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        payload: Json<CreateChatSnapshotPayload>,
    ) -> Result<Json<WorkshopSnapshot>> {
        let user = auth_user.0.user_id();

        let snapshot = WorkshopSnapshot::create(payload.chat_id, payload.message_id, user, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error creating chat snapshot: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;
        Ok(Json(snapshot))
    }

    /// /ws/share/:snapshot_id
    ///
    /// Get a chat snapshot by snapshot ID
    #[oai(path = "/ws/share/:snapshot_id", method = "get", tag = "ApiTags::Workshop")]
    async fn get_chat_snapshot(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] snapshot_id: Path<Uuid>,
    ) -> Result<Json<WorkshopSnapshotResponse>> {
        let snapshot = WorkshopSnapshotResponse::get_snapshot_response(snapshot_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting chat snapshot: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;
        Ok(Json(snapshot))
    }

    /// /ws/share/:snapshot_id/messages
    /// 
    /// Get all messages by snapshot ID
    #[oai(path = "/ws/share/:snapshot_id/messages", method = "get", tag = "ApiTags::Workshop")]
    async fn get_chat_snapshot_messages(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] snapshot_id: Path<Uuid>,
    ) -> Result<Json<Vec<WorkshopMessage>>> {
        let snapshot = WorkshopSnapshot::get_by_snapshot_id(snapshot_id.0, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting chat snapshot: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        let messages = WorkshopMessage::get_messages_upwards(&snapshot.message_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error getting chat snapshot messages: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(messages))
    }

    /// /ws/chat/:chat_id
    ///
    /// Delete a chat and all associated messages and snapshots
    #[oai(
        path = "/ws/chat/:chat_id",
        method = "delete",
        tag = "ApiTags::Workshop"
    )]
    async fn delete_chat(
        &self,
        state: Data<&AppState>,
        auth_user: AuthUser,
        #[oai(style = "simple")] chat_id: Path<Uuid>,
    ) -> Result<Json<serde_json::Value>> {
        let user_id = auth_user.0.user.user_id;

        let chat = WorkshopChat::find_by_id(*chat_id, &state)
            .await
            .map_err(|e| {
                tracing::error!("Error finding chat: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        if chat.user_id != user_id {
            tracing::warn!(
                "User {} attempted to delete chat {} owned by {}",
                user_id,
                *chat_id,
                chat.user_id
            );
            return Err(poem::Error::from_status(StatusCode::FORBIDDEN));
        }

        WorkshopChat::delete(&chat_id, &state).await.map_err(|e| {
            tracing::error!("Error deleting chat: {:?}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        tracing::info!(
            "Successfully deleted chat {} for user {}",
            *chat_id,
            user_id
        );

        Ok(Json(serde_json::json!({ "success": true })))
    }
}
