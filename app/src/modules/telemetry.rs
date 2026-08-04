use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use figment::{Figment, providers::Env};
pub use opentelemetry::trace::SpanId;

use opentelemetry::trace::{
    SamplingDecision, SamplingResult, Span, SpanContext, SpanKind, TraceContextExt,
    TraceFlags, TraceId, TraceState, Tracer, TracerProvider as _,
};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::{Sampler, SdkTracer, SdkTracerProvider};
use serde::Deserialize;
use serde_json::json;
use tracing_subscriber::{
    EnvFilter, Layer, filter::Targets, layer::SubscriberExt, util::SubscriberInitExt,
};

fn default_environment() -> String {
    "development".to_string()
}

fn default_max_payload_chars() -> usize {
    20_000
}

#[derive(Debug, Deserialize, Clone)]
pub struct LangfuseConfig {
    pub host: String,
    pub public_key: String,
    pub secret_key: String,
    /// Needed only to build admin deep links; traces still export without it.
    pub project_id: Option<String>,
    #[serde(default = "default_environment")]
    pub environment: String,
    #[serde(default = "default_max_payload_chars")]
    pub max_payload_chars: usize,
}

/// Which Langfuse observation a span becomes. Only `Generation` carries model
/// and token usage, and only generations get cost attributed.
#[derive(Debug, Clone, Copy)]
pub enum ObservationKind {
    Generation,
    Span,
    Event,
    Tool,
}

impl ObservationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generation => "generation",
            Self::Span => "span",
            Self::Event => "event",
            Self::Tool => "tool",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub reasoning: u32,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub name: String,
    /// Pre-assigned when something needs to attach to this span while the run
    /// is still executing, as a tool-triggered child run does.
    pub span_id: Option<SpanId>,
    pub kind: ObservationKind,
    pub start: SystemTime,
    pub end: SystemTime,
    pub input: Option<String>,
    pub output: Option<String>,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
    pub error: Option<String>,
    /// Set on tool observations. `gen_ai.tool.name` replaces the span name in
    /// Langfuse, and either attribute alone classifies the span as a tool.
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
}

/// Identifies a span another run can attach to. Generated when a run starts,
/// not when it ends, so work the run triggers can nest under it while it is
/// still executing.
#[derive(Debug, Clone, Copy)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
}

impl TraceContext {
    pub fn new() -> Self {
        Self {
            trace_id: new_trace_id(),
            span_id: new_span_id(),
        }
    }

    /// A child span under this one, sharing the trace.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: new_span_id(),
        }
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// One complete LLM run, ready to be emitted as a Langfuse trace.
#[derive(Debug, Clone)]
pub struct TraceSpec {
    /// The run's own root span. When it carries a `parent`, the run is emitted
    /// inside that trace rather than starting its own.
    pub context: TraceContext,
    pub parent: Option<TraceContext>,
    pub name: String,
    pub session_id: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Vec<(String, String)>,
    pub start: SystemTime,
    pub end: SystemTime,
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub observations: Vec<Observation>,
}

pub struct Telemetry {
    provider: SdkTracerProvider,
    tracer: SdkTracer,
    config: LangfuseConfig,
}

/// Installs the tracing subscriber, and the Langfuse exporter when configured.
/// Must run before anything emits a span. Mirrors `LlmService::from_env`: a
/// missing or partial config disables tracing instead of failing startup.
pub fn init() -> Option<Telemetry> {
    let config = Figment::new()
        .merge(Env::prefixed("LANGFUSE_"))
        .extract::<LangfuseConfig>()
        .ok();

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    );

    let Some(config) = config else {
        tracing_subscriber::registry().with(fmt_layer).init();
        tracing::info!("Langfuse tracing disabled; set LANGFUSE_HOST, LANGFUSE_PUBLIC_KEY and LANGFUSE_SECRET_KEY to enable");
        return None;
    };

    let provider = match build_provider(&config) {
        Ok(provider) => provider,
        Err(e) => {
            tracing_subscriber::registry().with(fmt_layer).init();
            tracing::error!("failed to build Langfuse exporter, tracing disabled: {e}");
            return None;
        }
    };

    // Deliberately narrow: everything reaching this layer becomes a Langfuse
    // trace, so HTTP, sqlx and indexer spans would bury the LLM runs.
    let targets = std::env::var("OTEL_TRACE_TARGETS")
        .ok()
        .and_then(|raw| Targets::from_str(&raw).ok())
        .unwrap_or_else(|| {
            Targets::new().with_target("ethereum_forum::modules::llm", tracing::Level::INFO)
        });

    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(provider.tracer("ethereum-forum"))
        .with_error_records_to_exceptions(true)
        .with_filter(targets);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!(
        host = %config.host,
        environment = %config.environment,
        "Langfuse tracing configured"
    );

    let tracer = provider.tracer("ethereum-forum/llm");
    Some(Telemetry {
        provider,
        tracer,
        config,
    })
}

