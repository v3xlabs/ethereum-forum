use async_openai::{
    Client,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
    },
};
use async_std::sync::RwLock;
use async_std::task;
use opentelemetry_http::HttpError;
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    models::{
        topics::{
            Topic,
            post::{Post, WorkshopPost},
        },
        workshop::{chat::WorkshopChat, message::WorkshopMessage},
    },
    modules::workshop::prompts::{
        OngoingPrompt, OngoingPromptManager, SHORTSUM_MODEL, SUMMARY_MODEL,
        truncate_messages_to_token_limit,
    },
    state::AppState,
};

pub mod mcp_client;
pub mod prompts;

pub struct WorkshopService {
    pub client: Client<async_openai::config::OpenAIConfig>,
    pub prompts: WorkshopPrompts,
    // Manager for request coalescing of streaming responses
    pub ongoing_prompts: OngoingPromptManager,
    // MCP client manager for AI tool calling
    pub mcp_client: Arc<RwLock<mcp_client::McpClientManager>>,
}

pub struct WorkshopPrompts {
    pub summerize: ChatCompletionRequestMessage,
    pub shortsum: ChatCompletionRequestMessage,
}

impl Default for WorkshopPrompts {
    fn default() -> Self {
        Self {
            summerize: ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: prompts::SUMMARY_PROMPT.to_string().into(),
                name: None,
            }),
            shortsum: ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: prompts::SHORTSUM_PROMPT.to_string().into(),
                name: None,
            }),
        }
    }
}

impl WorkshopService {
    pub async fn init() -> Self {
        let api_key =
            std::env::var("WORKSHOP_INTELLIGENCE_KEY").expect("WORKSHOP_INTELLIGENCE_KEY not set");

        let base_url = std::env::var("WORKSHOP_INTELLIGENCE_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

        tracing::info!("🔧 Workshop Service Init:");
        tracing::info!("  API Key present: {}", !api_key.is_empty());
        tracing::info!("  API Key length: {}", api_key.len());
        tracing::info!("  Base URL: {}", base_url);

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);

        let client = Client::with_config(config);
        tracing::info!("  OpenAI client configured successfully");

        // Initialize MCP client manager
        let mut mcp_client = mcp_client::McpClientManager::new();
        let mcp_base_url = std::env::var("MCP_BASE_URL")
            .unwrap_or_else(|_| "https://ethereum.forum/mcp".to_string());

        if let Err(e) = mcp_client.init_default_client(mcp_base_url).await {
            tracing::warn!("Failed to initialize MCP client: {}", e);
        }

        Self {
            client,
            prompts: WorkshopPrompts::default(),
            ongoing_prompts: OngoingPromptManager::new(),
            mcp_client: Arc::new(RwLock::new(mcp_client)),
        }
    }

