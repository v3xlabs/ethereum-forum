use async_std::task;
use futures::{Stream, StreamExt, stream};
use async_openai::{
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequest, ChatCompletionTool,
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestToolMessage,
        ChatCompletionRequestAssistantMessageContent, ChatCompletionMessageToolCall,
        ChatCompletionToolType, FunctionCall},
};
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use async_std::sync::{RwLock, Mutex};
use async_std::channel::{unbounded, Sender};
use tracing;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::state::AppState;

/// Helper function to normalize tool arguments by converting string numbers to actual numbers
/// for known numeric parameters
fn normalize_tool_arguments(tool_name: &str, args: Value) -> Value {
    let mut normalized_args = args;
    
    // List of tools and their numeric parameters that should be converted from strings to numbers
    let numeric_params = match tool_name {
        "get_topic_summary" => vec!["topic_id"],
        "get_posts" => vec!["page", "size", "topic_id"],
        "search_forum" => vec!["limit", "offset"],
        "search_topics" => vec!["limit", "offset"],
        "search_posts" => vec!["limit", "offset"],
        "search_posts_in_topic" => vec!["limit", "offset", "topic_id"],
        "search_by_user" => vec!["limit", "offset", "user_id"],
        "search_by_username" => vec!["limit", "offset"],
        "search_by_username_mention" => vec!["limit", "offset"],
        "username_to_user_id" => vec![],
        "get_user_profile" => vec![],
        "get_user_summary" => vec![],
        _ => vec![], // For unknown tools, don't convert anything
    };
    
    if let Value::Object(ref mut map) = normalized_args {
        // Collect updates to apply later to avoid borrowing conflicts
        let mut updates = Vec::new();
        
        for param in numeric_params {
            if let Some(value) = map.get(param) {
                match value {
                    Value::String(s) => {
                        // Try to parse as integer first, then as float
                        if let Ok(int_val) = s.parse::<i64>() {
                            updates.push((param.to_string(), Value::Number(serde_json::Number::from(int_val))));
                            tracing::debug!("🔢 Converted parameter '{}' from string '{}' to number {}", param, s, int_val);
                        } else if let Ok(float_val) = s.parse::<f64>() {
                            if let Some(num) = serde_json::Number::from_f64(float_val) {
                                updates.push((param.to_string(), Value::Number(num)));
                                tracing::debug!("🔢 Converted parameter '{}' from string '{}' to number {}", param, s, float_val);
                            }
                        }
                    }
                    _ => {
                        // Value is already a number or other type, leave it as is
                    }
                }
            }
        }
        
        // Apply the updates
        for (key, value) in updates {
            map.insert(key, value);
        }
    }
    
    normalized_args
}

pub const SUMMARY_PROMPT: &str = include_str!("./summary.md");
pub const SUMMARY_MODEL: &str = "mistralai/ministral-3b";

pub const WORKSHOP_PROMPT: &str = include_str!("./workshop.md");
pub const WORKSHOP_MODEL: &str = "google/gemini-2.5-flash-preview-05-20";
// pub const WORKSHOP_MODEL: &str = "google/gemini-2.0-flash-001";
// pub const WORKSHOP_MODEL: &str = "google/gemini-2.5-pro-preview";

// TODO: for consideration when we implementing reasoning decoding
// pub const WORKSHOP_MODEL: &str = "google/gemini-2.5-flash-preview-05-20:thinking";

pub const SHORTSUM_PROMPT: &str = include_str!("./shortsum.md");
pub const SHORTSUM_MODEL: &str = "mistralai/mistral-7b-instruct:free";

/// Constants for token limits
const MAX_INPUT_TOKENS: usize = 180000; // Limit input to 32k tokens to prevent excessive costs
const TOKENS_PER_MESSAGE_OVERHEAD: usize = 4; // Overhead tokens per message (role, formatting, etc.)
const TOKENS_PER_NAME: usize = 1; // Additional tokens if name is present

/// Simple token estimation function
/// This is a rough estimate - for exact counts you'd need the actual tokenizer
/// But this is good enough for preventing runaway costs
fn estimate_tokens_in_text(text: &str) -> usize {
    // Rough estimate: ~4 characters per token for English text
    // This errs on the side of overestimating to be safe
    (text.len() as f64 / 3.5).ceil() as usize
}

