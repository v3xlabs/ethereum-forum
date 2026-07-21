use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionStreamOptions, CompletionUsage,
    CreateChatCompletionRequest, ResponseFormat,
};
use async_std::task;
use chrono::Utc;
use futures::StreamExt;
use serde_json::json;
use sqlx::query_as;

use crate::models::llm::{LlmRun, LlmRunDraft};
use crate::models::topics::post::{Post, SummaryPost};
use crate::models::topics::{Topic, TopicSummary};
use crate::modules::llm::executor::{self, ToolDef};
use crate::modules::llm::recorder::RunRecorder;
use crate::modules::llm::streams::SharedStream;
use crate::modules::llm::tokens::{estimate_tokens_in_text, truncate_messages_to_token_limit};
use crate::state::AppState;

pub const SUMMARY_PROMPT: &str = include_str!("./prompts/summary.md");

const COMPLETED_STREAM_GRACE: Duration = Duration::from_secs(30);
const PERSIST_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PERSIST_POLL_ATTEMPTS: usize = 50;
const MAX_SUMMARY_TOKENS: u32 = 16_000;
const MAX_SUMMARY_POSTS: i32 = 512;
const CHUNK_TOKEN_BUDGET: usize = 30_000;

#[derive(Debug, thiserror::Error)]
pub enum SummaryError {
    #[error("topic not found")]
    TopicNotFound,
    #[error("LLM features are not configured")]
    Unconfigured,
    #[error("summary generation failed: {0}")]
    Generation(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub fn summary_key(discourse_id: &str, topic_id: i32) -> String {
    format!("summary-{discourse_id}-{topic_id}")
}

fn based_on(topic: &Topic) -> chrono::DateTime<chrono::Utc> {
    topic.last_post_at.unwrap_or_else(Utc::now)
}

fn is_fresh(summary: &TopicSummary, topic: &Topic) -> bool {
    summary.based_on.timestamp() == based_on(topic).timestamp()
}

pub async fn latest_cached_summary(
    topic: &Topic,
    state: &AppState,
) -> Result<Option<TopicSummary>, sqlx::Error> {
    query_as!(
        TopicSummary,
        "SELECT * FROM topic_summaries WHERE discourse_id = $1 AND topic_id = $2 ORDER BY based_on DESC, created_at DESC LIMIT 1",
        topic.discourse_id,
        topic.topic_id
    )
    .fetch_optional(&state.database.pool)
    .await
}

pub async fn fresh_cached_summary(
    topic: &Topic,
    state: &AppState,
) -> Result<Option<TopicSummary>, sqlx::Error> {
    Ok(latest_cached_summary(topic, state)
        .await?
        .filter(|summary| is_fresh(summary, topic)))
}

fn make_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_topic_summary",
            description: "Retrieve the cached AI-generated summary of a forum topic.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string"},
                    "topic_id": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id"]
            }),
        },
        ToolDef {
            name: "get_topic_overview",
            description: "Get metadata and first post of a topic without all posts.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string"},
                    "topic_id": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id"]
            }),
        },
        ToolDef {
            name: "get_posts",
            description: "Fetch a range of posts from a topic by post number.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string"},
                    "topic_id": {"type": "integer"},
                    "from_post": {"type": "integer"},
                    "to_post": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id", "from_post", "to_post"]
            }),
        },
        ToolDef {
            name: "search_forum",
            description: "Full-text search across all forum content.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer"}
                },
                "required": ["query"]
            }),
        },
        ToolDef {
            name: "note_candidate",
            description: "Propose a term for the shared memory glossary, with a source link.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "term": {"type": "string"},
                    "content": {"type": "string"},
                    "source_discourse_id": {"type": "string"},
                    "source_topic_id": {"type": "integer"},
                    "source_post_number": {"type": "integer"},
                    "link_reason": {"type": "string", "description": "Why this link matters, e.g. \"the core idea\""}
                },
                "required": ["term", "content"]
            }),
        },
    ]
}