fn build_provider(config: &LangfuseConfig) -> anyhow::Result<SdkTracerProvider> {
    let auth = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", config.public_key, config.secret_key));

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), format!("Basic {auth}"));
    // Without this, directly ingested OTel data is delayed by up to 10 minutes.
    headers.insert("x-langfuse-ingestion-version".to_string(), "4".to_string());

    // A programmatically set endpoint is used verbatim; only the env-var path
    // appends the signal suffix.
    let endpoint = format!(
        "{}/api/public/otel/v1/traces",
        config.host.trim_end_matches('/')
    );

    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_protocol(Protocol::HttpBinary)
        .with_timeout(Duration::from_secs(10))
        .with_headers(headers)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(
            Resource::builder()
                .with_service_name("ethereum-forum")
                .build(),
        )
        .build())
}

fn new_trace_id() -> TraceId {
    TraceId::from_bytes(*uuid::Uuid::new_v4().as_bytes())
}

fn new_span_id() -> SpanId {
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    let mut id = [0u8; 8];
    id.copy_from_slice(&bytes[..8]);
    SpanId::from_bytes(id)
}

/// Short-circuits the provider sampler. A trace id is persisted to `llm_runs`
/// for deep linking, so the trace behind it must never be sampled away.
fn force_sample() -> SamplingResult {
    SamplingResult {
        decision: SamplingDecision::RecordAndSample,
        attributes: Vec::new(),
        trace_state: TraceState::default(),
    }
}

/// One session per invocation. The descriptor keeps the sessions list
/// scannable; the trace suffix keeps repeat attempts distinct.
pub fn run_session(descriptor: &str, trace_id: TraceId) -> String {
    format!("{descriptor}@{}", &trace_id.to_string()[..8])
}

pub fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cutoff, _)) => format!("{}\u{2026}", &text[..cutoff]),
        None => text.to_string(),
    }
}

impl Telemetry {
    pub fn max_payload_chars(&self) -> usize {
        self.config.max_payload_chars
    }

    /// Deep-link template for the admin UI, with `{trace_id}` as the
    /// placeholder. `None` when no project id is configured.
    pub fn trace_url_template(&self) -> Option<String> {
        let project = self.config.project_id.as_ref()?;
        Some(format!(
            "{}/project/{}/traces/{{trace_id}}",
            self.config.host.trim_end_matches('/'),
            project
        ))
    }

    /// Blocks up to ~5s draining the batch queue. `Drop` never runs on a
    /// globally held provider, so without this the tail of the queue is lost.
    pub fn shutdown(&self) {
        if let Err(e) = self.provider.shutdown() {
            tracing::warn!("Langfuse tracer shutdown failed: {e}");
        }
    }

