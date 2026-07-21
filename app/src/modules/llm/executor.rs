use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent,
    ChatCompletionTool, ChatCompletionToolChoiceOption, ChatCompletionToolType,
    CreateChatCompletionRequest, FunctionObject,
};
use serde_json::Value;

use crate::modules::llm::recorder::RunRecorder;
use crate::modules::llm::streams::{SharedStream, ToolCallUpdate};
use crate::state::AppState;

#[derive(Clone)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

pub struct ToolResult {
    pub name: String,
    pub tool_call_id: String,
    pub output: String,
}

impl ToolDef {
    pub fn to_openai_tool(&self) -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: FunctionObject {
                name: self.name.into(),
                description: Some(self.description.into()),
                parameters: Some(self.parameters.clone()),
                strict: Some(false),
            },
        }
    }
}

/// Drops tool definitions whose backing services aren't configured, so models
/// are never offered tools that can only fail.
pub fn filter_available_tools(mut defs: Vec<ToolDef>, state: &AppState) -> Vec<ToolDef> {
    if state.meili.is_none() {
        defs.retain(|def| def.name != "search_forum");
    }
    defs
}

/// Human-readable description of a tool invocation, derived from its real arguments.
pub fn describe_tool_call(name: &str, args: &Value) -> String {
    let topic = || {
        args["topic_id"]
            .as_i64()
            .map(|id| format!("topic {id}"))
            .unwrap_or_else(|| "a topic".to_string())
    };
    match name {
        "search_forum" => match args["query"].as_str() {
            Some(query) => format!("Searching the forum for \u{201c}{query}\u{201d}"),
            None => "Searching the forum".to_string(),
        },
        "get_topic_summary" => format!("Recalling the summary of {}", topic()),
        "get_topic_overview" => format!("Looking up {}", topic()),
        "get_posts" => match (args["from_post"].as_i64(), args["to_post"].as_i64()) {
            (Some(from), Some(to)) => format!("Reading posts {from}\u{2013}{to} of {}", topic()),
            _ => format!("Reading posts of {}", topic()),
        },
        "note_candidate" => match args["term"].as_str() {
            Some(term) => format!("Noting \u{201c}{term}\u{201d} in the shared glossary"),
            None => "Noting a term in the shared glossary".to_string(),
        },
        _ => format!("Running {name}"),
    }
}

/// Short outcome line derived from a tool's real output, e.g. result counts.
fn describe_tool_result(name: &str, output: &str) -> Option<String> {
    match name {
        "search_forum" | "get_posts" => {
            let count = serde_json::from_str::<Value>(output)
                .ok()?
                .as_array()?
                .len();
            let noun = if name == "search_forum" { "result" } else { "post" };
            let plural = if count == 1 { "" } else { "s" };
            Some(format!("{count} {noun}{plural}"))
        }
        "get_topic_overview" => {
            let parsed = serde_json::from_str::<Value>(output).ok()?;
            let title = parsed["title"].as_str()?;
            Some(format!("\u{201c}{title}\u{201d}"))
        }
        "note_candidate" => Some("staged".to_string()),
        _ => None,
    }
}