fn tool_impls() -> Vec<&'static dyn executor::LlmTool> {
    vec![
        &executor::builtin::GetTopicSummary,
        &executor::builtin::GetTopicOverview,
        &executor::builtin::GetPosts,
        &executor::builtin::SearchForum,
        &executor::builtin::NoteCandidate,
    ]
}

fn build_shared_memory_section(state: &AppState) -> String {
    let pool = &state.database.pool;
    let memory = task::block_on(async {
        sqlx::query_as!(
            crate::models::llm::LlmMemory,
            "SELECT * FROM llm_memory ORDER BY term ASC"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    });

    let budget = state
        .llm
        .as_ref()
        .map(|llm| llm.config.memory_token_budget)
        .unwrap_or(0);
    crate::modules::llm::render_memory_section(&memory, budget)
}

fn build_system_prompt(base: &str, state: &AppState) -> String {
    let memory_section = build_shared_memory_section(state);
    if memory_section.is_empty() {
        base.to_string()
    } else {
        format!("{base}{memory_section}")
    }
}

async fn chunk_posts(
    discourse_id: &str,
    topic_id: i32,
    start_post: i32,
    end_post: Option<i32>,
    state: &AppState,
) -> Result<Vec<Vec<SummaryPost>>, SummaryError> {
    let limit = end_post
        .map(|e| (e - start_post + 1).max(1) as i64)
        .unwrap_or(MAX_SUMMARY_POSTS as i64);
    let posts = Post::find_by_topic_id(discourse_id, topic_id, 1, Some(limit as i32), state)
        .await
        .map(|(p, _)| p)
        .unwrap_or_default();

    if posts.is_empty() {
        return Ok(vec![]);
    }

    let summary_posts: Vec<SummaryPost> = posts.into_iter().map(Into::into).collect();
    let mut chunks: Vec<Vec<SummaryPost>> = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_tokens = 0usize;

    for post in summary_posts {
        let post_text = serde_json::to_string(&post).unwrap_or_default();
        let tokens = estimate_tokens_in_text(&post_text) + 4;

        if current_tokens + tokens > CHUNK_TOKEN_BUDGET && !current_chunk.is_empty() {
            chunks.push(std::mem::take(&mut current_chunk));
            current_tokens = 0;
        }

        current_tokens += tokens;
        current_chunk.push(post);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    Ok(chunks)
}

/// Runs a streaming completion, forwarding content deltas to `stream` as they arrive.
async fn stream_completion(
    client: &Client<OpenAIConfig>,
    mut request: CreateChatCompletionRequest,
    stream: &SharedStream,
) -> Result<(String, Option<CompletionUsage>), String> {
    request.stream = Some(true);
    request.stream_options = Some(ChatCompletionStreamOptions {
        include_usage: true,
    });

    let mut upstream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|e| format!("generation failed: {e}"))?;

    let mut content = String::new();
    let mut final_usage: Option<CompletionUsage> = None;
    while let Some(chunk) = upstream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if let Some(usage) = &chunk.usage {
            final_usage = Some(usage.clone());
        }
        for choice in &chunk.choices {
            if let Some(delta) = &choice.delta.content
                && !delta.is_empty()
            {
                content.push_str(delta);
                stream.publish(delta.clone()).await;
            }
        }
    }

    Ok((content, final_usage))
}

