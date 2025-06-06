use async_std::channel::{Sender, unbounded};
use async_std::sync::{Mutex, RwLock};
use async_std::task;
use futures::{Stream, StreamExt, stream};
use openai::Usage;
use openai::chat::{ChatCompletion, ChatCompletionDelta, ChatCompletionMessage};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tracing::info;

use crate::state::AppState;

pub const SUMMARY_PROMPT: &str = include_str!("./summary.md");
pub const SUMMARY_MODEL: &str = "mistralai/ministral-3b";

pub const WORKSHOP_PROMPT: &str = include_str!("./workshop.md");
pub const WORKSHOP_MODEL: &str = "google/gemini-flash-1.5-8b";

pub const SHORTSUM_PROMPT: &str = include_str!("./shortsum.md");
pub const SHORTSUM_MODEL: &str = "mistralai/mistral-7b-instruct:free";

// Shared state for an ongoing prompt
#[derive(Clone)]
pub struct OngoingPromptState {
    pub buffer: Arc<RwLock<VecDeque<ChatCompletionDelta>>>,
    pub senders: Arc<Mutex<Vec<Sender<Result<ChatCompletionDelta, String>>>>>,
    pub is_complete: Arc<RwLock<bool>>,
    pub error: Arc<RwLock<Option<String>>>,
    pub final_content: Arc<RwLock<Option<String>>>,
    pub usage: Arc<RwLock<Option<Usage>>>,
}

// Ongoing Prompt Struct
// this is used when a chatcompletion request is triggered and ongoing.
// it handles the request, buffering, and retransmitting the entire response before sending the rest of the stream
// it is stored in a hashmap for the sake of request coalescing
#[derive(Clone)]
pub struct OngoingPrompt {
    pub state: OngoingPromptState,
}

impl OngoingPrompt {
    pub async fn new(
        state: &AppState,
        messages: Vec<ChatCompletionMessage>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut chat_completion =
            ChatCompletion::builder(WORKSHOP_MODEL, messages.clone())
                .credentials(state.workshop.credentials.clone())
                .create_stream()
                .await?;

        let buffer = Arc::new(RwLock::new(VecDeque::new()));
        let senders = Arc::new(Mutex::new(Vec::new()));
        let is_complete = Arc::new(RwLock::new(false));
        let error = Arc::new(RwLock::new(None));
        let final_content = Arc::new(RwLock::new(None));

        let usage = Arc::new(RwLock::new(None));

        let ongoing_state = OngoingPromptState {
            buffer: buffer.clone(),
            senders: senders.clone(),
            is_complete: is_complete.clone(),
            error: error.clone(),
            final_content: final_content.clone(),
            usage: usage.clone(),
        };

        let buffer_clone = buffer.clone();
        let senders_clone = senders.clone();
        let is_complete_clone = is_complete.clone();
        let error_clone = error.clone();
        let final_content_clone = final_content.clone();
        let usage_clone = usage.clone();

        task::spawn(async move {
            let mut data = String::new();
            let completion_error: Option<String> = None;
            let mut usage_data: Option<Usage> = None;

            while let Some(chunk) = chat_completion.recv().await {
                // Buffer the chunk
                {
                    let mut buffer = buffer_clone.write().await;
                    buffer.push_back(chunk.clone());
                }

                // Broadcast to all active streams
                {
                    let mut senders_lock = senders_clone.lock().await;
                    senders_lock.retain(|sender| sender.try_send(Ok(chunk.clone())).is_ok());
                }

                // Accumulate content for final result
                if let Some(content) = chunk.choices.first().and_then(|c| c.delta.content.as_ref())
                {
                    data.push_str(content);
                }
                if let Some(u) = chunk.usage {
                    usage_data = Some(u);
                }
            }

            // Store final content
            {
                let mut final_content_lock = final_content_clone.write().await;
                *final_content_lock = Some(data.clone());
            }

            // Store usage info
            {
                let mut usage_lock = usage_clone.write().await;
                *usage_lock = usage_data;
            }

            // Store any error that occurred
            if let Some(err) = completion_error {
                let mut error_lock = error_clone.write().await;
                *error_lock = Some(err);
            }

            // Mark as complete and close all senders
            {
                let mut complete = is_complete_clone.write().await;
                *complete = true;
            }

            // Close all remaining senders
            {
                let mut senders_lock = senders_clone.lock().await;
                senders_lock.clear();
            }

            info!("Chat completion finished with data: {:?}", data);
        });

        Ok(Self {
            state: ongoing_state,
        })
    }

