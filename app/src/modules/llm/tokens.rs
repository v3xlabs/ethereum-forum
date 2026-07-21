use async_openai::types::{
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessageContent,
};

pub const fn default_max_input_tokens() -> usize {
    100_000
}

const TOKENS_PER_MESSAGE_OVERHEAD: usize = 4;
const TOKENS_PER_NAME: usize = 1;

pub fn estimate_tokens_in_text(text: &str) -> usize {
    (text.len() as f64 / 3.5).ceil() as usize
}

fn estimate_tokens_in_message(message: &ChatCompletionRequestMessage) -> usize {
    let mut tokens = TOKENS_PER_MESSAGE_OVERHEAD;

    match message {
        ChatCompletionRequestMessage::System(msg) => {
            tokens += match &msg.content {
                ChatCompletionRequestSystemMessageContent::Text(text) => {
                    estimate_tokens_in_text(text)
                }
                ChatCompletionRequestSystemMessageContent::Array(_) => 50,
            };
            if msg.name.is_some() {
                tokens += TOKENS_PER_NAME;
            }
        }
        ChatCompletionRequestMessage::User(msg) => {
            tokens += match &msg.content {
                ChatCompletionRequestUserMessageContent::Text(text) => {
                    estimate_tokens_in_text(text)
                }
                ChatCompletionRequestUserMessageContent::Array(_) => 50,
            };
            if msg.name.is_some() {
                tokens += TOKENS_PER_NAME;
            }
        }
        ChatCompletionRequestMessage::Assistant(msg) => {
            if let Some(ChatCompletionRequestAssistantMessageContent::Text(text)) = &msg.content {
                tokens += estimate_tokens_in_text(text);
            }
            if msg.name.is_some() {
                tokens += TOKENS_PER_NAME;
            }
        }
        _ => tokens += 50,
    }

    tokens
}

pub fn truncate_messages_to_token_limit(
    mut messages: Vec<ChatCompletionRequestMessage>,
    max_input_tokens: usize,
) -> Vec<ChatCompletionRequestMessage> {
    let mut total_tokens = 0;
    let mut kept = Vec::new();

    if matches!(
        messages.first(),
        Some(ChatCompletionRequestMessage::System(_))
    ) {
        let system = messages.remove(0);
        total_tokens += estimate_tokens_in_message(&system);
        kept.push(system);
    }

    let insert_at = kept.len();
    let mut truncated = 0usize;
    for message in messages.into_iter().rev() {
        let message_tokens = estimate_tokens_in_message(&message);
        if total_tokens + message_tokens <= max_input_tokens {
            total_tokens += message_tokens;
            kept.insert(insert_at, message);
        } else {
            truncated += 1;
        }
    }

    if truncated > 0 {
        tracing::warn!(
            "truncated {truncated} message(s) to stay under the {max_input_tokens}-token input limit (~{total_tokens} tokens kept)"
        );
    }

    kept
}