pub async fn run_tool_loop(
    system_prompt: &str,
    tool_defs: &[ToolDef],
    tool_impls: &[&dyn LlmTool],
    state: &AppState,
    config: &super::LlmConfig,
    events: Option<&SharedStream>,
    recorder: &mut RunRecorder,
) -> Result<String, String> {
    let messages = vec![ChatCompletionRequestMessage::System(
        ChatCompletionRequestSystemMessage {
            content: system_prompt.to_string().into(),
            name: None,
        },
    )];
    run_tool_loop_with_messages(messages, tool_defs, tool_impls, state, config, events, recorder)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_tool_loop_with_messages(
    mut messages: Vec<ChatCompletionRequestMessage>,
    tool_defs: &[ToolDef],
    tool_impls: &[&dyn LlmTool],
    state: &AppState,
    config: &super::LlmConfig,
    events: Option<&SharedStream>,
    recorder: &mut RunRecorder,
) -> Result<String, String> {
    let model = config.model.clone();
    let client = &state.llm.as_ref().ok_or("LLM not configured")?.client;

    let tools: Vec<ChatCompletionTool> = tool_defs.iter().map(|t| t.to_openai_tool()).collect();

    let max_rounds = config.max_tool_rounds.max(1);
    let max_tool_calls_per_round = config.max_tool_calls.max(1) as usize;

    for round in 0..max_rounds {
        let request = CreateChatCompletionRequest {
            model: model.clone(),
            messages: messages.clone(),
            tools: Some(tools.clone()),
            tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
            max_completion_tokens: Some(8000),
            ..Default::default()
        };

        let completion_started = std::time::Instant::now();
        let completion = client
            .chat()
            .create(request)
            .await
            .map_err(|e| {
                recorder.note(format!("tool round {} failed: {e}", round + 1));
                format!("completion failed: {e}")
            })?;

        recorder.rounds += 1;
        recorder.record_completion(
            &format!("tool round {}", round + 1),
            &model,
            completion.usage.as_ref(),
            completion_started.elapsed().as_millis() as u64,
        );

        let choice = completion
            .choices
            .first()
            .ok_or("no choices returned")?
            .clone();

        let msg = choice.message;

        if let Some(tool_calls) = msg.tool_calls {
            let mut tool_results = Vec::new();
            let batch: Vec<ChatCompletionMessageToolCall> = tool_calls
                .into_iter()
                .take(max_tool_calls_per_round)
                .collect();

            for tc in &batch {
                let fname = &tc.function.name;
                let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                let label = describe_tool_call(fname, &args);

                if let Some(events) = events {
                    events
                        .publish_tool_call(ToolCallUpdate {
                            call_id: tc.id.clone(),
                            tool: fname.clone(),
                            label: label.clone(),
                            status: "running".to_string(),
                            detail: None,
                        })
                        .await;
                }

                let tool = tool_impls.iter().find(|t| t.name() == fname);
                let call_started = std::time::Instant::now();
                let result = match tool {
                    Some(t) => t.call(args.clone(), state).await,
                    None => Err(format!("unknown tool: {fname}")),
                };
                let call_duration_ms = call_started.elapsed().as_millis() as u64;

                let (status, detail, output) = match &result {
                    Ok(out) => ("ok".to_string(), describe_tool_result(fname, out), Some(out.as_str())),
                    Err(error) => ("error".to_string(), Some(error.clone()), None),
                };

                recorder.record_tool_call(
                    fname,
                    &label,
                    &status,
                    detail.as_deref(),
                    output,
                    &args,
                    call_duration_ms,
                );

                if let Some(events) = events {
                    events
                        .publish_tool_call(ToolCallUpdate {
                            call_id: tc.id.clone(),
                            tool: fname.clone(),
                            label,
                            status,
                            detail,
                        })
                        .await;
                }

                tool_results.push(ToolResult {
                    name: fname.clone(),
                    tool_call_id: tc.id.clone(),
                    output: result.unwrap_or_else(|e| e),
                });
            }

            messages.push(ChatCompletionRequestMessage::Assistant(
                ChatCompletionRequestAssistantMessage {
                    content: None,
                    refusal: None,
                    name: None,
                    audio: None,
                    tool_calls: Some(batch),
                    ..Default::default()
                },
            ));

            for tr in tool_results {
                messages.push(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessage {
                        content: ChatCompletionRequestToolMessageContent::Text(tr.output),
                        tool_call_id: tr.tool_call_id,
                    },
                ));
            }

            continue;
        }

        return match msg.content {
            Some(text) if !text.trim().is_empty() => Ok(text),
            _ => Err("model returned no content after tool loop".into()),
        };
    }

    Err("tool loop exceeded max rounds".into())
}

#[async_trait::async_trait]
pub trait LlmTool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Value;
    async fn call(&self, args: Value, state: &AppState) -> Result<String, String>;
}

pub(crate) mod builtin {
    use serde_json::{json, Value};

    use super::LlmTool;
    use crate::{
        models::topics::{post::Post, Topic},
        modules::discourse::ForumSearchDocument,
        state::AppState,
    };

    pub struct GetTopicSummary;