/// Starts summary generation in the background, or coalesces onto an ongoing
/// run for the same topic. Returns the shared stream and whether a new run
/// was started. With `force`, the previous summary is ignored and the topic
/// is summarized from scratch.
pub async fn start_topic_summary(
    topic: &Topic,
    state: &AppState,
    force: bool,
) -> Result<(SharedStream, bool), SummaryError> {
    let llm = state.llm.as_ref().ok_or(SummaryError::Unconfigured)?;
    let key = summary_key(&topic.discourse_id, topic.topic_id);

    let (stream, created) = llm.streams.get_or_create(&key, SharedStream::default).await;
    if !created {
        return Ok((stream, false));
    }

    let topic = topic.clone();
    let state = state.clone();
    let spawned_stream = stream.clone();

    task::spawn(async move {
        let mut recorder = RunRecorder::new();
        let running_run = LlmRun::insert_running(
            "summary",
            Some(&topic.discourse_id),
            Some(topic.topic_id),
            &state,
        )
        .await
        .ok();
        let run_id = running_run.as_ref().map(|r| r.run_id);

        match generate_summary(&topic, &spawned_stream, &state, force, &mut recorder).await {
            Ok(content) => {
                persist_summary(topic, content, recorder, state, key, spawned_stream, run_id).await;
            }
            Err(e) => {
                tracing::error!(topic_id = topic.topic_id, "summary generation failed: {e}");
                recorder.note(format!("failed: {e}"));
                let usage = recorder.usage.clone();
                let draft = LlmRunDraft {
                    run_type: "summary",
                    discourse_id: Some(topic.discourse_id.clone()),
                    topic_id: Some(topic.topic_id),
                    prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
                    completion_tokens: (!usage.is_empty())
                        .then_some(usage.completion_tokens as i32),
                    reasoning_tokens: (usage.reasoning_tokens > 0)
                        .then_some(usage.reasoning_tokens as i32),
                    model_used: state.llm.as_ref().map(|l| l.config.model.clone()),
                    tool_calls: Some(recorder.tool_call_count()),
                    tool_rounds: Some(recorder.rounds as i32),
                    duration_ms: recorder.duration_ms(),
                    outcome: "failure",
                    error: Some(e.to_string()),
                    metadata: None,
                    trace: Some(recorder.trace_json()),
                };
                finalize_run(run_id, draft, &state).await;
                spawned_stream.finish(Err(e.to_string()), None).await;
                if let Some(llm) = &state.llm {
                    llm.streams.remove_if(&key, &spawned_stream).await;
                }
            }
        }
    });

    Ok((stream, true))
}

async fn generate_summary(
    topic: &Topic,
    stream: &SharedStream,
    state: &AppState,
    force: bool,
    recorder: &mut RunRecorder,
) -> Result<String, SummaryError> {
    let previous = if force {
        None
    } else {
        latest_cached_summary(topic, state).await?
    };
    let start_post_number = previous
        .as_ref()
        .and_then(|s| s.based_on_post_number)
        .unwrap_or(0);

    let chunks = chunk_posts(
        &topic.discourse_id,
        topic.topic_id,
        start_post_number + 1,
        None,
        state,
    )
    .await?;

    if chunks.is_empty() && previous.is_none() {
        return Err(SummaryError::Generation("no posts found".to_string()));
    }

    let base_prompt = format!(
        "{}\n\n## This thread\n\nThis thread lives on the \"{did}\" discourse instance as topic {tid}. User links MUST use the form [@username](/u/{did}/username) and post links the form /t/{did}/{tid}#p-{{post_number}}. Refer to users only by their username, never by numeric user ID; if a post has no username, refer to \"a community member\" with no link.",
        build_system_prompt(SUMMARY_PROMPT, state),
        did = topic.discourse_id,
        tid = topic.topic_id,
    );
    let system_prompt = if let Some(ref prev) = previous {
        format!(
            "{} {}\n\n## Previous summary\n\n{}\n\n## Previous structured data\n\n{}\n\nRevise the summary given the new posts.",
            base_prompt,
            "This is an incremental update. Update the overview, key_points, and open_questions, and include a changelog_entry for this window.",
            prev.summary_text,
            prev.summary_json.as_ref().map(|j| j.to_string()).unwrap_or_default(),
        )
    } else {
        base_prompt
    };

    let total_posts: usize = chunks.iter().map(|c| c.len()).sum();
    let use_fold = chunks.len() > 1 || chunks.first().map(|c| c.len()).unwrap_or(0) > 20;

    // Models occasionally emit a near-empty completion (e.g. bare "{}");
    // retry once with a stream reset rather than surfacing junk or silently
    // keeping the stale summary.
    let mut content = String::new();
    for attempt in 0..2 {
        if attempt > 0 {
            recorder.note("model returned invalid summary output; retrying");
            stream.publish_reset().await;
            stream
                .publish_tool_activity("Model returned an empty result — retrying".to_string())
                .await;
        }

        content = if use_fold {
            stream
                .publish_tool_activity(format!(
                    "Reading {total_posts} posts in {} sections",
                    chunks.len()
                ))
                .await;
            fold_chunks(topic, &system_prompt, chunks.clone(), stream, state, recorder).await?
        } else {
            run_with_tools(
                topic,
                &system_prompt,
                chunks.clone(),
                total_posts,
                stream,
                state,
                recorder,
            )
            .await?
        };

        if is_valid_summary_json(&content) {
            break;
        }
    }

    if !is_valid_summary_json(&content) {
        return Err(SummaryError::Generation(
            "model returned invalid summary output after retry".to_string(),
        ));
    }

    let usage = CompletionUsage {
        prompt_tokens: recorder.usage.prompt_tokens,
        completion_tokens: recorder.usage.completion_tokens,
        total_tokens: recorder.usage.total(),
        prompt_tokens_details: None,
        completion_tokens_details: None,
    };
    stream.finish(Ok(content.clone()), Some(usage)).await;
    Ok(content)
}

