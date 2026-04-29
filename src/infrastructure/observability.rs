use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{BatchSpanProcessor, TracerProvider as SdkTracerProvider},
    metrics::{PeriodicReader, SdkMeterProvider},
    logs::{BatchLogProcessor, LoggerProvider as SdkLoggerProvider},
    propagation::TraceContextPropagator,
    Resource,
};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use std::time::Duration;
use std::sync::OnceLock;
use crate::core::config::AppConfig;

static METER_PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();
static LOGGER_PROVIDER: OnceLock<SdkLoggerProvider> = OnceLock::new();

/// Initialise the global observability pipeline (Traces, Metrics, Logs).
///
/// Designed to work with an OpenTelemetry Collector or Grafana Alloy agent.
/// The agent is expected to be reachable at the endpoint defined in `OTEL_EXPORTER_OTLP_ENDPOINT`
/// (e.g., http://alloy:4317 within a Docker network).
pub fn init_tracing() {
    let cfg = AppConfig::get();
    
    // --- OpenTelemetry propagator (W3C TraceContext) -----------------------
    global::set_text_map_propagator(TraceContextPropagator::new());

    let resource = Resource::new(vec![
        KeyValue::new("service.name", cfg.otel_service_name.clone().unwrap_or_else(|| "zent-be".to_string())),
        KeyValue::new("deployment.environment", cfg.app_stage.clone()),
    ]);

    // Default to local agent if endpoint is not configured
    let agent_endpoint = cfg.otel_exporter_otlp_endpoint.clone()
        .unwrap_or_else(|| "http://localhost:4317".to_string());

    println!("OTEL Agent Endpoint configured as: {}", agent_endpoint);

    

    // --- Build OTLP tracer provider ------------------------------
    let otel_trace_layer = if let Some(provider) = build_otlp_tracer_provider(format!("{agent_endpoint}/v1/traces").as_str(), resource.clone()) {
        let tracer = provider.tracer("zent-be");
        global::set_tracer_provider(provider);
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    // --- Build OTLP meter provider -------------------------------
    if let Some(meter_provider) = build_otlp_meter_provider(format!("{agent_endpoint}/v1/metrics").as_str(), resource.clone()) {
        global::set_meter_provider(meter_provider.clone());
        let _ = METER_PROVIDER.set(meter_provider);
    }

    // --- Build OTLP logger provider ------------------------------
    let otel_log_layer = if let Some(logger_provider) = build_otlp_logger_provider(format!("{agent_endpoint}/v1/logs").as_str(), resource) {
        let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);
        let _ = LOGGER_PROVIDER.set(logger_provider);
        Some(layer)
    } else {
        None
    };

    // --- Env filter --------------------------------------------------------
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into());

    // --- JSON fmt layer (Console) ------------------------------------------
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
        .with(otel_trace_layer)
        .with(otel_log_layer)
        .init();
}

fn build_otlp_tracer_provider(endpoint: &str, resource: Resource) -> Option<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .ok()?;

    let batch_processor = BatchSpanProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(batch_processor)
        .build();

    Some(provider)
}

fn build_otlp_meter_provider(endpoint: &str, resource: Resource) -> Option<SdkMeterProvider> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .ok()?;

    let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_interval(Duration::from_secs(30))
        .build();

    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    Some(provider)
}

fn build_otlp_logger_provider(endpoint: &str, resource: Resource) -> Option<SdkLoggerProvider> {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .ok()?;

    let batch_processor = BatchLogProcessor::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();

    let provider = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_log_processor(batch_processor)
        .build();

    Some(provider)
}

/// Flush remaining signals when the application shuts down.
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
    if let Some(mp) = METER_PROVIDER.get() {
        let _ = mp.shutdown();
    }
    if let Some(lp) = LOGGER_PROVIDER.get() {
        let _ = lp.shutdown();
    }
}

/// Returns the global meter for the application.
pub fn meter() -> opentelemetry::metrics::Meter {
    global::meter("zent-be")
}