fn estimate_tokens_in_message(message: &ChatCompletionRequestMessage) -> usize {
    let mut token_count = TOKENS_PER_MESSAGE_OVERHEAD;
    
    match message {
        ChatCompletionRequestMessage::User(user_msg) => {
            let content = match &user_msg.content {
                async_openai::types::ChatCompletionRequestUserMessageContent::Text(text) => text,
                async_openai::types::ChatCompletionRequestUserMessageContent::Array(_) => "[Complex content]",
            };
            token_count += estimate_tokens_in_text(content);
            if user_msg.name.is_some() {
                token_count += TOKENS_PER_NAME;
            }
        },
        ChatCompletionRequestMessage::Assistant(assistant_msg) => {
            if let Some(content) = &assistant_msg.content {
                let text = match content {
                    async_openai::types::ChatCompletionRequestAssistantMessageContent::Text(text) => text,
                    async_openai::types::ChatCompletionRequestAssistantMessageContent::Array(_) => "[Complex content]",
                };
                token_count += estimate_tokens_in_text(text);
            }
            if assistant_msg.name.is_some() {
                token_count += TOKENS_PER_NAME;
            }
            // Add tokens for tool calls if present
            if let Some(tool_calls) = &assistant_msg.tool_calls {
                for tool_call in tool_calls {
                    token_count += estimate_tokens_in_text(&tool_call.function.name);
                    token_count += estimate_tokens_in_text(&tool_call.function.arguments);
                    token_count += 4; // Overhead for tool call structure
                }
            }
        },
        ChatCompletionRequestMessage::System(system_msg) => {
            let content = match &system_msg.content {
                async_openai::types::ChatCompletionRequestSystemMessageContent::Text(text) => text,
                async_openai::types::ChatCompletionRequestSystemMessageContent::Array(_) => "[Complex content]",
            };
            token_count += estimate_tokens_in_text(content);
            if system_msg.name.is_some() {
                token_count += TOKENS_PER_NAME;
            }
        },
        ChatCompletionRequestMessage::Tool(tool_msg) => {
            let content_text = match &tool_msg.content {
                async_openai::types::ChatCompletionRequestToolMessageContent::Text(text) => text,
                async_openai::types::ChatCompletionRequestToolMessageContent::Array(_) => "[Complex content]",
            };
            token_count += estimate_tokens_in_text(content_text);
            token_count += estimate_tokens_in_text(&tool_msg.tool_call_id);
        },
        _ => {
            // For any other message types, add a conservative estimate
            token_count += 50;
        }
    }
    
    token_count
}

pub fn truncate_messages_to_token_limit(mut messages: Vec<ChatCompletionRequestMessage>, tools: &Option<Vec<ChatCompletionTool>>) -> Vec<ChatCompletionRequestMessage> {
    // First, estimate tokens for tools if present
    let mut tool_tokens = 0;
    if let Some(tools_vec) = tools {
        for _tool in tools_vec {
            // Rough estimate for tool definitions
            tool_tokens += 100; // Conservative estimate per tool
        }
    }
    
    let mut total_tokens = tool_tokens;
    let mut kept_messages = Vec::new();
    let mut truncated_count = 0;
    
    // Always keep the system message first if it exists
    if let Some(first_message) = messages.first() {
        if matches!(first_message, ChatCompletionRequestMessage::System(_)) {
            let system_message = messages.remove(0);
            total_tokens += estimate_tokens_in_message(&system_message);
            kept_messages.push(system_message);
        }
    }
    
    // Keep messages from the end (most recent) while staying under limit
    // Work backwards to keep the most recent conversation
    for message in messages.into_iter().rev() {
        let message_tokens = estimate_tokens_in_message(&message);
        
        if total_tokens + message_tokens <= MAX_INPUT_TOKENS {
            total_tokens += message_tokens;
            kept_messages.insert(if kept_messages.is_empty() { 0 } else { 1 }, message); // Insert after system message if present
        } else {
            truncated_count += 1;
        }
    }
    
    if truncated_count > 0 {
        tracing::warn!(
            "🔪 Truncated {} message(s) to stay under {}-token limit. Current estimate: {} tokens",
            truncated_count,
            MAX_INPUT_TOKENS,
            total_tokens
        );
    } else {
        tracing::info!("✅ Messages within token limit. Estimated tokens: {}", total_tokens);
    }
    
    kept_messages
}

/// Enhanced state for streaming with tool call support
#[derive(Clone)]
pub struct OngoingPromptState {
    pub buffer: Arc<RwLock<VecDeque<StreamingEntry>>>,
    pub senders: Arc<Mutex<Vec<Sender<Result<StreamingEntry, String>>>>>,
    pub is_complete: Arc<RwLock<bool>>,
    pub error: Arc<RwLock<Option<String>>>,
    pub final_content: Arc<RwLock<Option<String>>>,
    pub conversation_history: Arc<RwLock<Vec<ChatCompletionRequestMessage>>>,
    pub tools: Arc<RwLock<Option<Vec<ChatCompletionTool>>>>,
    pub usage_data: Arc<RwLock<Option<async_openai::types::CompletionUsage>>>,
    pub model_used: Arc<RwLock<Option<String>>>,
}