/// Small-update path: let the model gather context with real tools (streamed
/// to subscribers as they run), then stream the final summary.
async fn run_with_tools(
    topic: &Topic,
    system_prompt: &str,
    chunks: Vec<Vec<SummaryPost>>,
    total_posts: usize,
    stream: &SharedStream,
    state: &AppState,
    recorder: &mut RunRecorder,
) -> Result<String, SummaryError> {
    let llm = state.llm.as_ref().ok_or(SummaryError::Unconfigured)?;

    if total_posts > 0 {
        let plural = if total_posts == 1 { "" } else { "s" };
        stream
            .publish_tool_activity(format!("Reading {total_posts} new post{plural}"))
            .await;
    } else {
        stream
            .publish_tool_activity("Reviewing the existing summary".to_string())
            .await;
    }

    let posts_payload = chunks.first().map(|chunk| {
        serde_json::to_string(&json!({
            "topic_info": topic,
            "posts": chunk,
        }))
        .unwrap_or_default()
    });

    let mut messages = vec![ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: system_prompt.to_string().into(),
            name: None,
        },
    )];
    if let Some(payload) = &posts_payload {
        messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: payload.clone().into(),
                name: None,
            },
        ));
    }

    let tool_defs = executor::filter_available_tools(make_tool_defs(), state);
    let impls = tool_impls();
    let tool_calls_before = recorder.tool_call_count();
    let tool_loop_answer = executor::run_tool_loop_with_messages(
        messages,
        &tool_defs,
        &impls,
        state,
        &llm.config,
        Some(stream),
        recorder,
    )
    .await
    .unwrap_or_else(|e| format!("Tool loop note: {e}"));

    // When the model answered without using any tools, its answer already came
    // from the full prompt — re-sending the same ~30k-token prompt for a second
    // completion would double the cost for nothing.
    if recorder.tool_call_count() == tool_calls_before && is_valid_summary_json(&tool_loop_answer) {
        stream.publish(tool_loop_answer.clone()).await;
        return Ok(tool_loop_answer);
    }

    let tool_activity = tool_loop_answer;
    stream
        .publish_tool_activity("Writing the summary".to_string())
        .await;

    let mut messages = vec![ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: format!("{system_prompt}\n\n## Tool results\n\n{tool_activity}").into(),
            name: None,
        },
    )];
    if let Some(payload) = posts_payload {
        messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: payload.into(),
                name: None,
            },
        ));
    }

    let request = CreateChatCompletionRequest {
        model: llm.config.model.clone(),
        messages: truncate_messages_to_token_limit(messages, llm.config.max_input_tokens),
        max_completion_tokens: Some(MAX_SUMMARY_TOKENS),
        ..Default::default()
    };

    let started = std::time::Instant::now();
    let (content, usage) = stream_completion(&llm.client, request, stream)
        .await
        .map_err(SummaryError::Generation)?;
    recorder.record_completion(
        "final summary",
        &llm.config.model,
        usage.as_ref(),
        started.elapsed().as_millis() as u64,
    );
    Ok(content)
}

