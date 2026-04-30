use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{WithExportConfig, SpanExporter, MetricExporter, LogExporter};
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
pub fn init_tracing() {
    let cfg = AppConfig::get();
    
    // 1. Set up global propagator (W3C TraceContext)
    global::set_text_map_propagator(TraceContextPropagator::new());

    // 2. Build Resource with service details and default attributes
    let resource = Resource::new_with_defaults(vec![
        KeyValue::new("service.name", cfg.otel_service_name.clone().unwrap_or_else(|| "zent-be".to_string())),
        KeyValue::new("deployment.environment", cfg.app_stage.clone()),
    ]);

    // 3. Determine endpoint (Default to local OTLP/HTTP if not specified)
    // Note: 4317 is usually gRPC, 4318 is usually HTTP. 
    // We use HTTP as requested in the original implementation.
    let agent_endpoint = cfg.otel_exporter_otlp_endpoint.clone()
        .unwrap_or_else(|| "http://localhost:4318".to_string());

    println!("Observability: Configuring OTEL Agent Endpoint: {}", agent_endpoint);

    // 4. Build OTLP tracer provider
    let otel_trace_layer = if let Some(provider) = build_otlp_tracer_provider(&agent_endpoint, resource.clone()) {
        let tracer = provider.tracer("zent-be");
        global::set_tracer_provider(provider);
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        println!("Warning: Failed to initialize OTLP tracer provider");
        None
    };

    // 5. Build OTLP meter provider
    if let Some(meter_provider) = build_otlp_meter_provider(&agent_endpoint, resource.clone()) {
        global::set_meter_provider(meter_provider.clone());
        if let Err(_) = METER_PROVIDER.set(meter_provider) {
            println!("Warning: METER_PROVIDER already set");
        }
    } else {
        println!("Warning: Failed to initialize OTLP meter provider");
    }

    // 6. Build OTLP logger provider and tracing layer
    let otel_log_layer = if let Some(logger_provider) = build_otlp_logger_provider(&agent_endpoint, resource) {
        let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);
        if let Err(_) = LOGGER_PROVIDER.set(logger_provider) {
            println!("Warning: LOGGER_PROVIDER already set");
        }
        Some(layer)
    } else {
        println!("Warning: Failed to initialize OTLP logger provider");
        None
    };

    // 7. Configure EnvFilter (defaults to debug, but suppress noisy OTEL/HTTP crates)
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "debug,hyper_util=info,reqwest=info,opentelemetry=off,opentelemetry_sdk=off,opentelemetry_otlp=off".into()
    });

    // 8. Configure Console JSON layer
    let json_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_span_events(FmtSpan::NONE);

    // 9. Assemble and initialize the tracing subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(json_layer)
        .with(otel_trace_layer)
        .with(otel_log_layer)
        .init();
        
    println!("Observability: Pipeline initialized successfully.");
}

/// Builds an OTLP/HTTP tracer provider.
fn build_otlp_tracer_provider(base_endpoint: &str, resource: Resource) -> Option<SdkTracerProvider> {
    let endpoint = if base_endpoint.ends_with("/v1/traces") {
        base_endpoint.to_string()
    } else {
        format!("{}/v1/traces", base_endpoint)
    };

    let exporter = SpanExporter::builder()
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

/// Builds an OTLP/HTTP meter provider.
fn build_otlp_meter_provider(base_endpoint: &str, resource: Resource) -> Option<SdkMeterProvider> {
    let endpoint = if base_endpoint.ends_with("/v1/metrics") {
        base_endpoint.to_string()
    } else {
        format!("{}/v1/metrics", base_endpoint)
    };

    let exporter = MetricExporter::builder()
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

/// Builds an OTLP/HTTP logger provider.
fn build_otlp_logger_provider(base_endpoint: &str, resource: Resource) -> Option<SdkLoggerProvider> {
    let endpoint = if base_endpoint.ends_with("/v1/logs") {
        base_endpoint.to_string()
    } else {
        format!("{}/v1/logs", base_endpoint)
    };

    let exporter = LogExporter::builder()
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

/// Flush and shutdown all global observability signals.
pub fn shutdown_tracing() {
    println!("Observability: Shutting down pipeline...");
    
    // Shut down Tracer
    global::shutdown_tracer_provider();
    
    // Shut down Meter
    if let Some(mp) = METER_PROVIDER.get() {
        if let Err(err) = mp.shutdown() {
            eprintln!("Error shutting down meter provider: {:?}", err);
        }
    }
    
    // Shut down Logger
    if let Some(lp) = LOGGER_PROVIDER.get() {
        if let Err(err) = lp.shutdown() {
            eprintln!("Error shutting down logger provider: {:?}", err);
        }
    }
    
    println!("Observability: Pipeline shut down complete.");
}

/// Returns the application-global meter.
pub fn meter() -> opentelemetry::metrics::Meter {
    global::meter("zent-be")
}