    /// Get a stream that starts from the beginning and includes all buffered chunks
    /// followed by any new chunks that arrive
    pub async fn get_stream(
        &self,
    ) -> impl Stream<Item = Result<ChatCompletionDelta, String>> + Send + 'static {
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
        stream::iter(buffered_chunks.into_iter().map(Ok)).chain(if let Some(err) = current_error {
            // If there's an error, yield it
            stream::once(async { Err(err) }).boxed()
        } else if currently_complete {
            // If complete, no more chunks
            stream::empty().boxed()
        } else {
            // Otherwise, yield from receiver
            receiver.boxed()
        })
    }

    /// Check if the prompt is complete
    pub async fn is_complete(&self) -> bool {
        *self.state.is_complete.read().await
    }

    /// Get any error that occurred
    pub async fn get_error(&self) -> Option<String> {
        self.state.error.read().await.clone()
    }

    /// Await completion and get the final content
    /// This method will block until the prompt is complete and return the accumulated content
    pub async fn await_completion(&self) -> Result<String, String> {
        // Wait for completion
        loop {
            if self.is_complete().await {
                break;
            }
            task::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Check for errors first
        if let Some(error) = self.get_error().await {
            return Err(error);
        }

        // Return the final content
        let final_content = self.state.final_content.read().await;
        Ok(final_content.clone().unwrap_or_default())
    }

    /// Get usage information if available
    pub async fn get_usage(&self) -> Option<Usage> {
        *self.state.usage.read().await
    }
}

/// Manager for handling request coalescing of ongoing prompts
pub struct OngoingPromptManager {
    prompts: Arc<RwLock<HashMap<String, OngoingPrompt>>>,
}

impl Default for OngoingPromptManager {
    fn default() -> Self {
        Self {
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl OngoingPromptManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an ongoing prompt with request coalescing
    /// If a prompt with the same key is already running, returns the existing one
    /// Otherwise creates a new one
    pub async fn get_or_create(
        &self,
        key: String,
        state: &AppState,
        messages: Vec<ChatCompletionMessage>,
    ) -> Result<OngoingPrompt, Box<dyn std::error::Error + Send + Sync>> {
        // First check if we already have this prompt
        {
            let prompts = self.prompts.read().await;
            if let Some(existing_prompt) = prompts.get(&key) {
                return Ok(existing_prompt.clone());
            }
        }

        // Create new prompt
        let new_prompt = OngoingPrompt::new(state, messages).await?;

        // Store it
        {
            let mut prompts = self.prompts.write().await;
            prompts.insert(key.clone(), new_prompt.clone());
        }

        tracing::info!("Stored ongoing prompt with key: {}", key);

        // Spawn cleanup task
        let prompts_clone = self.prompts.clone();
        let key_clone = key.clone();
        let prompt_clone = new_prompt.clone();

        task::spawn(async move {
            // Wait for completion
            loop {
                if prompt_clone.is_complete().await {
                    break;
                }
                task::sleep(std::time::Duration::from_millis(100)).await;
            }

            // Clean up after a delay to allow final stream requests
            task::sleep(std::time::Duration::from_secs(30)).await;

            let mut prompts = prompts_clone.write().await;
            prompts.remove(&key_clone);
            tracing::info!("Cleaned up completed prompt: {}", key_clone);
        });

        Ok(new_prompt)
    }

    /// Get an existing prompt if it exists
    pub async fn get(&self, key: &str) -> Option<OngoingPrompt> {
        let prompts = self.prompts.read().await;
        prompts.get(key).cloned()
    }

    /// Get all current prompts
    pub async fn list_keys(&self) -> Vec<String> {
        let prompts = self.prompts.read().await;
        prompts.keys().cloned().collect()
    }

    /// Remove a prompt manually
    pub async fn remove(&self, key: &str) -> Option<OngoingPrompt> {
        let mut prompts = self.prompts.write().await;
        prompts.remove(key)
    }

    /// Insert an existing prompt under an additional key
    pub async fn insert_additional_key(&self, key: String, prompt: OngoingPrompt) {
        let mut prompts = self.prompts.write().await;
        prompts.insert(key, prompt);
    }
}