    #[async_trait::async_trait]
    impl LlmTool for GetTopicSummary {
        fn name(&self) -> &'static str { "get_topic_summary" }
        fn description(&self) -> &'static str {
            "Retrieve the cached AI-generated summary of a forum topic."
        }
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string", "description": "The discourse instance ID"},
                    "topic_id": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id"]
            })
        }
        async fn call(&self, args: Value, state: &AppState) -> Result<String, String> {
            let did = args["discourse_id"].as_str().ok_or("missing discourse_id")?;
            let tid = args["topic_id"].as_i64().ok_or("missing topic_id")? as i32;
            Topic::get_summary_by_topic_id(did, tid, state)
                .await
                .map(|s| s.summary_text)
                .map_err(|e| format!("summary not available: {e}"))
        }
    }

    pub struct GetTopicOverview;

    #[async_trait::async_trait]
    impl LlmTool for GetTopicOverview {
        fn name(&self) -> &'static str { "get_topic_overview" }
        fn description(&self) -> &'static str {
            "Get metadata and the first post of a topic."
        }
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string"},
                    "topic_id": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id"]
            })
        }
        async fn call(&self, args: Value, state: &AppState) -> Result<String, String> {
            let did = args["discourse_id"].as_str().ok_or("missing discourse_id")?;
            let tid = args["topic_id"].as_i64().ok_or("missing topic_id")? as i32;
            let topic = Topic::get_by_topic_id(did, tid, state).await.map_err(|e| format!("topic not found: {e}"))?;
            let first_post = topic.get_first_post(state).await.ok();
            Ok(json!({
                "title": topic.title,
                "slug": topic.slug,
                "post_count": topic.post_count,
                "view_count": topic.view_count,
                "created_at": topic.created_at,
                "last_post_at": topic.last_post_at,
                "first_post_excerpt": first_post.as_ref().and_then(|p| p.cooked.as_deref()).map(|c| {
                    strip_tags::strip_tags(c).chars().take(1000).collect::<String>()
                }),
            }).to_string())
        }
    }

    pub struct GetPosts;

    #[async_trait::async_trait]
    impl LlmTool for GetPosts {
        fn name(&self) -> &'static str { "get_posts" }
        fn description(&self) -> &'static str {
            "Fetch posts from a topic within a range."
        }
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "discourse_id": {"type": "string"},
                    "topic_id": {"type": "integer"},
                    "from_post": {"type": "integer"},
                    "to_post": {"type": "integer"}
                },
                "required": ["discourse_id", "topic_id", "from_post", "to_post"]
            })
        }
        async fn call(&self, args: Value, state: &AppState) -> Result<String, String> {
            let did = args["discourse_id"].as_str().ok_or("missing discourse_id")?;
            let tid = args["topic_id"].as_i64().ok_or("missing topic_id")? as i32;
            let from = args["from_post"].as_i64().ok_or("missing from_post")? as i32;
            let to = args["to_post"].as_i64().ok_or("missing to_post")? as i32;
            let posts = Post::find_by_post_number_range(did, tid, from, to, state)
                .await
                .map_err(|e| format!("failed to fetch posts: {e}"))?;
            Ok(json!(posts.iter().map(|p| json!({
                "post_number": p.post_number,
                "user_id": p.user_id,
                "created_at": p.created_at,
                "updated_at": p.updated_at,
                "excerpt": p.cooked.as_deref().map(|c| strip_tags::strip_tags(c).chars().take(800).collect::<String>()),
            })).collect::<Vec<_>>()).to_string())
        }
    }

    pub struct SearchForum;

    #[async_trait::async_trait]
    impl LlmTool for SearchForum {
        fn name(&self) -> &'static str { "search_forum" }
        fn description(&self) -> &'static str {
            "Full-text search across all forum content."
        }
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer"}
                },
                "required": ["query"]
            })
        }
        async fn call(&self, args: Value, state: &AppState) -> Result<String, String> {
            let query = args["query"].as_str().ok_or("missing query")?;
            let limit = args["limit"].as_i64().unwrap_or(10) as usize;
            let Some(meili) = &state.meili else { return Err("Meilisearch not configured".into()) };
            let results = meili.index("forum")
                .search().with_query(query).with_limit(limit)
                .execute::<ForumSearchDocument>()
                .await.map_err(|e| format!("search failed: {e}"))?;
            let docs: Vec<Value> = results.hits.into_iter().map(|hit| {
                let d = hit.result;
                json!({"entity_type": d.entity_type, "title": d.title, "discourse_id": d.discourse_id, "topic_id": d.topic_id, "excerpt": d.cooked})
            }).collect();
            Ok(json!(docs).to_string())
        }
    }

    pub struct NoteCandidate;

    #[async_trait::async_trait]
    impl LlmTool for NoteCandidate {
        fn name(&self) -> &'static str { "note_candidate" }
        fn description(&self) -> &'static str {
            "Propose a term and definition for the shared memory glossary. The curator reviews staged candidates before they enter the live glossary, so this will NOT appear in summarizer prompts immediately."
        }
        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "term": {"type": "string"},
                    "content": {"type": "string"},
                    "source_discourse_id": {"type": "string", "description": "Discourse instance of the source, e.g. \"magicians\""},
                    "source_topic_id": {"type": "integer"},
                    "source_post_number": {"type": "integer", "description": "Specific post that defines or best explains the term"},
                    "link_reason": {"type": "string", "description": "Why this link matters, e.g. \"the core idea\" or \"author's definition\""}
                },
                "required": ["term", "content"]
            })
        }
        async fn call(&self, args: Value, state: &AppState) -> Result<String, String> {
            let term = args["term"].as_str().ok_or("missing term")?;
            let content = args["content"].as_str().ok_or("missing content")?;
            let source_discourse_id = args["source_discourse_id"].as_str();
            let source_topic_id = args["source_topic_id"].as_i64().map(|i| i as i32);
            let source_post_number = args["source_post_number"].as_i64().map(|i| i as i32);
            let link_reason = args["link_reason"].as_str();

            crate::models::llm::LlmMemoryStaging::insert(
                term,
                content,
                source_discourse_id,
                source_topic_id,
                source_post_number,
                link_reason,
                state,
            )
            .await
            .map_err(|e| format!("failed to stage memory candidate: {e}"))?;
            Ok(format!("staged '{term}' for curator review"))
        }
    }
}
