use async_openai::types::CompletionUsage;
use serde_json::{Value, json};
use std::time::Instant;

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

/// Collects the chronological trace of an LLM run (tool calls, completions,
/// notes) plus accumulated usage, for persistence on the `llm_runs` row.
pub struct RunRecorder {
    started: Instant,
    pub usage: RunUsage,
    pub rounds: u32,
    events: Vec<Value>,
}

impl Default for RunRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl RunRecorder {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            usage: RunUsage::default(),
            rounds: 0,
            events: Vec::new(),
        }
    }

    fn at_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn duration_ms(&self) -> i32 {
        self.started.elapsed().as_millis() as i32
    }

    pub fn record_tool_call(
        &mut self,
        tool: &str,
        label: &str,
        status: &str,
        detail: Option<&str>,
        output: Option<&str>,
        args: &Value,
        duration_ms: u64,
    ) {
        let at_ms = self.at_ms().saturating_sub(duration_ms);
        let truncated_output = output.map(|o| {
            const MAX: usize = 2000;
            if o.len() > MAX {
                format!("{}…", &o[..MAX])
            } else {
                o.to_string()
            }
        });
        self.events.push(json!({
            "type": "tool_call",
            "tool": tool,
            "label": label,
            "status": status,
            "detail": detail,
            "output": truncated_output,
            "args": args,
            "at_ms": at_ms,
            "duration_ms": duration_ms,
        }));
    }

    pub fn record_completion(
        &mut self,
        label: &str,
        model: &str,
        usage: Option<&CompletionUsage>,
        duration_ms: u64,
    ) {
        if let Some(usage) = usage {
            self.usage.add(usage);
        }
        let at_ms = self.at_ms().saturating_sub(duration_ms);
        self.events.push(json!({
            "type": "completion",
            "label": label,
            "model": model,
            "prompt_tokens": usage.map(|u| u.prompt_tokens),
            "completion_tokens": usage.map(|u| u.completion_tokens),
            "at_ms": at_ms,
            "duration_ms": duration_ms,
        }));
    }

    pub fn note(&mut self, label: impl Into<String>) {
        self.events.push(json!({
            "type": "note",
            "label": label.into(),
            "at_ms": self.at_ms(),
        }));
    }

    pub fn tool_call_count(&self) -> i32 {
        self.events
            .iter()
            .filter(|e| e["type"] == "tool_call")
            .count() as i32
    }

    pub fn trace_json(&self) -> Value {
        Value::Array(self.events.clone())
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

    #[test]
    fn accumulates_usage_across_completions() {
        let mut recorder = RunRecorder::new();
        recorder.record_completion("tool round 1", "m", Some(&usage(100, 20)), 5);
        recorder.record_completion("final", "m", Some(&usage(200, 80)), 5);
        recorder.record_completion("no usage reported", "m", None, 5);
        assert_eq!(recorder.usage.prompt_tokens, 300);
        assert_eq!(recorder.usage.completion_tokens, 100);
        assert_eq!(recorder.usage.total(), 400);
    }

    #[test]
    fn trace_preserves_order_and_counts_tool_calls() {
        let mut recorder = RunRecorder::new();
        recorder.record_tool_call("search_forum", "Searching", "ok", Some("3 results"), None, &json!({"query": "x"}), 10);
        recorder.record_completion("tool round 1", "m", Some(&usage(1, 1)), 5);
        recorder.record_tool_call("get_posts", "Reading", "error", Some("boom"), None, &json!({}), 10);
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
}
