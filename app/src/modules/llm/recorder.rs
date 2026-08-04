use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, ChatCompletionTool,
    CompletionUsage,
};
use serde_json::{Value, json};
use std::time::{Duration, Instant, SystemTime};

use crate::modules::telemetry::{
    Observation, ObservationKind, SpanId, TokenUsage, TraceContext, TraceSpec, truncate_chars,
};

/// Token usage accumulated across every completion a run makes (tool-loop
/// rounds, fold sections, and the final generation), so runs are billed for
/// what they actually consumed rather than just their last request.
#[derive(Debug, Default, Clone)]
pub struct RunUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: u32,
}

impl RunUsage {
    pub fn add(&mut self, usage: &CompletionUsage) {
        self.prompt_tokens += usage.prompt_tokens;
        self.completion_tokens += usage.completion_tokens;
        if let Some(details) = &usage.completion_tokens_details
            && let Some(reasoning) = details.reasoning_tokens
        {
            self.reasoning_tokens += reasoning;
        }
    }

    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// What a completion actually sent and received. Kept out of `llm_runs.trace`
/// and exported only to Langfuse, so Postgres never grows prompt-sized rows.
/// Both sides come from the `capture_*` helpers, which callers invoke before
/// the request is moved into the client.
pub struct CompletionPayload {
    pub input: Option<String>,
    pub output: Option<String>,
}

/// One tool invocation. Grouped into a struct because Langfuse needs the call
/// id alongside everything the persisted trace already recorded.
pub struct ToolCallRecord<'a> {
    pub tool: &'a str,
    pub span_id: SpanId,
    pub call_id: &'a str,
    pub label: &'a str,
    pub status: &'a str,
    pub detail: Option<&'a str>,
    pub output: Option<&'a str>,
    pub args: &'a Value,
    pub duration_ms: u64,
}

enum RunEvent {
    ToolCall {
        tool: String,
        span_id: SpanId,
        call_id: String,
        label: String,
        status: String,
        detail: Option<String>,
        output: Option<String>,
        args: Value,
        at_ms: u64,
        duration_ms: u64,
    },
    Completion {
        label: String,
        model: String,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        reasoning_tokens: u32,
        input: Option<String>,
        output: Option<String>,
        at_ms: u64,
        duration_ms: u64,
    },
    Note {
        label: String,
        at_ms: u64,
    },
}

/// Collects the chronological trace of an LLM run (tool calls, completions,
/// notes) plus accumulated usage, for persistence on the `llm_runs` row and
/// export as a Langfuse trace.
pub struct RunRecorder {
    started: Instant,
    /// `Instant` has no wall-clock mapping, and exported spans are backdated
    /// from this anchor.
    started_wall: SystemTime,
    max_payload_chars: usize,
    context: TraceContext,
    parent: Option<TraceContext>,
    pub usage: RunUsage,
    pub rounds: u32,
    events: Vec<RunEvent>,
}

impl RunRecorder {
    /// `max_payload_chars` of zero disables prompt/completion capture, which is
    /// what happens when Langfuse is not configured.
    pub fn new(max_payload_chars: usize) -> Self {
        Self {
            started: Instant::now(),
            started_wall: SystemTime::now(),
            max_payload_chars,
            context: TraceContext::new(),
            parent: None,
            usage: RunUsage::default(),
            rounds: 0,
            events: Vec::new(),
        }
    }

    /// Emits this run inside `parent`'s trace instead of starting its own.
    pub fn nested_under(mut self, parent: TraceContext) -> Self {
        self.parent = Some(parent);
        self
    }

    /// The span other runs attach to when nesting under this one. Available
    /// immediately, so work triggered mid-run can reference it.
    pub fn context(&self) -> TraceContext {
        self.context
    }

    fn at_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn duration_ms(&self) -> i32 {
        self.started.elapsed().as_millis() as i32
    }

    pub fn started_at(&self) -> SystemTime {
        self.started_wall
    }

    pub fn ended_at(&self) -> SystemTime {
        self.started_wall + self.started.elapsed()
    }

    fn capture(&self, text: &str) -> Option<String> {
        (self.max_payload_chars > 0).then(|| truncate_chars(text, self.max_payload_chars))
    }

    /// Serializes a request for export, before ownership moves into the client.
    /// Emits `{"messages": [...], "tools": [...]}` because that is the only
    /// shape Langfuse renders as a conversation *and* reads tool definitions
    /// from. Returns `None` when payload capture is disabled.
    pub fn capture_request(
        &self,
        messages: &[ChatCompletionRequestMessage],
        tools: Option<&[ChatCompletionTool]>,
    ) -> Option<String> {
        if self.max_payload_chars == 0 {
            return None;
        }
        let mut request = json!({ "messages": messages });
        if let Some(tools) = tools
            && !tools.is_empty()
        {
            request["tools"] = json!(tools);
        }
        Some(self.capture_value(request))
    }

