use async_openai::Client;
use async_openai::config::OpenAIConfig;
use figment::{Figment, providers::Env};
use serde::Deserialize;

use crate::modules::llm::streams::StreamRegistry;

pub mod curator;
pub mod digest;
pub mod executor;
pub mod recorder;
pub mod streams;
pub mod summary;
pub mod tokens;

fn default_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_max_input_tokens() -> usize {
    100_000
}

fn default_curator_interval_hours() -> i64 {
    24
}

fn default_max_tool_calls() -> u32 {
    8
}

fn default_memory_token_budget() -> usize {
    4096
}

fn default_max_tool_rounds() -> u32 {
    6
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub model: String,
    pub digest_model: Option<String>,
    pub curator_model: Option<String>,
    #[serde(default = "default_max_input_tokens")]
    pub max_input_tokens: usize,
    #[serde(default = "default_curator_interval_hours")]
    pub curator_interval_hours: i64,
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: u32,
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: u32,
    #[serde(default = "default_memory_token_budget")]
    pub memory_token_budget: usize,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(),
            model: String::new(),
            digest_model: None,
            curator_model: None,
            max_input_tokens: default_max_input_tokens(),
            curator_interval_hours: default_curator_interval_hours(),
            max_tool_calls: default_max_tool_calls(),
            max_tool_rounds: default_max_tool_rounds(),
            memory_token_budget: 4096,
        }
    }
}

pub struct LlmService {
    pub config: LlmConfig,
    pub client: Client<OpenAIConfig>,
    pub streams: StreamRegistry,
}

impl LlmService {
    pub fn from_env() -> Option<Self> {
        let config = match Figment::new()
            .merge(Env::prefixed("LLM_"))
            .extract::<LlmConfig>()
        {
            Ok(config) => config,
            Err(e) => {
                if std::env::var("WORKSHOP_INTELLIGENCE_KEY").is_ok()
                    && std::env::var("LLM_API_KEY").is_err()
                {
                    tracing::warn!(
                        "WORKSHOP_INTELLIGENCE_KEY is set but no longer read; migrate to LLM_API_KEY / LLM_MODEL (optionally LLM_BASE_URL, LLM_DIGEST_MODEL)"
                    );
                }
                tracing::warn!(
                    "LLM features disabled; set LLM_API_KEY and LLM_MODEL to enable ({e})"
                );
                return None;
            }
        };

        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key(config.api_key.clone())
                .with_api_base(config.base_url.clone()),
        );

        tracing::info!(
            model = %config.model,
            base_url = %config.base_url,
            max_input_tokens = %config.max_input_tokens,
            "LLM service configured"
        );

        Some(Self {
            config,
            client,
            streams: StreamRegistry::default(),
        })
    }

    pub fn digest_model(&self) -> String {
        self.config
            .digest_model
            .clone()
            .unwrap_or_else(|| self.config.model.clone())
    }

    pub fn curator_model(&self) -> String {
        self.config
            .curator_model
            .clone()
            .unwrap_or_else(|| self.config.model.clone())
    }
}

use crate::models::llm::LlmMemory;
use crate::modules::llm::tokens::estimate_tokens_in_text;

/// Renders the shared memory glossary into a system-prompt section, with
/// source links labelled briefly, capped to `token_budget` tokens. Returns an
/// empty string when memory is empty or the budget is zero. Entries are
/// rendered in `term` order; once the budget is exhausted further entries are
/// dropped (with a warning) so a growing glossary can never silently bloat
/// every run.
pub fn render_memory_section(memory: &[LlmMemory], token_budget: usize) -> String {
    if memory.is_empty() || token_budget == 0 {
        return String::new();
    }

    const HEADER: &str = "\n\n## Background context and terminology\n\n";
    const FOOTER: &str = "\n\nUse the above context silently and never reference this section in your output.";

    let header_tokens = estimate_tokens_in_text(HEADER);
    let footer_tokens = estimate_tokens_in_text(FOOTER);
    let mut budget = token_budget.saturating_sub(header_tokens + footer_tokens);

    let mut lines = Vec::with_capacity(memory.len());
    let mut dropped = 0usize;
    for entry in memory {
        let sources = entry.sources.as_ref().and_then(|s| s.as_array()).map(|arr| {
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
        let block = match sources {
            Some(s) if !s.is_empty() => format!("- **{}**: {}\n{}", entry.term, entry.content, s),
            _ => format!("- **{}**: {}", entry.term, entry.content),
        };
        let block_tokens = estimate_tokens_in_text(&block) + 1;
        if block_tokens > budget {
            dropped += 1;
            continue;
        }
        budget -= block_tokens;
        lines.push(block);
    }

    if dropped > 0 {
        tracing::warn!(
            dropped,
            budget = token_budget,
            "memory section exceeded token budget; dropped entries"
        );
    }

    if lines.is_empty() {
        return String::new();
    }

    format!("{HEADER}{}\n{FOOTER}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::render_memory_section;
    use crate::models::llm::LlmMemory;
    use chrono::Utc;
    use serde_json::json;

    fn memory(term: &str, content: &str, sources: serde_json::Value) -> LlmMemory {
        LlmMemory {
            entry_id: 0,
            term: term.into(),
            content: content.into(),
            sources: Some(sources),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn empty_memory_or_zero_budget_returns_empty() {
        assert_eq!(render_memory_section(&[], 4096), "");
        let m = memory("EIP-1559", "Fee market change.", json!([]));
        assert_eq!(render_memory_section(&[m], 0), "");
    }

    #[test]
    fn renders_term_content_and_labelled_sources() {
        let m = memory(
            "EIP-1559",
            "Fee market change.",
            json!([
                {"url": "/t/magicians/1234#p-2", "reason": "core proposal"},
                {"url": "https://eips.ethereum.org/EIPS/eip-1559", "reason": ""},
                {"url": "", "reason": "dropped"},
            ]),
        );
        let out = render_memory_section(&[m], 4096);
        assert!(out.contains("- **EIP-1559**: Fee market change."));
        assert!(out.contains("  - /t/magicians/1234#p-2 — core proposal"));
        assert!(out.contains("  - https://eips.ethereum.org/EIPS/eip-1559"));
        assert!(!out.contains("dropped"));
        assert!(out.contains("Use the above context silently"));
    }

    #[test]
    fn budget_cap_drops_entries() {
        let entries: Vec<LlmMemory> = (0..50)
            .map(|i| {
                memory(
                    &format!("term-{i}"),
                    &"x".repeat(200),
                    json!([]),
                )
            })
            .collect();
        let out = render_memory_section(&entries, 300);
        assert!(!out.is_empty());
        assert!(out.contains("term-0"));
        assert!(!out.contains("term-49"));
    }
}
