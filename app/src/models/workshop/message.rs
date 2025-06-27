use async_openai::types::{ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage};
use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as};
use uuid::Uuid;

use crate::{models::workshop::{chat::WorkshopChat}, modules::workshop::prompts::{StreamingEntry, StreamingEntryType}, state::AppState};


#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Object)]
pub struct WorkshopMessage {
    pub message_id: Uuid,
    pub chat_id: Uuid,
    pub sender_role: String, // "user", "assistant", or "system"
    pub message: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_events: Option<serde_json::Value>,
    // Token usage tracking (only populated for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
}


impl WorkshopMessage {
    pub async fn create_user_message(
        chat_id: Option<Uuid>,
        parent_message_id: Option<Uuid>,
        user_id: Uuid,
        message: String,
        state: &AppState,
    ) -> Result<Self, sqlx::Error> {
        let chat_id = match chat_id {
            Some(chat_id) => chat_id,
            _ => {
                let chat = WorkshopChat::create(user_id, state).await?;
                chat.chat_id
            }
        };

        query_as!(Self, "INSERT INTO workshop_messages (chat_id, sender_role, message, parent_message_id) VALUES ($1, $2, $3, $4) RETURNING message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used",
            chat_id,
            "user",
            message,
            parent_message_id
        )
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn create_system_response(
        chat_id: &Uuid,
        parent_message_id: Option<Uuid>,
        message: String,
        state: &AppState,
    ) -> Result<Self, sqlx::Error> {
        query_as!(Self, "INSERT INTO workshop_messages (chat_id, sender_role, message, parent_message_id) VALUES ($1, $2, $3, $4) RETURNING message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used",
            chat_id,
            "assistant",
            message,
            parent_message_id
        )
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn update_message_content(message_id: &Uuid, content: &str, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as!(Self, "UPDATE workshop_messages SET message = $1 WHERE message_id = $2 RETURNING message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used",
            content,
            message_id
        )
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn update_message_with_streaming_events(
        message_id: &Uuid, 
        content: &str, 
        streaming_events: &[StreamingEntry], 
        state: &AppState
    ) -> Result<Self, sqlx::Error> {
        let events_json = serde_json::to_value(streaming_events).unwrap_or(serde_json::Value::Null);
        query_as!(Self, "UPDATE workshop_messages SET message = $1, streaming_events = $2 WHERE message_id = $3 RETURNING message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used",
            content,
            events_json,
            message_id
        )
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn update_message_with_token_usage(
        message_id: &Uuid,
        content: &str,
        streaming_events: &[StreamingEntry],
        usage: Option<&async_openai::types::CompletionUsage>,
        model_used: &str,
        state: &AppState
    ) -> Result<Self, sqlx::Error> {
        let events_json = serde_json::to_value(streaming_events).unwrap_or(serde_json::Value::Null);
        
        let (prompt_tokens, completion_tokens, total_tokens, reasoning_tokens) = if let Some(usage) = usage {
            (
                Some(usage.prompt_tokens as i32),
                Some(usage.completion_tokens as i32),
                Some(usage.total_tokens as i32),
                // reasoning_tokens might be available in newer API versions
                None::<i32>, // For now, we don't have access to reasoning tokens in the current async-openai version
            )
        } else {
            (None, None, None, None)
        };

        query_as!(Self, 
            "UPDATE workshop_messages SET message = $1, streaming_events = $2, prompt_tokens = $3, completion_tokens = $4, total_tokens = $5, reasoning_tokens = $6, model_used = $7 WHERE message_id = $8 RETURNING message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used",
            content,
            events_json,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            reasoning_tokens,
            model_used,
            message_id
        )
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn get_messages_by_chat_id(
        chat_id: &Uuid,
        state: &AppState,
    ) -> Result<Vec<Self>, sqlx::Error> {
        query_as!(
            Self,
            "SELECT m.message_id, m.chat_id, m.sender_role, m.message, m.created_at, m.parent_message_id, m.streaming_events, m.prompt_tokens, m.completion_tokens, m.total_tokens, m.reasoning_tokens, m.model_used 
             FROM workshop_messages m 
             INNER JOIN workshop_chats c ON m.chat_id = c.chat_id 
             WHERE m.chat_id = $1 AND c.deleted_at IS NULL 
             ORDER BY m.created_at ASC",
            chat_id
        )
        .fetch_all(&state.database.pool)
        .await
    }

    /// Gets all messages (for use with snapshots) upwards
    /// As such only returning the singular branch up until parent_message_id = NULL
    /// Starts querying at chat_id message_id and works its way up to the root message
    pub async fn get_messages_upwards(
        message_id: &Uuid,
        state: &AppState,
    ) -> Result<Vec<Self>, sqlx::Error> {
        query_as(
            r#"WITH RECURSIVE message_tree AS (
            SELECT m.message_id, m.chat_id, m.sender_role, m.message, m.created_at, m.parent_message_id, m.streaming_events, m.prompt_tokens, m.completion_tokens, m.total_tokens, m.reasoning_tokens, m.model_used 
            FROM workshop_messages m 
            INNER JOIN workshop_chats c ON m.chat_id = c.chat_id 
            WHERE m.message_id = $1 AND c.deleted_at IS NULL
            UNION ALL
            SELECT m.message_id, m.chat_id, m.sender_role, m.message, m.created_at, m.parent_message_id, m.streaming_events, m.prompt_tokens, m.completion_tokens, m.total_tokens, m.reasoning_tokens, m.model_used 
            FROM workshop_messages m
            INNER JOIN message_tree mt ON m.message_id = mt.parent_message_id
            INNER JOIN workshop_chats c ON m.chat_id = c.chat_id AND c.deleted_at IS NULL
        )
        SELECT message_id, chat_id, sender_role, message, created_at, parent_message_id, streaming_events, prompt_tokens, completion_tokens, total_tokens, reasoning_tokens, model_used FROM message_tree
        ORDER BY created_at ASC"#,
        )
        .bind(message_id)
        .fetch_all(&state.database.pool)
        .await
    }

    /// Get streaming events as a Vec<StreamingEntry> if they exist
    pub fn get_streaming_events(&self) -> Option<Vec<StreamingEntry>> {
        self.streaming_events.as_ref().and_then(|v| {
            serde_json::from_value(v.clone()).ok()
        })
    }

    /// Set streaming events from a Vec<StreamingEntry>
    pub fn set_streaming_events(&mut self, events: Vec<StreamingEntry>) {
        self.streaming_events = serde_json::to_value(events).ok();
    }

    /// Extract text content from streaming events
    pub fn get_content_from_streaming_events(&self) -> String {
        if let Some(events) = self.get_streaming_events() {
            events
                .iter()
                .filter(|event| event.entry_type == StreamingEntryType::Content)
                .map(|event| event.content.as_str())
                .collect::<Vec<_>>()
                .join("")
        } else {
            self.message.clone()
        }
    }

    /// Extract OpenAI-compatible tool calls from streaming events for reuse in subsequent requests
    pub fn get_openai_tool_calls(&self) -> Option<Vec<async_openai::types::ChatCompletionMessageToolCall>> {
        let events = self.get_streaming_events()?;
        let mut tool_calls = std::collections::HashMap::new();

        // Process events to build complete tool calls
        for event in events {
            if let Some(tool_call_entry) = event.tool_call {
                let tool_call_id = tool_call_entry.tool_id.clone();
                
                // Get or create the tool call
                let tool_call = tool_calls.entry(tool_call_id.clone()).or_insert_with(|| {
                    async_openai::types::ChatCompletionMessageToolCall {
                        id: tool_call_id.clone(),
                        function: async_openai::types::FunctionCall {
                            name: tool_call_entry.tool_name.clone(),
                            arguments: tool_call_entry.arguments.clone().unwrap_or_default(),
                        },
                        r#type: async_openai::types::ChatCompletionToolType::Function,
                    }
                });

                // Update with any new information (tool calls can be updated across multiple events)
                if let Some(args) = &tool_call_entry.arguments {
                    if !args.is_empty() {
                        tool_call.function.arguments = args.clone();
                    }
                }
            }
        }

        if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls.into_values().collect())
        }
    }
}

impl Into<ChatCompletionRequestMessage> for WorkshopMessage {
    fn into(self) -> ChatCompletionRequestMessage {
        match self.sender_role.as_str() {
            "user" => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: self.message.into(),
                name: None,
            }),
            "assistant" => {
                // Extract tool calls first before moving self
                let tool_calls = self.get_openai_tool_calls();
                
                // For assistant messages, prefer content from streaming events over message field
                let content = if self.streaming_events.is_some() {
                    let content_from_events = self.get_content_from_streaming_events();
                    if content_from_events.is_empty() { None } else { Some(content_from_events.into()) }
                } else if self.message.is_empty() {
                    None
                } else {
                    Some(self.message.into())
                };
                
                ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                    content,
                    name: None,
                    tool_calls,
                    #[allow(deprecated)]
                    function_call: None,
                    refusal: None,
                    audio: None,
                })
            },
            "system" => ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: self.message.into(),
                name: None,
            }),
            _ => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: self.message.into(),
                name: None,
            }),
        }
    }
}