    /// Serializes a response message, preserving any `tool_calls` array so
    /// Langfuse can render tool-call cards and populate its tool columns.
    pub fn capture_response(&self, message: &ChatCompletionResponseMessage) -> Option<String> {
        if self.max_payload_chars == 0 {
            return None;
        }
        Some(self.capture_value(json!(message)))
    }

    pub fn capture_text(&self, text: &str) -> Option<String> {
        self.capture(text)
    }

    /// Truncates the strings *inside* a payload rather than the serialized
    /// result. Cutting the rendered JSON leaves it unparseable, and Langfuse
    /// then silently drops to raw-text rendering instead of chat bubbles.
    fn capture_value(&self, mut value: Value) -> String {
        self.truncate_strings(&mut value);
        value.to_string()
    }

    fn truncate_strings(&self, value: &mut Value) {
        match value {
            Value::String(text) => {
                if text.chars().count() > self.max_payload_chars {
                    *text = truncate_chars(text, self.max_payload_chars);
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| self.truncate_strings(v)),
            Value::Object(entries) => {
                entries.values_mut().for_each(|v| self.truncate_strings(v))
            }
            _ => {}
        }
    }

    pub fn record_tool_call(&mut self, record: ToolCallRecord<'_>) {
        const MAX_TOOL_OUTPUT: usize = 2000;
        let at_ms = self.at_ms().saturating_sub(record.duration_ms);
        self.events.push(RunEvent::ToolCall {
            tool: record.tool.to_string(),
            span_id: record.span_id,
            call_id: record.call_id.to_string(),
            label: record.label.to_string(),
            status: record.status.to_string(),
            detail: record.detail.map(str::to_string),
            output: record.output.map(|o| truncate_chars(o, MAX_TOOL_OUTPUT)),
            args: record.args.clone(),
            at_ms,
            duration_ms: record.duration_ms,
        });
    }

    pub fn record_completion(
        &mut self,
        label: &str,
        model: &str,
        usage: Option<&CompletionUsage>,
        duration_ms: u64,
        payload: CompletionPayload,
    ) {
        if let Some(usage) = usage {
            self.usage.add(usage);
        }
        let at_ms = self.at_ms().saturating_sub(duration_ms);
        self.events.push(RunEvent::Completion {
            label: label.to_string(),
            model: model.to_string(),
            prompt_tokens: usage.map(|u| u.prompt_tokens),
            completion_tokens: usage.map(|u| u.completion_tokens),
            reasoning_tokens: usage
                .and_then(|u| u.completion_tokens_details.as_ref())
                .and_then(|d| d.reasoning_tokens)
                .unwrap_or(0),
            input: payload.input,
            output: payload.output,
            at_ms,
            duration_ms,
        });
    }

    pub fn note(&mut self, label: impl Into<String>) {
        self.events.push(RunEvent::Note {
            label: label.into(),
            at_ms: self.at_ms(),
        });
    }

    pub fn tool_call_count(&self) -> i32 {
        self.events
            .iter()
            .filter(|e| matches!(e, RunEvent::ToolCall { .. }))
            .count() as i32
    }

    /// The `llm_runs.trace` column. Deliberately excludes prompt and completion
    /// payloads.
    pub fn trace_json(&self) -> Value {
        Value::Array(
            self.events
                .iter()
                .map(|event| match event {
                    RunEvent::ToolCall {
                        tool,
                        label,
                        status,
                        detail,
                        output,
                        args,
                        at_ms,
                        duration_ms,
                        ..
                    } => json!({
                        "type": "tool_call",
                        "tool": tool,
                        "label": label,
                        "status": status,
                        "detail": detail,
                        "output": output,
                        "args": args,
                        "at_ms": at_ms,
                        "duration_ms": duration_ms,
                    }),
                    RunEvent::Completion {
                        label,
                        model,
                        prompt_tokens,
                        completion_tokens,
                        at_ms,
                        duration_ms,
                        ..
                    } => json!({
                        "type": "completion",
                        "label": label,
                        "model": model,
                        "prompt_tokens": prompt_tokens,
                        "completion_tokens": completion_tokens,
                        "at_ms": at_ms,
                        "duration_ms": duration_ms,
                    }),
                    RunEvent::Note { label, at_ms } => json!({
                        "type": "note",
                        "label": label,
                        "at_ms": at_ms,
                    }),
                })
                .collect::<Vec<_>>(),
        )
    }

    /// A Langfuse trace carrying this run's observations. Callers fill in the
    /// remaining fields with struct update syntax before emitting.
    pub fn trace_spec(&self, name: &str) -> TraceSpec {
        TraceSpec {
            context: self.context,
            parent: self.parent,
            name: name.to_string(),
            session_id: None,
            tags: Vec::new(),
            metadata: Vec::new(),
            start: self.started_at(),
            end: self.ended_at(),
            input: None,
            output: None,
            error: None,
            observations: self.observations(),
        }
    }

    /// The same events as Langfuse observations, backdated from the wall-clock
    /// anchor.
    fn observations(&self) -> Vec<Observation> {
        self.events
            .iter()
            .map(|event| match event {
                RunEvent::ToolCall {
                    tool,
                    span_id,
                    call_id,
                    label,
                    status,
                    detail,
                    output,
                    args,
                    at_ms,
                    duration_ms,
                } => {
                    let start = self.started_wall + Duration::from_millis(*at_ms);
                    Observation {
                        name: label.clone(),
                        span_id: Some(*span_id),
                        kind: ObservationKind::Tool,
                        start,
                        end: start + Duration::from_millis(*duration_ms),
                        input: Some(args.to_string()),
                        output: output.clone(),
                        model: None,
                        usage: None,
                        error: (status == "error")
                            .then(|| detail.clone().unwrap_or_else(|| tool.clone())),
                        tool_name: Some(tool.clone()),
                        tool_call_id: Some(call_id.clone()),
                    }
                }
                RunEvent::Completion {
                    label,
                    model,
                    prompt_tokens,
                    completion_tokens,
                    reasoning_tokens,
                    input,
                    output,
                    at_ms,
                    duration_ms,
                } => {
                    let start = self.started_wall + Duration::from_millis(*at_ms);
                    Observation {
                        name: label.clone(),
                        span_id: None,
                        kind: ObservationKind::Generation,
                        start,
                        end: start + Duration::from_millis(*duration_ms),
                        input: input.clone(),
                        output: output.clone(),
                        model: Some(model.clone()),
                        usage: Some(TokenUsage {
                            input: prompt_tokens.unwrap_or(0),
                            output: completion_tokens.unwrap_or(0),
                            reasoning: *reasoning_tokens,
                        }),
                        error: None,
                        tool_name: None,
                        tool_call_id: None,
                    }
                }
                RunEvent::Note { label, at_ms } => {
                    let start = self.started_wall + Duration::from_millis(*at_ms);
                    Observation {
                        name: label.clone(),
                        span_id: None,
                        kind: ObservationKind::Event,
                        start,
                        end: start,
                        input: None,
                        output: None,
                        model: None,
                        usage: None,
                        error: None,
                        tool_name: None,
                        tool_call_id: None,
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(prompt: u32, completion: u32) -> CompletionUsage {
        CompletionUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }
    }

    fn payload(recorder: &RunRecorder, output: Option<&str>) -> CompletionPayload {
        CompletionPayload {
            input: None,
            output: output.and_then(|o| recorder.capture_text(o)),
        }
    }

    fn tool_call<'a>(
        tool: &'a str,
        status: &'a str,
        detail: Option<&'a str>,
        output: Option<&'a str>,
        args: &'a Value,
        duration_ms: u64,
    ) -> ToolCallRecord<'a> {
        ToolCallRecord {
            tool,
            span_id: crate::modules::telemetry::TraceContext::new().span_id,
            call_id: "call_1",
            label: "Doing a thing",
            status,
            detail,
            output,
            args,
            duration_ms,
        }
    }

    #[test]
    fn accumulates_usage_across_completions() {
        let mut recorder = RunRecorder::new(0);
        let none = || CompletionPayload {
            input: None,
            output: None,
        };
        recorder.record_completion("tool round 1", "m", Some(&usage(100, 20)), 5, none());
        recorder.record_completion("final", "m", Some(&usage(200, 80)), 5, none());
        recorder.record_completion("no usage reported", "m", None, 5, none());
        assert_eq!(recorder.usage.prompt_tokens, 300);
        assert_eq!(recorder.usage.completion_tokens, 100);
        assert_eq!(recorder.usage.total(), 400);
    }

    #[test]
    fn trace_preserves_order_and_counts_tool_calls() {
        let mut recorder = RunRecorder::new(0);
        let search_args = json!({"query": "x"});
        let empty_args = json!({});
        recorder.record_tool_call(tool_call("search_forum", "ok", None, Some("3 results"), &search_args, 10));
        let p = payload(&recorder, None);
        recorder.record_completion("tool round 1", "m", Some(&usage(1, 1)), 5, p);
        recorder.record_tool_call(tool_call("get_posts", "error", Some("boom"), None, &empty_args, 10));
        recorder.note("done");
        let trace = recorder.trace_json();
        let events = trace.as_array().unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0]["type"], "tool_call");
        assert_eq!(events[1]["type"], "completion");
        assert_eq!(events[2]["status"], "error");
        assert_eq!(events[3]["type"], "note");
        assert_eq!(recorder.tool_call_count(), 2);
    }

    #[test]
    fn payloads_never_reach_the_persisted_trace() {
        let mut recorder = RunRecorder::new(1000);
        let p = payload(&recorder, Some("the model said something"));
        recorder.record_completion("final", "m", Some(&usage(1, 1)), 5, p);
        let serialized = recorder.trace_json().to_string();
        assert!(!serialized.contains("the model said something"));

        let observations = recorder.observations();
        assert_eq!(
            observations[0].output.as_deref(),
            Some("the model said something")
        );
    }

    #[test]
    fn zero_limit_disables_payload_capture() {
        let mut recorder = RunRecorder::new(0);
        let p = payload(&recorder, Some("secret"));
        recorder.record_completion("final", "m", None, 5, p);
        assert!(recorder.observations()[0].output.is_none());
    }

    #[test]
    fn tool_output_truncation_survives_multibyte_input() {
        let mut recorder = RunRecorder::new(16);
        let text = "\u{201c}".repeat(4000);
        let args = json!({});
        recorder.record_tool_call(tool_call("get_posts", "ok", None, Some(&text), &args, 1));
        let output = recorder.observations()[0].output.clone().unwrap();
        assert_eq!(output.chars().count(), 2001);
    }

    #[test]
    fn tool_calls_become_tool_observations_with_call_id() {
        let mut recorder = RunRecorder::new(0);
        let args = json!({"query": "x"});
        recorder.record_tool_call(tool_call("search_forum", "ok", None, Some("[]"), &args, 5));
        let observation = &recorder.observations()[0];
        assert!(matches!(observation.kind, ObservationKind::Tool));
        assert_eq!(observation.tool_name.as_deref(), Some("search_forum"));
        assert_eq!(observation.tool_call_id.as_deref(), Some("call_1"));
    }

    /// Truncating serialized JSON leaves it unparseable, and Langfuse then
    /// silently renders raw text instead of a conversation.
    #[test]
    fn captured_requests_stay_valid_json_when_truncated() {
        let recorder = RunRecorder::new(20);
        let messages = vec![ChatCompletionRequestMessage::System(
            async_openai::types::ChatCompletionRequestSystemMessage {
                content: "x".repeat(5000).into(),
                name: None,
            },
        )];
        let captured = recorder.capture_request(&messages, None).unwrap();
        let parsed: Value = serde_json::from_str(&captured).expect("captured request must parse");
        let content = parsed["messages"][0]["content"].as_str().unwrap();
        assert_eq!(content.chars().count(), 21);
        assert!(captured.len() < 5000);
    }

    /// A tool-triggered run attaches to the span id assigned before the tool
    /// ran, so the child nests under that specific call rather than the run.
    #[test]
    fn tool_span_id_is_preserved_for_children_to_attach_to() {
        let mut recorder = RunRecorder::new(0);
        let call_context = recorder.context().child();
        let args = json!({});
        recorder.record_tool_call(ToolCallRecord {
            span_id: call_context.span_id,
            ..tool_call("get_topic_summary", "ok", None, None, &args, 5)
        });
        assert_eq!(recorder.observations()[0].span_id, Some(call_context.span_id));
        assert_eq!(call_context.trace_id, recorder.context().trace_id);
    }

    #[test]
    fn nesting_emits_into_the_parent_trace() {
        let parent = TraceContext::new();
        let standalone = RunRecorder::new(0).trace_spec("summary");
        assert!(standalone.parent.is_none());

        let nested = RunRecorder::new(0).nested_under(parent).trace_spec("summary");
        let nested_parent = nested.parent.expect("parent must be retained");
        assert_eq!(nested_parent.span_id, parent.span_id);
        assert_eq!(nested_parent.trace_id, parent.trace_id);
        // The child keeps its own root span so it stays a distinct subtree.
        assert_ne!(nested.context.span_id, parent.span_id);
    }

    #[test]
    fn observations_are_backdated_from_the_wall_clock_anchor() {
        let mut recorder = RunRecorder::new(0);
        let args = json!({});
        recorder.record_tool_call(tool_call("get_posts", "ok", None, None, &args, 40));
        let observation = &recorder.observations()[0];
        assert!(observation.start >= recorder.started_at());
        assert_eq!(
            observation.end.duration_since(observation.start).unwrap(),
            Duration::from_millis(40)
        );
    }
}
