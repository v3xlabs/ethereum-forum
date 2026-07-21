use std::time::Duration;

use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
};
use async_std::task;
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use strip_tags::strip_tags;

use crate::models::digest::ActivityDigest;
use crate::models::llm::{LlmRun, LlmRunDraft};
use crate::models::topics::{Topic, post::Post};
use crate::modules::llm::executor::{self, ToolDef};
use crate::modules::llm::recorder::RunRecorder;
use crate::modules::llm::tokens::truncate_messages_to_token_limit;
use crate::state::AppState;

pub const DIGEST_PROMPT: &str = include_str!("./prompts/digest.md");

const DIGEST_INTERVAL_HOURS: i64 = 12;
const DIGEST_PERIOD_DAYS: i64 = 3;
const MAX_POSTS_PER_TOPIC: i64 = 8;
const EXCERPT_MAX_CHARS: usize = 800;
const MAX_DIGEST_TOKENS: u32 = 16_000;
const RETRY_INTERVAL: Duration = Duration::from_secs(3600);

#[derive(Debug, thiserror::Error)]
pub enum DigestError {
    #[error("LLM features are not configured")]
    Unconfigured,
    #[error("no recent forum activity to digest")]
    NoRecentActivity,
    #[error("digest generation failed: {0}")]
    Generation(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn make_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_topic_summary",
            description: "Retrieve cached summary of a forum topic.",
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
            description: "Get metadata and first post of a topic.",
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

fn build_previous_digest_context(state: &AppState) -> String {
    let pool = &state.database.pool;
    let latest = task::block_on(async {
        sqlx::query_as!(
            ActivityDigest,
            "SELECT * FROM activity_digests ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or_default()
    });

    match latest {
        Some(digest) => format!(
            "\n\n## Previous digest\n\n{}\n\nUse the previous digest above to infer what is new. Do not re-list items already covered.",
            digest.digest_text
        ),
        None => String::new(),
    }
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

pub async fn generate_and_store(state: &AppState) -> Result<ActivityDigest, DigestError> {
    let mut recorder = RunRecorder::new();
    let running_run = LlmRun::insert_running("digest", None, None, state)
        .await
        .ok();
    let run_id = running_run.as_ref().map(|r| r.run_id);
    match generate_and_store_inner(state, &mut recorder, run_id).await {
        Ok(digest) => Ok(digest),
        Err(e) => {
            if !matches!(e, DigestError::Unconfigured | DigestError::NoRecentActivity) {
                recorder.note(format!("failed: {e}"));
                let usage = recorder.usage.clone();
                LlmRun::finalize(
                    run_id,
                    LlmRunDraft {
                        run_type: "digest",
                        prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
                        completion_tokens: (!usage.is_empty())
                            .then_some(usage.completion_tokens as i32),
                        reasoning_tokens: (usage.reasoning_tokens > 0)
                            .then_some(usage.reasoning_tokens as i32),
                        model_used: state.llm.as_ref().map(|l| l.digest_model()),
                        tool_calls: Some(recorder.tool_call_count()),
                        tool_rounds: Some(recorder.rounds as i32),
                        duration_ms: recorder.duration_ms(),
                        outcome: "failure",
                        error: Some(e.to_string()),
                        trace: Some(recorder.trace_json()),
                        ..Default::default()
                    },
                    state,
                )
                .await;
            } else if run_id.is_some() {
                let _ = sqlx::query!("DELETE FROM llm_runs WHERE run_id = $1 AND outcome = 'running'", run_id)
                    .execute(&state.database.pool)
                    .await;
            }
            Err(e)
        }
    }
}

async fn generate_and_store_inner(
    state: &AppState,
    recorder: &mut RunRecorder,
    run_id: Option<uuid::Uuid>,
) -> Result<ActivityDigest, DigestError> {
    let llm = state.llm.as_ref().ok_or(DigestError::Unconfigured)?;

    let period_end = Utc::now();
    let period_start = period_end - ChronoDuration::days(DIGEST_PERIOD_DAYS);

    let topics = Topic::get_digest_candidates(state).await?;
    if topics.is_empty() {
        return Err(DigestError::NoRecentActivity);
    }

    let mut topic_payloads = Vec::with_capacity(topics.len());
    for topic in &topics {
        let mut posts = Post::find_recent_by_topic(
            &topic.discourse_id,
            topic.topic_id,
            MAX_POSTS_PER_TOPIC,
            state,
        )
        .await?;
        posts.reverse();

        let recent_posts: Vec<serde_json::Value> = posts
            .iter()
            .map(|post| {
                let excerpt: String = post
                    .cooked
                    .as_deref()
                    .map(strip_tags)
                    .unwrap_or_default()
                    .chars()
                    .take(EXCERPT_MAX_CHARS)
                    .collect();

                json!({
                    "username": post.extra.as_ref().and_then(|extra| extra.get("username")),
                    "created_at": post.created_at,
                    "excerpt": excerpt,
                })
            })
            .collect();

        topic_payloads.push(json!({
            "discourse_id": topic.discourse_id,
            "topic_id": topic.topic_id,
            "title": topic.title,
            "view_count": topic.view_count,
            "post_count": topic.post_count,
            "last_post_at": topic.last_post_at,
            "recent_posts": recent_posts,
        }));
    }

    let previous_digest = build_previous_digest_context(state);
    let shared_memory = build_shared_memory_section(state);

    let system_prompt = format!(
        "{}{}{}",
        DIGEST_PROMPT,
        previous_digest,
        shared_memory,
    );

    let tool_impls: Vec<&dyn executor::LlmTool> = vec![
        &executor::builtin::GetTopicSummary,
        &executor::builtin::GetTopicOverview,
        &executor::builtin::SearchForum,
        &executor::builtin::NoteCandidate,
    ];
    let tool_defs = executor::filter_available_tools(make_tool_defs(), state);

    let tool_results = executor::run_tool_loop(
        &system_prompt,
        &tool_defs,
        &tool_impls,
        state,
        &llm.config,
        None,
        recorder,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("digest tool loop failed: {e}");
        recorder.note(format!("tool loop failed: {e}"));
        String::new()
    });

    let final_system = format!(
        "{}\n\n## Tool results\n\n{}",
        system_prompt,
        if tool_results.is_empty() {
            "No tools were called.".into()
        } else {
            tool_results
        }
    );

    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: final_system.into(),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: serde_json::to_string(&json!({
                "period_start": period_start,
                "period_end": period_end,
                "topics": topic_payloads,
            }))
            .map_err(|e| DigestError::Generation(e.to_string()))?
            .into(),
            name: None,
        }),
    ];