/// Long-topic path: fold post sections into an accumulated summary, streaming
/// the final section's output so subscribers watch the summary being written.
async fn fold_chunks(
    topic: &Topic,
    system_prompt: &str,
    chunks: Vec<Vec<SummaryPost>>,
    stream: &SharedStream,
    state: &AppState,
    recorder: &mut RunRecorder,
) -> Result<String, SummaryError> {
    let llm = state.llm.as_ref().ok_or(SummaryError::Unconfigured)?;
    let section_count = chunks.len();
    let mut accumulated_summary = String::new();

    for (i, chunk) in chunks.iter().enumerate() {
        stream
            .publish_tool_activity(format!(
                "Summarizing section {} of {section_count} ({} posts)",
                i + 1,
                chunk.len()
            ))
            .await;

        let update_prompt = if i == 0 {
            format!("{system_prompt}\n\nGenerate the initial summary for this thread based on the following posts.")
        } else {
            format!(
                "{system_prompt}\n\nRevise the following summary given the new batch of posts.\n\n## Current summary\n\n{accumulated_summary}\n\n## New posts"
            )
        };

        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: update_prompt.into(),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: serde_json::to_string(&json!({
                    "topic_info": topic,
                    "posts": chunk,
                }))
                .map_err(|e| SummaryError::Generation(e.to_string()))?
                .into(),
                name: None,
            }),
        ];

        let request = CreateChatCompletionRequest {
            model: llm.config.model.clone(),
            messages: truncate_messages_to_token_limit(messages, llm.config.max_input_tokens),
            response_format: Some(ResponseFormat::JsonObject),
            max_completion_tokens: Some(MAX_SUMMARY_TOKENS),
            ..Default::default()
        };

        let is_last = i == section_count - 1;
        let section_label = format!("fold section {}/{section_count}", i + 1);
        let started = std::time::Instant::now();

        if is_last {
            let (content, usage) = stream_completion(&llm.client, request, stream)
                .await
                .map_err(SummaryError::Generation)?;
            recorder.record_completion(
                &section_label,
                &llm.config.model,
                usage.as_ref(),
                started.elapsed().as_millis() as u64,
            );
            return Ok(content);
        }

        let completion = llm
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| SummaryError::Generation(e.to_string()))?;

        recorder.record_completion(
            &section_label,
            &llm.config.model,
            completion.usage.as_ref(),
            started.elapsed().as_millis() as u64,
        );

        accumulated_summary = completion
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
    }

    Err(SummaryError::Generation("no posts to summarize".to_string()))
}

fn is_valid_summary_json(text: &str) -> bool {
    parse_summary_json(text).is_some()
}

fn parse_summary_json(text: &str) -> Option<serde_json::Value> {
    let cleaned = strip_code_fences(text);
    let parsed = serde_json::from_str::<serde_json::Value>(&cleaned).ok()?;
    parsed
        .get("overview")
        .and_then(|overview| overview.as_str())
        .is_some_and(|overview| !overview.trim().is_empty())
        .then_some(parsed)
}

fn strip_code_fences(text: &str) -> String {
    let text = text.trim();
    if text.starts_with("```") {
        let first_newline = text.find('\n').unwrap_or(0);
        let after_fence = &text[first_newline + 1..];
        if after_fence.ends_with("```") {
            return after_fence[..after_fence.len() - 3].trim().to_string();
        }
        if let Some(idx) = after_fence.rfind("```") {
            return after_fence[..idx].trim().to_string();
        }
    }
    text.to_string()
}

async fn persist_summary(
    topic: Topic,
    summary_text: String,
    recorder: RunRecorder,
    state: AppState,
    key: String,
    stream: SharedStream,
    run_id: Option<uuid::Uuid>,
) {
    store_summary(&topic, &summary_text, recorder, &state, run_id).await;

    async_std::task::sleep(COMPLETED_STREAM_GRACE).await;
    if let Some(llm) = &state.llm {
        llm.streams.remove_if(&key, &stream).await;
    }
}

