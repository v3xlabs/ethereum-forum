use once_cell::sync::Lazy;
use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Meter},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;

static METER: Lazy<Meter> = Lazy::new(|| global::meter("ethereum-forum"));

// Manual tracking for OTLP export since we can't easily read from OpenTelemetry counters
static PROMPT_TOKENS_COUNTER: AtomicU64 = AtomicU64::new(0);
static COMPLETION_TOKENS_COUNTER: AtomicU64 = AtomicU64::new(0);
static TOTAL_TOKENS_COUNTER: AtomicU64 = AtomicU64::new(0);

// Model and user-specific counters: (user_id, model) -> (prompt, completion, total)
static MODEL_USER_METRICS: Lazy<Mutex<HashMap<(String, String), (AtomicU64, AtomicU64, AtomicU64)>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

pub static PROMPT_TOKENS: Lazy<Counter<u64>> = Lazy::new(|| {
    METER
        .u64_counter("openai_prompt_tokens")
        .with_description("OpenAI prompt tokens")
        .with_unit("tokens")
        .build()
});

pub static COMPLETION_TOKENS: Lazy<Counter<u64>> = Lazy::new(|| {
    METER
        .u64_counter("openai_completion_tokens")
        .with_description("OpenAI completion tokens")
        .with_unit("tokens")
        .build()
});

pub static TOTAL_TOKENS: Lazy<Counter<u64>> = Lazy::new(|| {
    METER
        .u64_counter("openai_total_tokens")
        .with_description("OpenAI total tokens")
        .with_unit("tokens")
        .build()
});

pub fn record_openai_usage(user_id: Option<uuid::Uuid>, model_name: &str, usage: &openai::Usage) {
    let user_id_str = match user_id {
        Some(id) => id.to_string(),
        None => "system".to_string(),
    };
    
    let attrs = vec![
        KeyValue::new("user_id", user_id_str.clone()),
        KeyValue::new("model", model_name.to_string()),
    ];
    
    tracing::info!(
        "📊 Recording OpenAI usage - prompt: {}, completion: {}, total: {}, user: {:?}, model: {}",
        usage.prompt_tokens,
        usage.completion_tokens,
        usage.total_tokens,
        user_id,
        model_name
    );
    
    // Record in OpenTelemetry counters
    PROMPT_TOKENS.add(usage.prompt_tokens as u64, &attrs);
    COMPLETION_TOKENS.add(usage.completion_tokens as u64, &attrs);
    TOTAL_TOKENS.add(usage.total_tokens as u64, &attrs);
    
    // Also track manually for OTLP export
    PROMPT_TOKENS_COUNTER.fetch_add(usage.prompt_tokens as u64, Ordering::Relaxed);
    COMPLETION_TOKENS_COUNTER.fetch_add(usage.completion_tokens as u64, Ordering::Relaxed);
    TOTAL_TOKENS_COUNTER.fetch_add(usage.total_tokens as u64, Ordering::Relaxed);
    
    // Track per-user and per-model metrics
    if let Ok(mut model_user_metrics) = MODEL_USER_METRICS.lock() {
        let key = (user_id_str, model_name.to_string());
        let (prompt_counter, completion_counter, total_counter) = model_user_metrics
            .entry(key)
            .or_insert_with(|| (AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)));
        
        prompt_counter.fetch_add(usage.prompt_tokens as u64, Ordering::Relaxed);
        completion_counter.fetch_add(usage.completion_tokens as u64, Ordering::Relaxed);
        total_counter.fetch_add(usage.total_tokens as u64, Ordering::Relaxed);
    }
}

// Test function to verify metrics are working
pub fn test_metrics() {
    tracing::info!("🧪 Testing metrics system...");
    
    // Create a few test usage records
    let test_usage1 = openai::Usage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };
    
    let test_usage2 = openai::Usage {
        prompt_tokens: 15,
        completion_tokens: 25,
        total_tokens: 40,
    };
    
    // Record some test metrics
    record_openai_usage(None, "test-model-1", &test_usage1);
    record_openai_usage(Some(uuid::Uuid::new_v4()), "test-model-2", &test_usage2);
    
    tracing::info!("✅ Test metrics recorded successfully");
    tracing::info!(
        "📊 Total counters - prompt: {}, completion: {}, total: {}",
        PROMPT_TOKENS_COUNTER.load(Ordering::Relaxed),
        COMPLETION_TOKENS_COUNTER.load(Ordering::Relaxed),
        TOTAL_TOKENS_COUNTER.load(Ordering::Relaxed)
    );
}