/// Streaming entry types to support different kinds of streaming content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingEntry {
    pub content: String,
    #[serde(rename = "type")]
    pub entry_type: StreamingEntryType,
    pub tool_call: Option<ToolCallEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamingEntryType {
    Content,
    ToolCallStart,
    ToolCallResult,
    ToolCallError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub tool_name: String,
    pub tool_id: String,
    pub arguments: Option<String>,
    pub result: Option<String>,
    pub status: ToolCallStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Starting,
    Executing,
    Success,
    Error,
}

/// Enhanced OngoingPrompt with tool calling support
#[derive(Clone)]
pub struct OngoingPrompt {
    pub state: OngoingPromptState,
}

impl OngoingPrompt {
    pub async fn new(state: &AppState, messages: Vec<ChatCompletionRequestMessage>, tools: Option<Vec<ChatCompletionTool>>, model: Option<String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("🚀 Creating new OngoingPrompt with {} messages and {} tools", 
            messages.len(), tools.as_ref().map(|t| t.len()).unwrap_or(0));
        
        let model = model.unwrap_or_else(|| WORKSHOP_MODEL.to_string());
        
        tracing::info!("📡 API Request Details:");
        tracing::info!("  Model: {}", model);
        tracing::info!("  Messages count: {}", messages.len());
        tracing::info!("  Tools count: {}", tools.as_ref().map(|t| t.len()).unwrap_or(0));
        tracing::info!("  Stream: true");
        
        // Debug log the tools being sent to identify potential issues
        if let Some(ref tools_list) = tools {
            tracing::info!("🔧 Tools being sent to LLM:");
            for (idx, tool) in tools_list.iter().enumerate() {
                tracing::info!("  Tool {}: {} - {:?}", 
                    idx + 1, 
                    &tool.function.name, 
                    &tool.function.description
                );
                
                // Log tool parameters schema (truncated for readability)
                if let Some(ref params) = tool.function.parameters {
                    let params_str = serde_json::to_string_pretty(params)
                        .unwrap_or_else(|_| "Failed to serialize".to_string());
                    let truncated = if params_str.len() > 500 {
                        format!("{}... [truncated]", &params_str[..500])
                    } else {
                        params_str
                    };
                    tracing::info!("    Parameters: {}", truncated);
                } else {
                    tracing::info!("    Parameters: None");
                }
            }
        }
        
        // Log first few characters of first message for debugging
        if let Some(first_msg) = messages.first() {
            match first_msg {
                ChatCompletionRequestMessage::System(sys_msg) => {
                    tracing::info!("  First message (System): {}...", 
                        match &sys_msg.content {
                            async_openai::types::ChatCompletionRequestSystemMessageContent::Text(text) => 
                                text.chars().take(100).collect::<String>(),
                            _ => "[Complex content]".to_string(),
                        }
                    );
                },
                ChatCompletionRequestMessage::User(user_msg) => {
                    tracing::info!("  First message (User): {}...", 
                        match &user_msg.content {
                            async_openai::types::ChatCompletionRequestUserMessageContent::Text(text) => 
                                text.chars().take(100).collect::<String>(),
                            _ => "[Complex content]".to_string(),
                        }
                    );
                },
                _ => tracing::info!("  First message: [Other type]"),
            }
        }
        
        let buffer = Arc::new(RwLock::new(VecDeque::new()));
        let senders = Arc::new(Mutex::new(Vec::new()));
        let is_complete = Arc::new(RwLock::new(false));
        let error = Arc::new(RwLock::new(None));
        let final_content = Arc::new(RwLock::new(None));
        let conversation_history = Arc::new(RwLock::new(messages.clone()));
        let tools_arc = Arc::new(RwLock::new(tools.clone()));
        let usage_data = Arc::new(RwLock::new(None));
        let model_used = Arc::new(RwLock::new(Some(model.clone())));
        
        let ongoing_state = OngoingPromptState {
            buffer: buffer.clone(),
            senders: senders.clone(),
            is_complete: is_complete.clone(),
            error: error.clone(),
            final_content: final_content.clone(),
            conversation_history: conversation_history.clone(),
            tools: tools_arc.clone(),
            usage_data: usage_data.clone(),
            model_used: model_used.clone(),
        };

        // Clone everything needed for the background task
        let state_clone = state.clone();
        let buffer_clone = buffer.clone();
        let senders_clone = senders.clone();
        let is_complete_clone = is_complete.clone();
        let error_clone = error.clone();
        let final_content_clone = final_content.clone();
        let conversation_history_clone = conversation_history.clone();
        let tools_clone = tools_arc.clone();
        let usage_data_clone = usage_data.clone();
        
        task::spawn(async move {
            let mut accumulated_content = String::new();
            let mut conversation_complete = false;
            let mut completion_error: Option<String> = None;

            tracing::info!("🔄 Starting enhanced stream processing with tool call support...");
            
            while !conversation_complete && completion_error.is_none() {
                // Get current conversation state
                let current_messages = {
                    let history = conversation_history_clone.read().await;
                    history.clone()
                };
                
                let current_tools = {
                    let tools_lock = tools_clone.read().await;
                    tools_lock.clone()
                };

                // Apply token limits to prevent excessive costs
                let truncated_messages = truncate_messages_to_token_limit(current_messages, &current_tools);

                // Create request for this iteration
                let request = CreateChatCompletionRequest {
                    model: model.clone(),
                    messages: truncated_messages,
                    tools: current_tools,
                    tool_choice: None,
                    stream: Some(true),
                    max_completion_tokens: Some(4000), // Limit output tokens to 4k to prevent excessive generation costs
                    ..Default::default()
                };

                tracing::info!("📞 Making API call for conversation turn...");
                let mut stream = match state_clone.workshop.client
                    .chat()
                    .create_stream(request)
                    .await
                {
                    Ok(stream) => stream,
                    Err(e) => {
                        tracing::error!("❌ Failed to create chat completion stream: {:?}", e);
                        completion_error = Some(e.to_string());
                        break;
                    }
                };

                let mut turn_content = String::new();
                let mut current_tool_call: Option<ChatCompletionMessageToolCall> = None;
                let mut chunk_count = 0;
                let mut tools_executed_this_turn = false;

                // Process the stream for this conversation turn
                while let Some(result) = stream.next().await {
                    chunk_count += 1;
                    
                    match result {
                        Ok(chunk) => {
                            // tracing::debug!("📦 Received chunk #{}: {:?}", chunk_count, chunk);
                            
                            // Capture usage data if present
                            if let Some(usage) = &chunk.usage {
                                let mut usage_lock = usage_data_clone.write().await;
                                *usage_lock = Some(usage.clone());
                                tracing::info!("💰 Captured usage data: prompt_tokens={}, completion_tokens={}, total_tokens={}", 
                                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                            }
                            
                            for choice in &chunk.choices {
                                // Handle content
                                if let Some(content) = &choice.delta.content {
                                    if !content.is_empty() {
                                        tracing::debug!("📝 Content from chunk #{}: '{}'", chunk_count, content);
                                        
                                        // Buffer the content
                                        {
                                            let mut buffer = buffer_clone.write().await;
                                            buffer.push_back(StreamingEntry {
                                                content: content.clone(),
                                                entry_type: StreamingEntryType::Content,
                                                tool_call: None,
                                            });
                                        }
                                        
                                        // Broadcast to all active streams
                                        {
                                            let mut senders_lock = senders_clone.lock().await;
                                            senders_lock.retain(|sender| {
                                                sender.try_send(Ok(StreamingEntry {
                                                    content: content.clone(),
                                                    entry_type: StreamingEntryType::Content,
                                                    tool_call: None,
                                                })).is_ok()
                                            });
                                        }
                                        
                                        turn_content.push_str(content);
                                        accumulated_content.push_str(content);
                                    }
                                }

                                // Handle tool calls - process them immediately as they complete
                                if let Some(ref tool_calls_chunk) = choice.delta.tool_calls {
                                    tracing::info!("🔧 TOOL CALL DETECTED in chunk #{}", chunk_count);
                                    for tool_call_chunk in tool_calls_chunk {
                                        if let Some(id) = &tool_call_chunk.id {
                                            // If we have a previous tool call that was being built, execute it now
                                            if let Some(completed_call) = current_tool_call.take() {
                                                tracing::info!("📋 EXECUTING COMPLETED TOOL CALL: {} with args: {}", 
                                                    completed_call.function.name, completed_call.function.arguments);
                                                
                                                // Execute the tool call immediately
                                                let tool_execution_result = Self::execute_tool_call(
                                                    &completed_call,
                                                    &state_clone,
                                                    &buffer_clone,
                                                    &senders_clone,
                                                    &conversation_history_clone
                                                ).await;
                                                
                                                if let Err(e) = tool_execution_result {
                                                    tracing::error!("❌ Tool execution failed: {}", e);
                                                } else {
                                                    tools_executed_this_turn = true;
                                                }
                                            }
                                            
                                            tracing::info!("🆕 NEW TOOL CALL STARTED: ID={}", id);
                                            current_tool_call = Some(ChatCompletionMessageToolCall {
                                                id: id.clone(),
                                                r#type: ChatCompletionToolType::Function,
                                                function: FunctionCall {
                                                    name: String::new(),
                                                    arguments: String::new(),
                                                },
                                            });
                                        }
                                        
                                        if let Some(ref mut call) = current_tool_call {
                                            if let Some(ref function) = tool_call_chunk.function {
                                                if let Some(ref name) = function.name {
                                                    call.function.name.push_str(name);
                                                    tracing::debug!("🔧 Tool name fragment: '{}'", name);
                                                }
                                                if let Some(ref args) = function.arguments {
                                                    call.function.arguments.push_str(args);
                                                    tracing::debug!("📝 Tool args fragment: '{}'", args);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Check for finish reason
                                if let Some(finish_reason) = &choice.finish_reason {
                                    tracing::info!("🏁 Turn finished with reason: {:?}", finish_reason);
                                    
                                    // Execute any remaining tool call
                                    if let Some(completed_call) = current_tool_call.take() {
                                        tracing::info!("📋 EXECUTING FINAL TOOL CALL: {} with args: {}", 
                                            completed_call.function.name, completed_call.function.arguments);
                                        
                                        let tool_execution_result = Self::execute_tool_call(
                                            &completed_call,
                                            &state_clone,
                                            &buffer_clone,
                                            &senders_clone,
                                            &conversation_history_clone
                                        ).await;
                                        
                                        if let Err(e) = tool_execution_result {
                                            tracing::error!("❌ Final tool execution failed: {}", e);
                                        } else {
                                            tools_executed_this_turn = true;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ Stream error on chunk #{}: {}", chunk_count, e);
                            
                            // Check if this is a tool call parsing error and we have a partial tool call
                            if e.to_string().contains("unknown variant") && e.to_string().contains("expected `function`") {
                                tracing::warn!("🔧 Detected malformed tool call response, attempting recovery...");
                                
                                // If we have a current tool call in progress, try to complete it
                                if let Some(completed_call) = current_tool_call.take() {
                                    if !completed_call.function.name.is_empty() {
                                        tracing::info!("🔄 RECOVERING TOOL CALL: {} with args: {}", 
                                            completed_call.function.name, completed_call.function.arguments);
                                        
                                        let tool_execution_result = Self::execute_tool_call(
                                            &completed_call,
                                            &state_clone,
                                            &buffer_clone,
                                            &senders_clone,
                                            &conversation_history_clone
                                        ).await;
                                        
                                        if let Err(e) = tool_execution_result {
                                            tracing::error!("❌ Recovery tool execution failed: {}", e);
                                        } else {
                                            tools_executed_this_turn = true;
                                        }
                                        
                                        // Continue processing instead of erroring out
                                        break;
                                    }
                                }
                            }
                            
                            completion_error = Some(e.to_string());
                            break;
                        }
                    }
                }

                // After processing the stream, check if we had any assistant content to add
                if !turn_content.is_empty() {
                    // Add assistant message with just content (tool calls are handled separately as they execute)
                    let mut history = conversation_history_clone.write().await;
                    history.push(ChatCompletionRequestMessage::Assistant(
                        ChatCompletionRequestAssistantMessage {
                            content: Some(ChatCompletionRequestAssistantMessageContent::Text(turn_content.clone())),
                            refusal: None,
                            name: None,
                            tool_calls: None,
                            function_call: None,
                            audio: None,
                        }
                    ));
                    tracing::info!("💾 Added assistant message with content to conversation");
                }

                // Check if conversation should continue based on whether we executed any tools
                // during this specific turn
                if tools_executed_this_turn {
                    tracing::info!("🔄 Continuing conversation after tool execution...");
                    continue;
                } else {
                    tracing::info!("🔚 No tools executed this turn - conversation complete");
                    conversation_complete = true;
                }
            }

            tracing::info!("🏁 Enhanced stream processing finished. Final content length: {}", accumulated_content.len());

            // Store final content
            {
                let mut final_content_lock = final_content_clone.write().await;
                *final_content_lock = Some(accumulated_content.clone());
                tracing::info!("💾 Stored final content: {} characters", accumulated_content.len());
            }

            // Store any error that occurred
            if let Some(err) = completion_error.clone() {
                let mut error_lock = error_clone.write().await;
                *error_lock = Some(err.clone());
                tracing::error!("💾 Stored error: {}", err);
            }

            // Mark as complete and close all senders
            {
                let mut complete = is_complete_clone.write().await;
                *complete = true;
                tracing::info!("✅ Marked prompt as complete");
            }
            
            // Close all remaining senders
            {
                let mut senders_lock = senders_clone.lock().await;
                let sender_count = senders_lock.len();
                senders_lock.clear();
                tracing::info!("📡 Closed {} remaining senders", sender_count);
            }

            if let Some(error) = completion_error {
                tracing::error!("❌ Enhanced chat completion finished with error: \"{}\"", error);
            } else {
                tracing::info!("✅ Enhanced chat completion finished successfully with data: \"{}\"", &accumulated_content[..accumulated_content.len().min(100)]);
            }
        });
            
        Ok(Self { 
            state: ongoing_state,
        })
    }
    
    /// Get a stream that starts from the beginning and includes all buffered chunks
    /// followed by any new chunks that arrive
    pub async fn get_stream(&self) -> impl Stream<Item = Result<StreamingEntry, String>> + Send + 'static {
        let buffer = self.state.buffer.clone();
        let senders = self.state.senders.clone();
        let is_complete = self.state.is_complete.clone();
        let error = self.state.error.clone();
        
        // Create a channel for this stream
        let (sender, receiver) = unbounded();
        
        // Add sender to the list
        {
            let mut senders_lock = senders.lock().await;
            senders_lock.push(sender);
        }
        
        // Create the stream
        let buffered_chunks = {
            let buffer_read = buffer.read().await;
            buffer_read.iter().cloned().collect::<Vec<_>>()
        };
        
        // Check if we have an error
        let current_error = {
            let error_read = error.read().await;
            error_read.clone()
        };
        
        // Check if complete
        let currently_complete = {
            let complete_read = is_complete.read().await;
            *complete_read
        };
        
        // Create the stream that first yields buffered chunks, then live chunks
        stream::iter(buffered_chunks.into_iter().map(Ok))
            .chain(
                if let Some(err) = current_error {
                    // If there's an error, yield it
                    stream::once(async { Err(err) }).boxed()
                } else if currently_complete {
                    // If complete, no more chunks
                    stream::empty().boxed()
                } else {
                    // Otherwise, yield from receiver
                    receiver.boxed()
                }
            )
    }
    
    /// Check if the prompt is complete
    pub async fn is_complete(&self) -> bool {
        *self.state.is_complete.read().await
    }
    
    /// Get any error that occurred
    pub async fn get_error(&self) -> Option<String> {
        self.state.error.read().await.clone()
    }
    
    /// Wait for the prompt to complete and return the final content
    pub async fn await_completion(&self) -> Result<String, String> {
        loop {
            {
                let is_complete = self.state.is_complete.read().await;
                if *is_complete {
                    break;
                }
            }
            
            // Small delay to avoid busy waiting
            task::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        // Check for errors first
        if let Some(error) = self.get_error().await {
            return Err(error);
        }
        
        // Return final content
        let final_content = self.state.final_content.read().await;
        match final_content.as_ref() {
            Some(content) => Ok(content.clone()),
            None => Err("No content available".to_string()),
        }
    }

    /// Get all streaming events that were collected during completion
    pub async fn get_all_events(&self) -> Vec<StreamingEntry> {
        let buffer = self.state.buffer.read().await;
        buffer.iter().cloned().collect()
    }

    /// Get the usage data captured from the API response
    pub async fn get_usage_data(&self) -> Option<async_openai::types::CompletionUsage> {
        let usage_lock = self.state.usage_data.read().await;
        usage_lock.clone()
    }

    /// Get the model used for this request
    pub async fn get_model_used(&self) -> Option<String> {
        let model_lock = self.state.model_used.read().await;
        model_lock.clone()
    }

    /// Execute a single tool call and handle streaming of results
    async fn execute_tool_call(
        tool_call: &ChatCompletionMessageToolCall,
        state: &AppState,
        buffer: &Arc<RwLock<VecDeque<StreamingEntry>>>,
        senders: &Arc<Mutex<Vec<Sender<Result<StreamingEntry, String>>>>>,
        conversation_history: &Arc<RwLock<Vec<ChatCompletionRequestMessage>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool_name = &tool_call.function.name;
        let tool_args = &tool_call.function.arguments;
        
        tracing::info!("🟢🟢🟢 EXECUTING TOOL: {} 🟢🟢🟢", tool_name);
        tracing::info!("🆔 Call ID: {}", tool_call.id);
        tracing::info!("📋 Args: {}", tool_args);
        
        // Stream tool call start to user
        let tool_start_entry = StreamingEntry {
            content: String::new(),
            entry_type: StreamingEntryType::ToolCallStart,
            tool_call: Some(ToolCallEntry {
                tool_name: tool_name.clone(),
                tool_id: tool_call.id.clone(),
                arguments: Some(tool_args.clone()),
                result: None,
                status: ToolCallStatus::Starting,
            }),
        };
        
        {
            let mut buffer_lock = buffer.write().await;
            buffer_lock.push_back(tool_start_entry.clone());
        }
        {
            let mut senders_lock = senders.lock().await;
            senders_lock.retain(|sender| {
                sender.try_send(Ok(tool_start_entry.clone())).is_ok()
            });
        }

        // Parse arguments and call the tool
        let tool_result = match serde_json::from_str(tool_args) {
            Ok(mut args_json) => {
                tracing::info!("✅ Tool arguments parsed successfully");
                
                // Normalize numeric arguments (convert string numbers to actual numbers)
                args_json = normalize_tool_arguments(tool_name, args_json);
                tracing::info!("🔢 Tool arguments after normalization: {}", args_json);
                
                // Send executing status
                let executing_entry = StreamingEntry {
                    content: String::new(),
                    entry_type: StreamingEntryType::ToolCallStart,
                    tool_call: Some(ToolCallEntry {
                        tool_name: tool_name.clone(),
                        tool_id: tool_call.id.clone(),
                        arguments: Some(tool_args.clone()),
                        result: None,
                        status: ToolCallStatus::Executing,
                    }),
                };
                
                {
                    let mut buffer_lock = buffer.write().await;
                    buffer_lock.push_back(executing_entry.clone());
                }
                {
                    let mut senders_lock = senders.lock().await;
                    senders_lock.retain(|sender| {
                        sender.try_send(Ok(executing_entry.clone())).is_ok()
                    });
                }
                
                match state.workshop.mcp_client.write().await.call_tool(tool_name, args_json).await {
                    Ok(response) => {
                        let content: String = response.content
                            .into_iter()
                            .filter_map(|c| c.text)
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        tracing::info!("✅ TOOL EXECUTION SUCCESS: {}", tool_name);
                        tracing::info!("📤 Tool result length: {} characters", content.len());
                        tracing::info!("📄 Tool result preview: {}...", 
                            content.chars().take(200).collect::<String>());
                        
                        // Send success result
                        let success_entry = StreamingEntry {
                            content: String::new(),
                            entry_type: StreamingEntryType::ToolCallResult,
                            tool_call: Some(ToolCallEntry {
                                tool_name: tool_name.clone(),
                                tool_id: tool_call.id.clone(),
                                arguments: Some(tool_args.clone()),
                                result: Some(content.clone()),
                                status: ToolCallStatus::Success,
                            }),
                        };
                        
                        {
                            let mut buffer_lock = buffer.write().await;
                            buffer_lock.push_back(success_entry.clone());
                        }
                        {
                            let mut senders_lock = senders.lock().await;
                            senders_lock.retain(|sender| {
                                sender.try_send(Ok(success_entry.clone())).is_ok()
                            });
                        }
                        
                        content
                    }
                    Err(e) => {
                        tracing::error!("❌ TOOL EXECUTION FAILED: {} - Error: {}", tool_name, e);
                        let error_msg = format!("Error executing tool {}: {}", tool_name, e);
                        
                        // Send error result
                        let error_entry = StreamingEntry {
                            content: String::new(),
                            entry_type: StreamingEntryType::ToolCallError,
                            tool_call: Some(ToolCallEntry {
                                tool_name: tool_name.clone(),
                                tool_id: tool_call.id.clone(),
                                arguments: Some(tool_args.clone()),
                                result: Some(error_msg.clone()),
                                status: ToolCallStatus::Error,
                            }),
                        };
                        
                        {
                            let mut buffer_lock = buffer.write().await;
                            buffer_lock.push_back(error_entry.clone());
                        }
                        {
                            let mut senders_lock = senders.lock().await;
                            senders_lock.retain(|sender| {
                                sender.try_send(Ok(error_entry.clone())).is_ok()
                            });
                        }
                        
                        error_msg
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ TOOL ARGS PARSE FAILED: {}", e);
                let error_msg = format!("Error parsing tool arguments: {}", e);
                
                // Send parse error
                let error_entry = StreamingEntry {
                    content: String::new(),
                    entry_type: StreamingEntryType::ToolCallError,
                    tool_call: Some(ToolCallEntry {
                        tool_name: tool_name.clone(),
                        tool_id: tool_call.id.clone(),
                        arguments: Some(tool_args.clone()),
                        result: Some(error_msg.clone()),
                        status: ToolCallStatus::Error,
                    }),
                };
                
                {
                    let mut buffer_lock = buffer.write().await;
                    buffer_lock.push_back(error_entry.clone());
                }
                {
                    let mut senders_lock = senders.lock().await;
                    senders_lock.retain(|sender| {
                        sender.try_send(Ok(error_entry.clone())).is_ok()
                    });
                }
                
                error_msg
            }
        };

        // Add assistant message with tool call to conversation history first
        {
            let mut history = conversation_history.write().await;
            history.push(ChatCompletionRequestMessage::Assistant(
                ChatCompletionRequestAssistantMessage {
                    content: None,
                    refusal: None,
                    name: None,
                    tool_calls: Some(vec![tool_call.clone()]),
                    function_call: None,
                    audio: None,
                }
            ));
            tracing::info!("💾 Added assistant message with tool call to conversation");
        }

        // Add tool result to conversation history
        {
            let mut history = conversation_history.write().await;
            history.push(ChatCompletionRequestMessage::Tool(
                ChatCompletionRequestToolMessage {
                    content: async_openai::types::ChatCompletionRequestToolMessageContent::Text(tool_result.clone()),
                    tool_call_id: tool_call.id.clone(),
                }
            ));
            tracing::info!("💾 Added tool result to conversation for call ID: {}", tool_call.id);
        }
        
        tracing::info!("🟢🟢🟢 TOOL EXECUTION COMPLETED: {} 🟢🟢🟢", tool_name);
        Ok(())
    }
}

/// Manager for ongoing prompts with request coalescing
pub struct OngoingPromptManager {
    prompts: Arc<RwLock<HashMap<String, OngoingPrompt>>>,
}

impl Default for OngoingPromptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OngoingPromptManager {
    pub fn new() -> Self {
        Self {
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get an existing prompt or create a new one with request coalescing
    /// If a prompt with the same key already exists, return the existing one
    /// Otherwise, create a new prompt and store it
    pub async fn get_or_create(
        &self,
        key: String,
        state: &AppState,
        messages: Vec<ChatCompletionRequestMessage>,
        tools: Option<Vec<ChatCompletionTool>>,
        model: Option<String>,
    ) -> Result<OngoingPrompt, Box<dyn std::error::Error + Send + Sync>> {
        // First check if we already have this prompt
        {
            let prompts = self.prompts.read().await;
            if let Some(existing) = prompts.get(&key) {
                tracing::info!("🔄 Returning existing prompt for key: {} (tools provided: {})", 
                    key, tools.as_ref().map(|t| t.len()).unwrap_or(0));
                return Ok(existing.clone());
            }
        }

        // Create new prompt
        tracing::info!("🆕 Creating new prompt for key: {} (tools provided: {})", 
            key, tools.as_ref().map(|t| t.len()).unwrap_or(0));
        let prompt = OngoingPrompt::new(state, messages, tools, model).await?;
        
        // Store it
        {
            let mut prompts = self.prompts.write().await;
            prompts.insert(key.clone(), prompt.clone());
        }
        
        tracing::info!("Stored ongoing prompt with key: {}", key);
        Ok(prompt)
    }

    /// Get an existing prompt
    pub async fn get(&self, key: &str) -> Option<OngoingPrompt> {
        let prompts = self.prompts.read().await;
        prompts.get(key).cloned()
    }

    /// List all prompt keys (for debugging)
    pub async fn list_keys(&self) -> Vec<String> {
        let prompts = self.prompts.read().await;
        prompts.keys().cloned().collect()
    }

    /// Remove a prompt
    pub async fn remove(&self, key: &str) -> Option<OngoingPrompt> {
        let mut prompts = self.prompts.write().await;
        prompts.remove(key)
    }

    /// Insert a prompt with an additional key (for system message access)
    pub async fn insert_additional_key(&self, key: String, prompt: OngoingPrompt) {
        let mut prompts = self.prompts.write().await;
        prompts.insert(key, prompt);
    }
}