    let request = CreateChatCompletionRequest {
        model: llm.digest_model(),
        messages: truncate_messages_to_token_limit(messages, llm.config.max_input_tokens),
        max_completion_tokens: Some(MAX_DIGEST_TOKENS),
        ..Default::default()
    };

    let completion_started = std::time::Instant::now();
    let completion = llm
        .client
        .chat()
        .create(request)
        .await
        .map_err(|e| DigestError::Generation(e.to_string()))?;

    recorder.record_completion(
        "digest generation",
        &llm.digest_model(),
        completion.usage.as_ref(),
        completion_started.elapsed().as_millis() as u64,
    );

    let digest_text = completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| DigestError::Generation("model returned no content".to_string()))?;

    let topics_included = serde_json::Value::Array(
        topics
            .iter()
            .map(|topic| {
                json!({
                    "discourse_id": topic.discourse_id,
                    "topic_id": topic.topic_id,
                    "title": topic.title,
                    "slug": topic.slug,
                })
            })
            .collect(),
    );

    let digest = ActivityDigest::insert(
        period_start,
        period_end,
        &digest_text,
        topics_included,
        state,
    )
    .await?;

    let usage = recorder.usage.clone();
    LlmRun::finalize(
        run_id,
        LlmRunDraft {
            run_type: "digest",
            prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
            completion_tokens: (!usage.is_empty()).then_some(usage.completion_tokens as i32),
            reasoning_tokens: (usage.reasoning_tokens > 0).then_some(usage.reasoning_tokens as i32),
            model_used: Some(llm.digest_model()),
            tool_calls: Some(recorder.tool_call_count()),
            tool_rounds: Some(recorder.rounds as i32),
            duration_ms: recorder.duration_ms(),
            outcome: "success",
            trace: Some(recorder.trace_json()),
            ..Default::default()
        },
        state,
    )
    .await;

    tracing::info!(digest_id = digest.digest_id, "stored activity digest");
    Ok(digest)
}

pub async fn run_periodically(state: AppState) {
    loop {
        let due = match ActivityDigest::get_latest(&state).await {
            Ok(Some(latest)) => latest.created_at + ChronoDuration::hours(DIGEST_INTERVAL_HOURS),
            Ok(None) => Utc::now(),
            Err(e) => {
                tracing::error!("failed to read latest activity digest: {e}");
                task::sleep(RETRY_INTERVAL).await;
                continue;
            }
        };

        let wait = (due - Utc::now()).to_std().unwrap_or(Duration::ZERO);
        if !wait.is_zero() {
            tracing::info!("next activity digest due in {}s", wait.as_secs());
            task::sleep(wait).await;
        }

        match generate_and_store(&state).await {
            Ok(digest) => {
                tracing::info!(digest_id = digest.digest_id, "generated activity digest");
            }
            Err(DigestError::NoRecentActivity) => {
                tracing::info!("no recent forum activity; skipping digest generation");
                task::sleep(RETRY_INTERVAL).await;
            }
            Err(e) => {
                tracing::error!("activity digest generation failed: {e}");
                task::sleep(RETRY_INTERVAL).await;
            }
        }
    }
}
