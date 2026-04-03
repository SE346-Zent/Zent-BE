use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::{BatchSpanProcessor, TracerProvider as SdkTracerProvider};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialise the global tracing / OpenTelemetry pipeline.
///
/// Log output is **structured JSON** in the format:
/// ```json
/// {
///   "timestamp": "2026-03-30T12:03:23Z",
///   "level": "ERROR",
///   "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
///   "span_id": "00f067aa0ba902b7",
///   "message": "Database connection timeout",
///   "context": { "endpoint": "GET /work-orders", "user_id": "uuid-1234" }
/// }
/// ```
///
/// When the `OTEL_EXPORTER_OTLP_ENDPOINT` env-var is set, traces are also
/// exported over gRPC to the configured collector (e.g. Jaeger, Grafana Tempo).
pub fn init_tracing() {
    // --- OpenTelemetry propagator (W3C TraceContext) -----------------------
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    // --- Build OTLP tracer provider (or noop) ------------------------------
    let otel_layer = match build_otlp_tracer_provider() {
        Some(provider) => {
            let tracer = provider.tracer("zent-be");
            // Register globally so spans propagate across async boundaries
            opentelemetry::global::set_tracer_provider(provider);
            Some(tracing_opentelemetry::layer().with_tracer(tracer))
        }
        None => None,
    };

    // --- Env filter --------------------------------------------------------
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into());

    // --- JSON fmt layer ----------------------------------------------------
    let json_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_span_events(FmtSpan::NONE);

    // --- Assemble the subscriber -------------------------------------------
    tracing_subscriber::registry()
        .with(env_filter)
        .with(json_layer)
        .with(otel_layer)
        .init();
}

/// Try to build an OTLP gRPC tracer provider.
/// Returns `None` when `OTEL_EXPORTER_OTLP_ENDPOINT` is not set.
fn build_otlp_tracer_provider() -> Option<SdkTracerProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    if endpoint.is_empty() {
        return None;
    }

    let exporter = SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to build OTLP span exporter");

    let batch_processor =
        BatchSpanProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();

    let provider = SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .build();

    Some(provider)
}

/// Flush remaining spans / traces when the application shuts down.
pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}