// Manual OTLP export function using HTTP
pub async fn export_metrics_to_otlp() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let base_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string()); // Use HTTP endpoint
    
    let endpoint = if base_endpoint.ends_with("/v1/metrics") {
        base_endpoint
    } else {
        format!("{}/v1/metrics", base_endpoint.trim_end_matches('/'))
    };
    
    tracing::debug!("📤 Exporting metrics to OTLP HTTP endpoint: {}", endpoint);
    
    // Get current metric values
    let prompt_tokens = PROMPT_TOKENS_COUNTER.load(Ordering::Relaxed);
    let completion_tokens = COMPLETION_TOKENS_COUNTER.load(Ordering::Relaxed);
    let total_tokens = TOTAL_TOKENS_COUNTER.load(Ordering::Relaxed);
    
    let timestamp_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos() as u64;
    
    tracing::debug!(
        "📊 Current metrics - prompt: {}, completion: {}, total: {}",
        prompt_tokens, completion_tokens, total_tokens
    );
    
    // Create all metric data points including per-user breakdown
    let mut all_data_points = Vec::new();
    
    // Add system-level aggregated metrics (across all models)
    all_data_points.push(serde_json::json!({
        "attributes": [
            {"key": "user_id", "value": {"stringValue": "system"}},
            {"key": "model", "value": {"stringValue": "all"}}
        ],
        "timeUnixNano": timestamp_nanos,
        "asInt": total_tokens.to_string()
    }));
    
    // Add per-user and per-model metrics
    if let Ok(model_user_metrics) = MODEL_USER_METRICS.lock() {
        for ((user_id, model), (_, _, total_counter)) in model_user_metrics.iter() {
            let user_total = total_counter.load(Ordering::Relaxed);
            if user_total > 0 {
                all_data_points.push(serde_json::json!({
                    "attributes": [
                        {"key": "user_id", "value": {"stringValue": user_id}},
                        {"key": "model", "value": {"stringValue": model}}
                    ],
                    "timeUnixNano": timestamp_nanos,
                    "asInt": user_total.to_string()
                }));
            }
        }
    }
    
    // Create OTLP payload with all three metrics
    let otlp_payload = serde_json::json!({
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": "ethereum-forum"}},
                    {"key": "service.version", "value": {"stringValue": "0.1.0"}}
                ]
            },
            "scopeMetrics": [{
                "scope": {
                    "name": "ethereum-forum",
                    "version": "0.1.0"
                },
                "metrics": [
                    {
                        "name": "openai_prompt_tokens",
                        "description": "OpenAI prompt tokens",
                        "unit": "tokens",
                        "sum": {
                            "dataPoints": [{
                                "attributes": [
                                    {"key": "user_id", "value": {"stringValue": "system"}},
                                    {"key": "model", "value": {"stringValue": "all"}}
                                ],
                                "timeUnixNano": timestamp_nanos,
                                "asInt": prompt_tokens.to_string()
                            }],
                            "aggregationTemporality": 2,
                            "isMonotonic": true
                        }
                    },
                    {
                        "name": "openai_completion_tokens",
                        "description": "OpenAI completion tokens", 
                        "unit": "tokens",
                        "sum": {
                            "dataPoints": [{
                                "attributes": [
                                    {"key": "user_id", "value": {"stringValue": "system"}},
                                    {"key": "model", "value": {"stringValue": "all"}}
                                ],
                                "timeUnixNano": timestamp_nanos,
                                "asInt": completion_tokens.to_string()
                            }],
                            "aggregationTemporality": 2,
                            "isMonotonic": true
                        }
                    },
                    {
                        "name": "openai_total_tokens",
                        "description": "OpenAI total tokens",
                        "unit": "tokens",
                        "sum": {
                            "dataPoints": all_data_points,
                            "aggregationTemporality": 2,
                            "isMonotonic": true
                        }
                    }
                ]
            }]
        }]
    });
    
    // Send to OTLP collector
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("content-type", "application/json")
        .json(&otlp_payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        tracing::info!("✅ Successfully exported metrics to OTLP collector");
    } else {
        tracing::warn!("⚠️ OTLP export failed: {} - {}", response.status(), response.text().await?);
    }
    
    Ok(())
}

// Start background metrics export task
pub fn start_metrics_export_task() {
    let export_interval = std::env::var("OTEL_METRIC_EXPORT_INTERVAL")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .unwrap_or(30);
    
    tracing::info!("🚀 Starting metrics export task (interval: {}s)", export_interval);
    
    async_std::task::spawn(async move {
        let mut interval = async_std::stream::interval(std::time::Duration::from_secs(export_interval));
        use futures::StreamExt;
        
        while let Some(_) = interval.next().await {
            if let Err(e) = export_metrics_to_otlp().await {
                tracing::warn!("❌ Failed to export metrics: {}", e);
            }
        }
    });
}
