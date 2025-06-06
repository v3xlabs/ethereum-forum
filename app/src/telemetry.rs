use opentelemetry::global;
use opentelemetry_sdk::metrics::SdkMeterProvider;

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    
    tracing::info!("🔧 Initializing OpenTelemetry metrics with target: {}", endpoint);

    // Create basic meter provider - metrics will be recorded in memory
    let meter_provider = SdkMeterProvider::builder().build();

    // Set the global meter provider
    global::set_meter_provider(meter_provider);
    
    tracing::info!("✅ OpenTelemetry SDK initialized - metrics recording enabled");
    tracing::info!("🔧 Target OTLP endpoint: {}", endpoint);
    tracing::info!("📝 Metrics can be accessed programmatically or exported manually");
    Ok(())
} 