    pub async fn create_workshop_summary(
        topic: &Topic,
        state: &AppState,
    ) -> Result<String, HttpError> {
        let posts =
            Post::find_by_topic_id(&topic.discourse_id, topic.topic_id, 1, Some(512), state).await;

        let (posts, _) = posts.unwrap_or_default();
        let posts: Vec<WorkshopPost> = posts.into_iter().map(|x| x.into()).collect();

        let messages = vec![
            state.workshop.prompts.summerize.clone(),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: serde_json::to_string(&json!({
                    "topic_info": topic,
                    "posts": posts,
                }))
                .unwrap()
                .into(),
                name: None,
            }),
        ];

        // Apply token limits to prevent excessive costs
        let truncated_messages = truncate_messages_to_token_limit(messages, &None);

        let request = CreateChatCompletionRequest {
            model: SUMMARY_MODEL.to_string(),
            messages: truncated_messages,
            max_completion_tokens: Some(2000), // Limit output to 2k tokens for summaries
            ..Default::default()
        };

        let chat_completion = state.workshop.client.chat().create(request).await?;

        let response = chat_completion.choices.first().unwrap().message.clone();

        // Log usage data if available
        if let Some(usage) = &chat_completion.usage {
            tracing::info!(
                "💰 Summary generation usage - prompt: {}, completion: {}, total: {}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens
            );
        }

        Ok(response.content.unwrap_or_default())
    }

    /// Process next message with default model
    ///
    /// Fetches the entire chat history from chat_id upwards and processes it with the LLM
    /// Returns the next message from the LLM using request coalescing
    pub async fn process_next_message(
        chat_id: Uuid,
        message_id: Uuid,
        state: &AppState,
    ) -> Result<(OngoingPrompt, WorkshopMessage), Box<dyn std::error::Error + Send + Sync>> {
        Self::process_next_message_with_model(chat_id, message_id, None, state).await
    }

    /// Process next message with specified model
    ///
    /// Fetches the entire chat history from chat_id upwards and processes it with the LLM
    /// Returns the next message from the LLM using request coalescing
    pub async fn process_next_message_with_model(
        chat_id: Uuid,
        message_id: Uuid,
        model: Option<String>,
        state: &AppState,
    ) -> Result<(OngoingPrompt, WorkshopMessage), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "🔄 Starting process_next_message for chat: {}, message: {}",
            chat_id,
            message_id
        );

        let mut messages: Vec<ChatCompletionRequestMessage> =
            WorkshopMessage::get_messages_upwards(&message_id, state)
                .await?
                .into_iter()
                .map(|m| m.into())
                .collect();

        tracing::info!("📝 Retrieved {} messages for context", messages.len());

        // Process MCP tool calls from the conversation
        match mcp_client::ToolCallHelper::process_tool_calls(
            &mut *state.workshop.mcp_client.write().await,
            &messages,
        )
        .await
        {
            Ok(tool_results) => {
                if !tool_results.is_empty() {
                    let tool_context =
                        mcp_client::ToolCallHelper::format_tool_results(&tool_results);
                    info!("Adding tool results to context: {}", tool_context);

                    // Add tool results as a system message
                    let tool_message =
                        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                            content: format!("Tool Results:\n{}", tool_context).into(),
                            name: None,
                        });
                    messages.push(tool_message);
                }
            }
            Err(e) => {
                info!("Error processing tool calls: {}", e);
            }
        }

        let system_message =
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: prompts::WORKSHOP_PROMPT.to_string().into(),
                name: None,
            });

        messages.insert(0, system_message);

        // Get available MCP tools for the chat completion
        tracing::info!("🔧 Getting MCP tools...");
        let mut mcp_client_lock_result = state.workshop.mcp_client.write().await;
        tracing::info!("🔓 MCP client lock acquired successfully");

        let tools = match mcp_client_lock_result.get_openai_tools().await {
            Ok(mut tools) if !tools.is_empty() => {
                tracing::info!("✅ Got {} MCP tools", tools.len());

                Some(tools)
            }
            Ok(_) => {
                tracing::info!("ℹ️ No MCP tools available");
                None
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to get MCP tools: {}", e);
                None
            }
        };

        drop(mcp_client_lock_result); // Explicitly drop the lock
        tracing::info!("🔒 MCP client lock released");

        // Use chat_id + message_id as the coalescing key
        let key = format!("{}-{}", chat_id, message_id);
        tracing::info!("🔑 Using coalescing key: {}", key);

        // Get or create the ongoing prompt
        tracing::info!("🚀 Creating OngoingPrompt...");
        let ongoing_prompt = state
            .workshop
            .ongoing_prompts
            .get_or_create(key.clone(), state, messages, tools, model)
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to create OngoingPrompt: {}", e);
                e
            })?;

        // Create an empty system message for this response
        let system_response = WorkshopMessage::create_system_response(
            &chat_id,
            Some(message_id),
            "".to_string(),
            state,
        )
        .await?;

        // Also store the ongoing prompt with the system message key for streaming access
        let system_message_key = format!("{}-{}", chat_id, system_response.message_id);
        state
            .workshop
            .ongoing_prompts
            .insert_additional_key(system_message_key.clone(), ongoing_prompt.clone())
            .await;
        tracing::info!(
            "Also stored ongoing prompt with system key: {}",
            system_message_key
        );

        // Spawn a task to handle completion and update the message
        let prompt_clone = ongoing_prompt.clone();
        let state_clone = state.clone();
        let system_response_clone = system_response.clone();
        let system_message_key_clone = system_message_key.clone();

        task::spawn(async move {
            tracing::info!("⏳ Waiting for prompt completion...");
            match prompt_clone.await_completion().await {
                Ok(content) => {
                    tracing::info!(
                        "✅ Prompt completed successfully with {} characters",
                        content.len()
                    );

                    // Collect all streaming events
                    let streaming_events = prompt_clone.get_all_events().await;
                    tracing::info!("📊 Collected {} streaming events", streaming_events.len());

                    // Get usage data and model information
                    let usage_data = prompt_clone.get_usage_data().await;
                    let model_used = prompt_clone
                        .get_model_used()
                        .await
                        .unwrap_or_else(|| "unknown".to_string());

                    // Update the message content and streaming events with token usage
                    if let Err(e) = WorkshopMessage::update_message_with_token_usage(
                        &system_response_clone.message_id,
                        "", // Empty message field - use streaming_events as source of truth
                        &streaming_events,
                        usage_data.as_ref(),
                        &model_used,
                        &state_clone,
                    )
                    .await
                    {
                        tracing::error!("❌ Error updating message with token usage: {:?}", e);
                    } else {
                        tracing::info!(
                            "✅ Updated message with empty content field (using streaming_events as source of truth)"
                        );
                        if let Some(usage) = &usage_data {
                            tracing::info!(
                                "💰 Token usage saved - prompt: {}, completion: {}, total: {}",
                                usage.prompt_tokens,
                                usage.completion_tokens,
                                usage.total_tokens
                            );
                        }
                    }

                    // Update the chat's last message
                    if let Err(e) = WorkshopChat::update_last_message(
                        &system_response_clone.chat_id,
                        &system_response_clone.message_id,
                        &state_clone,
                    )
                    .await
                    {
                        tracing::error!("❌ Error updating chat: {:?}", e);
                    } else {
                        tracing::info!("✅ Updated chat last message successfully");
                        // Trigger background summarization agent after successful update
                        Self::shortsum_agent(system_response_clone.chat_id, state_clone.clone())
                            .await;
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Error in prompt completion: \"{}\"", e);
                    tracing::error!("❌ Full error details: {:?}", e);

                    // Collect streaming events even on error (may contain partial tool calls)
                    let streaming_events = prompt_clone.get_all_events().await;
                    tracing::info!(
                        "📊 Collected {} streaming events (with error)",
                        streaming_events.len()
                    );

                    // Get usage data and model information even on error
                    let usage_data = prompt_clone.get_usage_data().await;
                    let model_used = prompt_clone
                        .get_model_used()
                        .await
                        .unwrap_or_else(|| "unknown".to_string());

                    // Update the message with error and any streaming events/token usage that were collected
                    let error_message = format!("Error: stream failed: {}", e);
                    if let Err(update_err) = WorkshopMessage::update_message_with_token_usage(
                        &system_response_clone.message_id,
                        &error_message,
                        &streaming_events,
                        usage_data.as_ref(),
                        &model_used,
                        &state_clone,
                    )
                    .await
                    {
                        tracing::error!(
                            "❌ Error updating message with error and token usage: {:?}",
                            update_err
                        );
                    } else {
                        tracing::info!(
                            "📝 Updated message with error content, streaming events, and token usage"
                        );
                        if let Some(usage) = &usage_data {
                            tracing::info!(
                                "💰 Token usage saved (error case) - prompt: {}, completion: {}, total: {}",
                                usage.prompt_tokens,
                                usage.completion_tokens,
                                usage.total_tokens
                            );
                        }
                    }
                }
            }

            // Clean up the system message key after a delay
            task::sleep(std::time::Duration::from_secs(30)).await;
            state_clone
                .workshop
                .ongoing_prompts
                .remove(&system_message_key_clone)
                .await;
            tracing::info!(
                "Cleaned up system message key: {}",
                system_message_key_clone
            );
        });

        Ok((ongoing_prompt, system_response))
    }

    /// Get an ongoing prompt for streaming (if it exists)
    pub async fn get_ongoing_prompt(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
    ) -> Option<OngoingPrompt> {
        let key = format!("{}-{}", chat_id, message_id);
        info!("Getting ongoing prompt for key: {}", key);
        self.ongoing_prompts.get(&key).await
    }

    /// Create workshop summary using streaming with request coalescing
    /// Returns an OngoingPrompt that can be used for streaming the summary generation
    pub async fn create_workshop_summary_streaming(
        topic: &Topic,
        state: &AppState,
    ) -> Result<OngoingPrompt, Box<dyn std::error::Error + Send + Sync>> {
        let posts =
            Post::find_by_topic_id(&topic.discourse_id, topic.topic_id, 1, Some(512), state).await;
        let (posts, _) = posts.unwrap_or_default();
        let posts: Vec<WorkshopPost> = posts.into_iter().map(|x| x.into()).collect();

        let messages = vec![
            state.workshop.prompts.summerize.clone(),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: serde_json::to_string(&json!({
                    "topic_info": topic,
                    "posts": posts,
                }))
                .unwrap()
                .into(),
                name: None,
            }),
        ];

        // Apply token limits to prevent excessive costs
        let truncated_messages = truncate_messages_to_token_limit(messages, &None);

        // Use topic_id as the coalescing key for summaries
        let key = Self::summary_key(&topic.discourse_id, topic.topic_id);

        // Get or create the ongoing prompt (no tools needed for summaries)
        let ongoing_prompt = state
            .workshop
            .ongoing_prompts
            .get_or_create(
                key,
                state,
                truncated_messages,
                None,
                Some(SUMMARY_MODEL.to_string()),
            )
            .await?;

        Ok(ongoing_prompt)
    }

    fn summary_key(discourse_id: &str, topic_id: i32) -> String {
        format!("summary-{}-{}", discourse_id, topic_id)
    }

    /// Get an ongoing summary prompt for streaming (if it exists)
    pub async fn get_ongoing_summary_prompt(
        &self,
        discourse_id: &str,
        topic_id: i32,
    ) -> Option<OngoingPrompt> {
        let key = Self::summary_key(discourse_id, topic_id);
        info!("Getting ongoing summary prompt for key: {}", key);
        self.ongoing_prompts.get(&key).await
    }

    /// Background agent that summarizes conversations after last message updates
    pub async fn shortsum_agent(chat_id: Uuid, state: AppState) {
        tracing::info!("🔄 Starting shortsum agent for chat: {}", chat_id);

        // Spawn the task to run in the background
        task::spawn(async move {
            // Small delay to ensure the message update has been committed
            task::sleep(std::time::Duration::from_millis(500)).await;

            match Self::generate_chat_summary(chat_id, &state).await {
                Ok(summary) => {
                    match WorkshopChat::update_summary(&chat_id, &summary, &state).await {
                        Ok(_) => {
                            tracing::info!(
                                "✅ Successfully updated chat summary for chat: {}",
                                chat_id
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "❌ Failed to update chat summary for chat {}: {}",
                                chat_id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to generate summary for chat {}: {}", chat_id, e);
                }
            }
        });
    }

    /// Generate a summary of the conversation in a chat
    async fn generate_chat_summary(
        chat_id: Uuid,
        state: &AppState,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get all messages in the chat
        let messages = WorkshopMessage::get_messages_by_chat_id(&chat_id, state).await?;

        // Convert workshop messages to chat completion messages for context
        let conversation_messages: Vec<ChatCompletionRequestMessage> =
            messages.into_iter().map(|m| m.into()).collect();

        // Create the conversation context as a single string
        let conversation_context = conversation_messages
            .iter()
            .map(|msg| {
                let (role, content) = match msg {
                    ChatCompletionRequestMessage::User(user_msg) => {
                        let content = match &user_msg.content {
                            async_openai::types::ChatCompletionRequestUserMessageContent::Text(text) => text.clone(),
                            async_openai::types::ChatCompletionRequestUserMessageContent::Array(_) => "[Complex content]".to_string(),
                        };
                        ("User", content)
                    },
                    ChatCompletionRequestMessage::Assistant(assistant_msg) => {
                        let content = if let Some(refusal) = &assistant_msg.refusal {
                            format!("[Refusal: {}]", refusal)
                        } else {
                            assistant_msg.content.as_ref().map(|c| match c {
                                async_openai::types::ChatCompletionRequestAssistantMessageContent::Text(text) => text.clone(),
                                async_openai::types::ChatCompletionRequestAssistantMessageContent::Array(_) => "[Complex content]".to_string(),
                            }).unwrap_or_default()
                        };
                        ("Assistant", content)
                    },
                    ChatCompletionRequestMessage::System(system_msg) => {
                        let content = match &system_msg.content {
                            async_openai::types::ChatCompletionRequestSystemMessageContent::Text(text) => text.clone(),
                            async_openai::types::ChatCompletionRequestSystemMessageContent::Array(_) => "[Complex content]".to_string(),
                        };
                        ("System", content)
                    },
                    _ => ("Unknown", "".to_string()),
                };
                format!("{}: {}", role, content)
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        // Prepare the messages for summarization
        let summary_messages = vec![
            state.workshop.prompts.shortsum.clone(),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: conversation_context.into(),
                name: None,
            }),
        ];

        // Apply token limits to prevent excessive costs
        let truncated_summary_messages = truncate_messages_to_token_limit(summary_messages, &None);

        // Generate the summary using async-openai
        let request = CreateChatCompletionRequest {
            model: SHORTSUM_MODEL.to_string(),
            messages: truncated_summary_messages,
            max_completion_tokens: Some(40),
            ..Default::default()
        };

        let chat_completion = state.workshop.client.chat().create(request).await?;

        let summary = chat_completion
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .unwrap_or(&"Unable to generate summary".to_string())
            .clone();

        // Log usage data if available
        if let Some(usage) = &chat_completion.usage {
            tracing::info!(
                "💰 Short summary generation usage - prompt: {}, completion: {}, total: {}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens
            );
        }

        tracing::info!(
            "📝 Generated summary for chat {}: {}",
            chat_id,
            &summary[..summary.len().min(100)]
        );

        Ok(summary)
    }
}