    /// Emits a run as one Langfuse trace with a root span plus one child per
    /// observation, all backdated from the recorded timings. Returns the trace
    /// id to persist for deep linking.
    pub fn emit(&self, spec: TraceSpec) -> TraceId {
        // A nested run joins its parent's trace, so the trace-level fields
        // belong to the parent and must not be restated here — Langfuse
        // resolves them from every span and conflicting values are ambiguous.
        let nested = spec.parent.is_some();
        let trace_id = spec.parent.map_or(spec.context.trace_id, |p| p.trace_id);
        let root_span_id = spec.context.span_id;

        // Langfuse resolves trace-level fields from every span, not just the
        // root, so these are repeated onto each child.
        let mut shared = Vec::new();
        if !nested {
            shared.push(KeyValue::new("langfuse.trace.name", spec.name.clone()));
            if let Some(session) = &spec.session_id {
                shared.push(KeyValue::new("langfuse.session.id", session.clone()));
            }
        }
        shared.push(KeyValue::new(
            "langfuse.environment",
            self.config.environment.clone(),
        ));
        if !spec.tags.is_empty() {
            shared.push(KeyValue::new(
                "langfuse.trace.tags",
                opentelemetry::Value::Array(
                    spec.tags
                        .iter()
                        .map(|t| t.clone().into())
                        .collect::<Vec<opentelemetry::StringValue>>()
                        .into(),
                ),
            ));
        }
        for (key, value) in &spec.metadata {
            shared.push(KeyValue::new(
                format!("langfuse.trace.metadata.{key}"),
                value.clone(),
            ));
        }

        let mut root_attributes = shared.clone();
        root_attributes.push(KeyValue::new(
            "langfuse.observation.type",
            ObservationKind::Span.as_str(),
        ));
        if let Some(input) = &spec.input {
            let key = if nested {
                "langfuse.observation.input"
            } else {
                "langfuse.trace.input"
            };
            root_attributes.push(KeyValue::new(key, input.clone()));
        }
        if let Some(output) = &spec.output {
            let key = if nested {
                "langfuse.observation.output"
            } else {
                "langfuse.trace.output"
            };
            root_attributes.push(KeyValue::new(key, output.clone()));
        }
        push_error(&mut root_attributes, spec.error.as_deref());

        // An empty context is required for a root: `Context::current()` would
        // override the trace id above with any ambient span's, breaking the
        // persisted link.
        let root_parent = match spec.parent {
            Some(parent) => sampled_parent(parent.trace_id, parent.span_id),
            None => Context::new(),
        };
        let mut root = self.tracer.build_with_context(
            self.tracer
                .span_builder(spec.name.clone())
                .with_trace_id(trace_id)
                .with_span_id(root_span_id)
                .with_kind(SpanKind::Internal)
                .with_start_time(spec.start)
                .with_end_time(spec.end)
                .with_sampling_result(force_sample())
                .with_attributes(root_attributes),
            &root_parent,
        );

        let parent = sampled_parent(trace_id, root_span_id);

        for observation in spec.observations {
            let mut attributes = shared.clone();
            attributes.push(KeyValue::new(
                "langfuse.observation.type",
                observation.kind.as_str(),
            ));
            if let Some(input) = observation.input {
                attributes.push(KeyValue::new("langfuse.observation.input", input));
            }
            if let Some(output) = observation.output {
                attributes.push(KeyValue::new("langfuse.observation.output", output));
            }
            if let Some(model) = observation.model {
                attributes.push(KeyValue::new("langfuse.observation.model.name", model));
            }
            if let Some(tool_name) = observation.tool_name {
                attributes.push(KeyValue::new("gen_ai.tool.name", tool_name));
                attributes.push(KeyValue::new("gen_ai.operation.name", "execute_tool"));
            }
            if let Some(call_id) = observation.tool_call_id {
                attributes.push(KeyValue::new("gen_ai.tool.call.id", call_id));
            }
            if let Some(usage) = observation.usage {
                attributes.push(KeyValue::new(
                    "langfuse.observation.usage_details",
                    json!({
                        "input": usage.input,
                        "output": usage.output,
                        "reasoning": usage.reasoning,
                    })
                    .to_string(),
                ));
            }
            push_error(&mut attributes, observation.error.as_deref());

            let mut span = self.tracer.build_with_context(
                self.tracer
                    .span_builder(observation.name)
                    .with_span_id(observation.span_id.unwrap_or_else(new_span_id))
                    .with_kind(SpanKind::Internal)
                    .with_start_time(observation.start)
                    .with_end_time(observation.end)
                    .with_sampling_result(force_sample())
                    .with_attributes(attributes),
                &parent,
            );
            // `end()` would stamp the current time over the backdated end.
            span.end_with_timestamp(observation.end);
        }

        root.end_with_timestamp(spec.end);
        trace_id
    }
}

/// TraceFlags::SAMPLED is load-bearing: ParentBased(AlwaysOn) silently drops
/// every child whose parent context is not marked sampled.
fn sampled_parent(trace_id: TraceId, span_id: SpanId) -> Context {
    Context::new().with_remote_span_context(SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::SAMPLED,
        false,
        TraceState::default(),
    ))
}

fn push_error(attributes: &mut Vec<KeyValue>, error: Option<&str>) {
    if let Some(error) = error {
        attributes.push(KeyValue::new("langfuse.observation.level", "ERROR"));
        attributes.push(KeyValue::new(
            "langfuse.observation.status_message",
            error.to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn truncation_respects_character_boundaries() {
        let text = "\u{201c}EIP-1559\u{201d} \u{2014} fee market";
        assert_eq!(truncate_chars(text, 1), "\u{201c}\u{2026}");
        assert_eq!(truncate_chars(text, 100), text);
    }

    #[test]
    fn truncation_keeps_short_text_untouched() {
        assert_eq!(truncate_chars("abc", 3), "abc");
        assert_eq!(truncate_chars("abcd", 3), "abc\u{2026}");
    }
}
