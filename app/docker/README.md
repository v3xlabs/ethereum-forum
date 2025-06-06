# OpenTelemetry Observability Stack

This Docker Compose setup provides a complete observability stack for monitoring your Ethereum Forum application's OpenAI usage metrics.

## Services

### OpenTelemetry Collector
- **Port**: 4317 (gRPC), 4318 (HTTP)
- **Purpose**: Receives OTLP data from your Rust application
- **Config**: `otel-collector-config.yaml`

### Prometheus  
- **Port**: 9090
- **Purpose**: Stores metrics data
- **Config**: `prometheus.yml`
- **URL**: http://localhost:9090

### Grafana
- **Port**: 3001
- **Purpose**: Visualizes metrics with dashboards
- **Credentials**: admin/admin
- **URL**: http://localhost:3001

## Getting Started

1. **Start the stack:**
   ```bash
   docker compose up -d
   ```

2. **Configure your Rust app** to send OTLP data to `localhost:4317` (gRPC) or `localhost:4318` (HTTP)

3. **Access Grafana** at http://localhost:3001 (admin/admin)
   - The "OpenAI Usage Metrics" dashboard is automatically provisioned
   - View real-time token usage, rates, and user breakdowns

4. **Access Prometheus** at http://localhost:9090 for raw metric queries

## Metrics Available

Your Rust application exports these OpenAI usage metrics:
- `openai_prompt_tokens_total` - Total prompt tokens used
- `openai_completion_tokens_total` - Total completion tokens used  
- `openai_total_tokens_total` - Total tokens used (prompt + completion)

Each metric includes a `user_id` label for per-user tracking.

## Environment Variables for Rust App

Make sure your Rust application is configured to send OTLP data:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=ethereum-forum
```

## Stopping the Stack

```bash
docker compose down
```

To also remove volumes:
```bash
docker compose down -v
``` 