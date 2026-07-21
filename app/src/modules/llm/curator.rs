use std::time::Duration;

use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, CreateChatCompletionRequest, ResponseFormat,
};
use async_std::task;
use serde_json::json;

use crate::models::llm::{LlmMemory, LlmMemorySnapshot, LlmMemoryStaging, LlmRun, LlmRunDraft};
use crate::modules::llm::executor::{self, ToolDef};
use crate::modules::llm::recorder::RunRecorder;
use crate::modules::llm::tokens::truncate_messages_to_token_limit;
use crate::state::AppState;

pub const CURATOR_PROMPT: &str = include_str!("./prompts/curator.md");

const MAX_CURATOR_TOKENS: u32 = 16_000;
const RETRY_INTERVAL: Duration = Duration::from_secs(3600);

pub fn strip_code_fences(text: &str) -> String {
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

#[derive(Debug, thiserror::Error)]
pub enum CuratorError {
    #[error("LLM features are not configured")]
    Unconfigured,
    #[error("curator run failed: {0}")]
    Run(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn make_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "search_forum",
            description: "Full-text search across the forum to verify accuracy of memory entries.",
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
            name: "get_topic_summary",
            description: "Retrieve cached summary of a topic to verify facts.",
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
            description: "Fetch posts from a topic to verify specific claims.",
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
    ]
}

pub async fn run_curator(state: &AppState) -> Result<CuratorOutput, CuratorError> {
    let mut recorder = RunRecorder::new();
    let running_run = LlmRun::insert_running("curator", None, None, state)
        .await
        .ok();
    let run_id = running_run.as_ref().map(|r| r.run_id);
    match run_curator_inner(state, &mut recorder, run_id).await {
        Ok(output) => Ok(output),
        Err(e) => {
            if !matches!(e, CuratorError::Unconfigured) {
                recorder.note(format!("failed: {e}"));
                let usage = recorder.usage.clone();
                LlmRun::finalize(
                    run_id,
                    LlmRunDraft {
                        run_type: "curator",
                        prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
                        completion_tokens: (!usage.is_empty())
                            .then_some(usage.completion_tokens as i32),
                        reasoning_tokens: (usage.reasoning_tokens > 0)
                            .then_some(usage.reasoning_tokens as i32),
                        model_used: state.llm.as_ref().map(|l| l.curator_model()),
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

async fn run_curator_inner(
    state: &AppState,
    recorder: &mut RunRecorder,
    run_id: Option<uuid::Uuid>,
) -> Result<CuratorOutput, CuratorError> {
    let llm = state.llm.as_ref().ok_or(CuratorError::Unconfigured)?;

    let current_memory = LlmMemory::get_all(state).await.unwrap_or_default();
    let latest_snapshot = LlmMemorySnapshot::get_latest(state).await.unwrap_or_default();

    let recent_runs = sqlx::query_as!(
        LlmRun,
        "SELECT * FROM llm_runs ORDER BY created_at DESC LIMIT 20"
    )
    .fetch_all(&state.database.pool)
    .await
    .unwrap_or_default();

    let recent_summaries = sqlx::query_as!(
        crate::models::topics::TopicSummary,
        "SELECT * FROM topic_summaries ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&state.database.pool)
    .await
    .unwrap_or_default();

    let recent_digest = sqlx::query_as!(
        crate::models::digest::ActivityDigest,
        "SELECT * FROM activity_digests ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&state.database.pool)
    .await
    .unwrap_or_default();

    let staged_candidates = LlmMemoryStaging::recent(100, state)
        .await
        .unwrap_or_default();

    let snapshot_version = latest_snapshot.as_ref().map(|s| s.version).unwrap_or(0);

    let now = chrono::Utc::now();
    let run_started = now;
    let now_str = now.format("%Y-%m-%d %H:%M UTC").to_string();
    let curator_system = format!(
        "{CURATOR_PROMPT}\n\n## Current date\n\n{now_str}",
    );

    let memory_text = current_memory
        .iter()
        .map(|m| {
            let sources = m.sources.as_ref().and_then(|s| s.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let url = s.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        if url.is_empty() {
                            return None;
                        }
                        let reason = s
                            .get("reason")
                            .and_then(|r| r.as_str())
                            .map(str::trim)
                            .filter(|r| !r.is_empty());
                        match reason {
                            Some(r) => Some(format!("  - {url} — {r}")),
                            None => Some(format!("  - {url}")),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            match sources {
                Some(s) if !s.is_empty() => format!("- {}: {}\n{}", m.term, m.content, s),
                _ => format!("- {}: {}", m.term, m.content),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let runs_text = recent_runs
        .iter()
        .map(|r| {
            format!(
                "- {} ({}): {} tokens, {}ms, outcome={}",
                r.run_type,
                r.created_at.format("%Y-%m-%d %H:%M"),
                r.total_tokens.unwrap_or(0),
                r.duration_ms.unwrap_or(0),
                r.outcome
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summaries_text = recent_summaries
        .iter()
        .map(|s| {
            format!(
                "- topic {} ({}): {}",
                s.topic_id,
                s.discourse_id,
                s.summary_text.chars().take(200).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let digest_text = recent_digest
        .as_ref()
        .map(|d| d.digest_text.chars().take(500).collect::<String>())
        .unwrap_or_default();

    let staged_text = staged_candidates
        .iter()
        .map(|s| {
            let source = match (&s.source_discourse_id, s.source_topic_id) {
                (Some(did), Some(tid)) => {
                    let anchor = s
                        .source_post_number
                        .map(|p| format!("#p-{p}"))
                        .unwrap_or_default();
                    let reason = s
                        .link_reason
                        .as_deref()
                        .map(|r| format!(" — {r}"))
                        .unwrap_or_default();
                    format!(" (source: /t/{did}/{tid}{anchor}{reason})")
                }
                _ => String::new(),
            };
            format!("- {}: {}{}", s.term, s.content, source)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let tool_impls: Vec<&dyn executor::LlmTool> = vec![
        &executor::builtin::SearchForum,
        &executor::builtin::GetTopicSummary,
        &executor::builtin::GetPosts,
    ];
    let tool_defs = executor::filter_available_tools(make_tool_defs(), state);

    let payload = format!(
        r#"Current memory version: {version}

## Current memory entries
{memory}

## Staged candidates (awaiting your review)
{staged}

## Recent LLM runs
{runs}

## Recent summaries
{summaries}

## Latest digest
{digest}"#,
        version = snapshot_version,
        memory = if memory_text.is_empty() { "(none)".into() } else { memory_text },
        staged = if staged_text.is_empty() { "(none)".into() } else { staged_text },
        runs = runs_text,
        summaries = summaries_text,
        digest = digest_text,
    );

    let tool_messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: curator_system.clone().into(),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: payload.clone().into(),
            name: None,
        }),
    ];
    let tool_results = executor::run_tool_loop_with_messages(
        tool_messages,
        &tool_defs,
        &tool_impls,
        state,
        &llm.config,
        None,
        recorder,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("curator tool loop failed: {e}");
        recorder.note(format!("tool loop failed: {e}"));
        String::new()
    });

    let system_prompt = format!(
        "{}\n\n## Tool results\n\n{}\n\n## Current state\n\n{}",
        curator_system,
        if tool_results.is_empty() {
            "No tools called.".into()
        } else {
            tool_results
        },
        payload,
    );

    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: system_prompt.into(),
            name: None,
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: "Review the current state above and produce your evaluation.".into(),
            name: None,
        }),
    ];

    let request = CreateChatCompletionRequest {
        model: llm.curator_model(),
        messages: truncate_messages_to_token_limit(messages, llm.config.max_input_tokens),
        response_format: Some(ResponseFormat::JsonObject),
        max_completion_tokens: Some(MAX_CURATOR_TOKENS),
        ..Default::default()
    };

    let completion_started = std::time::Instant::now();
    let completion = llm
        .client
        .chat()
        .create(request)
        .await
        .map_err(|e| CuratorError::Run(e.to_string()))?;

    recorder.record_completion(
        "curator evaluation",
        &llm.curator_model(),
        completion.usage.as_ref(),
        completion_started.elapsed().as_millis() as u64,
    );

    let content = completion
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    let cleaned = strip_code_fences(&content);
    let (output, parsed_cleanly) = parse_curator_output(&cleaned);
    if !parsed_cleanly {
        recorder.note("output was not parseable JSON; no memory updates applied");
    }

    let mut update_count = 0usize;
    for update in &output.memory_updates {
        let sources = normalize_source_links(update.sources.as_ref());
        match LlmMemory::upsert(&update.term, &update.content, &sources, state).await {
            Ok(_) => {
                update_count += 1;
                recorder.note(format!("memory upsert: {}", update.term));
            }
            Err(e) => {
                tracing::error!(term = %update.term, "curator failed to upsert memory entry: {e}");
                recorder.note(format!("memory upsert failed for {}: {e}", update.term));
            }
        }
    }

    let mut removal_count = 0usize;
    for term in &output.memory_removals {
        match LlmMemory::delete_by_term(term, state).await {
            Ok(true) => {
                removal_count += 1;
                recorder.note(format!("memory removal: {term}"));
            }
            Ok(false) => {
                recorder.note(format!("memory removal skipped (not found): {term}"));
            }
            Err(e) => {
                tracing::error!(term = %term, "curator failed to remove memory entry: {e}");
                recorder.note(format!("memory removal failed for {term}: {e}"));
            }
        }
    }

    // Clear staged candidates that were evaluated during this run. Rows added
    // by summarizers after `run_started` survive and wait for the next run.
    let cleared = match LlmMemoryStaging::clear_before(run_started, state).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("failed to clear staging table: {e}");
            0
        }
    };

    // Create snapshot
    let new_version = snapshot_version + 1;
    let memory_snapshot = serde_json::to_value(
        LlmMemory::get_all(state).await.unwrap_or_default(),
    )
    .unwrap_or_default();

    let usage = recorder.usage.clone();
    LlmRun::finalize(
        run_id,
        LlmRunDraft {
            run_type: "curator",
            prompt_tokens: (!usage.is_empty()).then_some(usage.prompt_tokens as i32),
            completion_tokens: (!usage.is_empty()).then_some(usage.completion_tokens as i32),
            reasoning_tokens: (usage.reasoning_tokens > 0).then_some(usage.reasoning_tokens as i32),
            model_used: Some(llm.curator_model()),
            tool_calls: Some(recorder.tool_call_count()),
            tool_rounds: Some(recorder.rounds as i32),
            duration_ms: recorder.duration_ms(),
            outcome: "success",
            metadata: Some(json!({
                "memory_updates": update_count,
                "memory_removals": removal_count,
                "staged_evaluated": staged_candidates.len(),
                "staged_cleared": cleared,
                "snapshot_version": new_version,
                "parsed_cleanly": parsed_cleanly,
            })),
            trace: Some(recorder.trace_json()),
            ..Default::default()
        },
        state,
    )
    .await;

    let _ = LlmMemorySnapshot::create(
        new_version,
        &memory_snapshot,
        run_id,
        Some(&output.snapshot_summary),
        state,
    )
    .await;

    tracing::info!(
        "curator run complete: {update_count} memory updates, {removal_count} removals, snapshot version {new_version}"
    );

    Ok(output)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, poem_openapi::Object)]
pub struct MemoryUpdate {
    pub term: String,
    pub content: String,
    #[serde(default)]
    pub sources: Option<serde_json::Value>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, Clone, poem_openapi::Object)]
pub struct CuratorOutput {
    #[serde(default)]
    pub memory_updates: Vec<MemoryUpdate>,
    #[serde(default)]
    pub memory_removals: Vec<String>,
    #[serde(default)]
    pub snapshot_summary: String,
    #[serde(default)]
    pub action_log: String,
}

/// Canonicalizes a memory-link URL. Allowed forms: site-relative paths
/// (`/t/magicians/1234#p-56`), EIP/ERC shorthands ("EIP-1559" becomes the
/// eips.ethereum.org page), eips.ethereum.org URLs, the ethereum/EIPs and
/// ethereum/ERCs GitHub repos, and legacy "magicians/1234" topic refs.
/// Anything else is rejected so junk links never reach the glossary.
fn normalize_memory_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() {
        return None;
    }
    if url.starts_with('/') {
        return Some(url.to_string());
    }

    let lower = url.to_ascii_lowercase();
    if let Some(number) = lower.strip_prefix("eip-").or_else(|| lower.strip_prefix("erc-"))
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
    {
        return Some(format!("https://eips.ethereum.org/EIPS/eip-{number}"));
    }

    const ALLOWED_PREFIXES: [&str; 3] = [
        "https://eips.ethereum.org/",
        "https://github.com/ethereum/eips",
        "https://github.com/ethereum/ercs",
    ];
    if ALLOWED_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
        return Some(url.to_string());
    }

    let looks_like_topic_ref = !lower.starts_with("http")
        && url.split('/').count() == 2
        && url
            .split('/')
            .nth(1)
            .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()));
    if looks_like_topic_ref {
        return Some(format!("/t/{url}"));
    }

    None
}

/// Normalizes model-provided sources into `[{url, reason}]` links. Accepts
/// legacy plain strings and structured objects; drops entries whose URL isn't
/// an allowed form.
pub fn normalize_source_links(sources: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(serde_json::Value::Array(items)) = sources else {
        return json!([]);
    };

    let links: Vec<serde_json::Value> = items
        .iter()
        .filter_map(|item| match item {
            serde_json::Value::String(s) => {
                let url = normalize_memory_url(s)?;
                Some(json!({"url": url, "reason": null}))
            }
            serde_json::Value::Object(obj) => {
                let url = normalize_memory_url(obj.get("url")?.as_str()?)?;
                let reason = obj
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .map(str::trim)
                    .filter(|r| !r.is_empty());
                Some(json!({"url": url, "reason": reason}))
            }
            _ => None,
        })
        .collect();

    serde_json::Value::Array(links)
}

/// Parses curator output, tolerating prose around the JSON object. Returns the
/// parsed output and whether it parsed cleanly; on total failure the whole text
/// becomes the snapshot summary so nothing is lost, but that is a bug signal.
fn parse_curator_output(cleaned: &str) -> (CuratorOutput, bool) {
    match serde_json::from_str::<CuratorOutput>(cleaned) {
        Ok(output) => (output, true),
        Err(direct_err) => {
            let embedded = cleaned.find('{').and_then(|start| {
                let end = cleaned.rfind('}')?;
                (end > start)
                    .then(|| serde_json::from_str::<CuratorOutput>(&cleaned[start..=end]).ok())
                    .flatten()
            });

            match embedded {
                Some(output) => {
                    tracing::warn!(
                        "curator output had prose around the JSON object; extracted embedded JSON"
                    );
                    (output, true)
                }
                None => {
                    tracing::error!(
                        excerpt = %cleaned.chars().take(300).collect::<String>(),
                        "curator output was not parseable JSON ({direct_err}); no memory updates applied"
                    );
                    (
                        CuratorOutput {
                            snapshot_summary: cleaned.to_string(),
                            ..Default::default()
                        },
                        false,
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_source_links, parse_curator_output};
    use serde_json::json;

    #[test]
    fn normalizes_link_forms() {
        let input = json!([
            "magicians/1234",
            "EIP-1559",
            {"url": "erc-20", "reason": "token standard"},
            {"url": "/t/research/19116#p-4", "reason": "the core idea"},
            {"url": "https://eips.ethereum.org/EIPS/eip-4844", "reason": null},
            {"url": "https://github.com/ethereum/ERCs/pull/123", "reason": "spec change"},
            {"url": "https://example.com/spam"},
            "not a link",
            "",
            42
        ]);
        let links = normalize_source_links(Some(&input));
        let links = links.as_array().unwrap();
        assert_eq!(links.len(), 6);
        assert_eq!(links[0]["url"], "/t/magicians/1234");
        assert_eq!(links[1]["url"], "https://eips.ethereum.org/EIPS/eip-1559");
        assert_eq!(links[2]["url"], "https://eips.ethereum.org/EIPS/eip-20");
        assert_eq!(links[2]["reason"], "token standard");
        assert_eq!(links[3]["url"], "/t/research/19116#p-4");
        assert_eq!(links[3]["reason"], "the core idea");
        assert_eq!(links[4]["url"], "https://eips.ethereum.org/EIPS/eip-4844");
        assert_eq!(links[5]["url"], "https://github.com/ethereum/ERCs/pull/123");
    }

    #[test]
    fn empty_or_missing_sources_become_empty_array() {
        assert_eq!(normalize_source_links(None), json!([]));
        assert_eq!(normalize_source_links(Some(&json!("x"))), json!([]));
        assert_eq!(normalize_source_links(Some(&json!([]))), json!([]));
    }

    #[test]
    fn parses_clean_json() {
        let input = r#"{"memory_updates": [{"term": "EIP-1559", "content": "fee market change", "sources": ["magicians/1"]}], "snapshot_summary": "s", "action_log": "a"}"#;
        let (output, parsed) = parse_curator_output(input);
        assert!(parsed);
        assert_eq!(output.memory_updates.len(), 1);
        assert_eq!(output.memory_updates[0].term, "EIP-1559");
    }

    #[test]
    fn parses_json_with_missing_fields() {
        let input = r#"{"memory_updates": [{"term": "blob", "content": "data availability unit"}]}"#;
        let (output, parsed) = parse_curator_output(input);
        assert!(parsed);
        assert_eq!(output.memory_updates.len(), 1);
        assert_eq!(output.action_log, "");
    }

    #[test]
    fn extracts_json_embedded_in_prose() {
        let input = "Here is my evaluation:\n{\"memory_updates\": [{\"term\": \"PeerDAS\", \"content\": \"peer data availability sampling\"}], \"snapshot_summary\": \"s\", \"action_log\": \"a\"}\nDone.";
        let (output, parsed) = parse_curator_output(input);
        assert!(parsed);
        assert_eq!(output.memory_updates.len(), 1);
        assert_eq!(output.memory_updates[0].term, "PeerDAS");
    }

    #[test]
    fn falls_back_to_snapshot_summary_on_unparseable_output() {
        let input = "I would like to store: EIP-7702 is an account abstraction proposal.";
        let (output, parsed) = parse_curator_output(input);
        assert!(!parsed);
        assert!(output.memory_updates.is_empty());
        assert_eq!(output.snapshot_summary, input);
    }
}

pub async fn run_periodically(state: AppState) {
    let interval_hours = state
        .llm
        .as_ref()
        .map(|llm| llm.config.curator_interval_hours)
        .unwrap_or(24);

    loop {
        let wait = Duration::from_secs((interval_hours * 3600) as u64);
        tracing::info!("next curator run in {interval_hours}h");
        task::sleep(wait).await;

        match run_curator(&state).await {
            Ok(output) => {
                tracing::info!(
                    "curator run succeeded: {} memory updates",
                    output.memory_updates.len(),
                );
            }
            Err(CuratorError::Unconfigured) => {
                tracing::info!("LLM not configured, curator disabled");
                return;
            }
            Err(e) => {
                tracing::error!("curator run failed: {e}");
                task::sleep(RETRY_INTERVAL).await;
            }
        }
    }
}