fn summary_run_draft(topic: &Topic, recorder: &RunRecorder, state: &AppState) -> LlmRunDraft {
    let usage = &recorder.usage;
    LlmRunDraft {
        run_type: "summary",
        discourse_id: Some(topic.discourse_id.clone()),
        topic_id: Some(topic.topic_id),
        prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
        completion_tokens: (!usage.is_empty()).then_some(usage.completion_tokens as i32),
        reasoning_tokens: (usage.reasoning_tokens > 0).then_some(usage.reasoning_tokens as i32),
        model_used: state.llm.as_ref().map(|l| l.config.model.clone()),
        tool_calls: Some(recorder.tool_call_count()),
        tool_rounds: Some(recorder.rounds as i32),
        duration_ms: recorder.duration_ms(),
        outcome: "success",
        error: None,
        metadata: None,
        trace: Some(recorder.trace_json()),
    }
}

/// Updates a running row if one was created, otherwise falls back to insert.
async fn finalize_run(run_id: Option<uuid::Uuid>, draft: LlmRunDraft, state: &AppState) {
    LlmRun::finalize(run_id, draft, state).await;
}

async fn store_summary(topic: &Topic, summary_text: &str, recorder: RunRecorder, state: &AppState, run_id: Option<uuid::Uuid>) {
    let cleaned = strip_code_fences(summary_text);

    // A truncated or non-JSON result must never be persisted as a summary or
    // recorded as success — that hides crashes and dropped upstream streams.
    let Some(parsed) = parse_summary_json(&cleaned) else {
        let reason = if cleaned.trim().is_empty() {
            "empty content".to_string()
        } else {
            format!(
                "summary output was not valid summary JSON ({} chars)",
                cleaned.len()
            )
        };
        tracing::error!(topic_id = topic.topic_id, "summary rejected: {reason}");
        finalize_run(
            run_id,
            LlmRunDraft {
                outcome: "failure",
                error: Some(reason),
                ..summary_run_draft(topic, &recorder, state)
            },
            state,
        )
        .await;
        return;
    };
    let new_post_number = topic.post_count;

    let result = sqlx::query!(
        "INSERT INTO topic_summaries (discourse_id, topic_id, based_on, based_on_post_number, summary_text, summary_json, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        topic.discourse_id, topic.topic_id, based_on(topic), new_post_number, cleaned, parsed
    )
    .execute(&state.database.pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!(topic_id = topic.topic_id, "stored topic summary");
            finalize_run(run_id, summary_run_draft(topic, &recorder, state), state).await;
        }
        Err(e) => {
            tracing::error!(topic_id = topic.topic_id, "failed to store summary: {e}");
            finalize_run(
                run_id,
                LlmRunDraft {
                    outcome: "failure",
                    error: Some(e.to_string()),
                    ..summary_run_draft(topic, &recorder, state)
                },
                state,
            )
            .await;
        }
    }
}

pub async fn get_or_generate_summary(
    discourse_id: &str,
    topic_id: i32,
    state: &AppState,
) -> Result<TopicSummary, SummaryError> {
    let topic = Topic::get_by_topic_id(discourse_id, topic_id, state)
        .await
        .map_err(|_| SummaryError::TopicNotFound)?;

    let stale = match latest_cached_summary(&topic, state).await? {
        Some(summary) if is_fresh(&summary, &topic) => return Ok(summary),
        cached => cached,
    };

    if state.llm.is_none() {
        return stale.ok_or(SummaryError::Unconfigured);
    }

    let (stream, _) = start_topic_summary(&topic, state, false).await?;
    let content = stream
        .await_completion()
        .await
        .map_err(SummaryError::Generation)?;
    if content.trim().is_empty() {
        return Err(SummaryError::Generation("model returned no content".to_string()));
    }

    let previous_id = stale.map(|summary| summary.summary_id);
    for _ in 0..PERSIST_POLL_ATTEMPTS {
        if let Some(summary) = latest_cached_summary(&topic, state).await?
            && Some(summary.summary_id) != previous_id
        {
            return Ok(summary);
        }
        task::sleep(PERSIST_POLL_INTERVAL).await;
    }

    Err(SummaryError::Generation(
        "summary completed but was not persisted".to_string(),
    ))
}
