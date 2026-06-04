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
pub fn init_tracing() {
    let cfg = AppConfig::get();
    
    // 1. Set up global propagator (W3C TraceContext)
    global::set_text_map_propagator(TraceContextPropagator::new());

    // 2. Build Resource
    let resource = Resource::new_with_defaults(vec![
        KeyValue::new("service.name", cfg.otel_service_name.clone().unwrap_or_else(|| "zent-be".to_string())),
        KeyValue::new("environment", cfg.app_stage.clone()),
    ]);

    let agent_endpoint = cfg.otel_exporter_otlp_endpoint.clone()
        .unwrap_or_else(|| "http://localhost:4318".to_string());

    println!("Observability: Configuring OTEL Agent Endpoint: {}", agent_endpoint);

    // 3. Build OTLP tracer provider
    let otel_trace_layer = if let Some(provider) = build_otlp_tracer_provider(&agent_endpoint, resource.clone()) {
        let tracer = provider.tracer(cfg.otel_service_name.clone().unwrap_or_else(|| "zent-be".to_string()));
        global::set_tracer_provider(provider);
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        println!("Warning: Failed to initialize OTLP tracer provider");
        None
    };

    // 4. Build OTLP meter provider
    match build_otlp_meter_provider(&agent_endpoint, resource.clone()) {
        Some(meter_provider) => {
            global::set_meter_provider(meter_provider.clone());
            let _ = METER_PROVIDER.set(meter_provider);
            println!("Observability: OTLP meter provider initialized → {}/v1/metrics", agent_endpoint);
        }
        None => {
            eprintln!("WARNING: Failed to initialize OTLP meter provider. Metrics will NOT be exported to Grafana. Check OTEL_EXPORTER_OTLP_ENDPOINT (current: {})", agent_endpoint);
        }
    }

    // 5. Build OTLP logger provider and tracing layer
    let otel_log_layer = match build_otlp_logger_provider(&agent_endpoint, resource) {
        Some(logger_provider) => {
            let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);
            let _ = LOGGER_PROVIDER.set(logger_provider);
            println!("Observability: OTLP logger provider initialized → {}/v1/logs", agent_endpoint);
            Some(layer)
        }
        None => {
            eprintln!("WARNING: Failed to initialize OTLP logger provider. Logs will NOT be exported to Grafana.");
            None
        }
    };

    // 6. Configure EnvFilter
    // OTel SDK errors MUST be visible — if export fails, you need to see why.
    // Debug/Info from OTel is still suppressed to avoid noise.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "info,zent_be=debug,hyper_util=info,reqwest=info,opentelemetry=error,opentelemetry_sdk=error,opentelemetry_otlp=error".into()
    });

    // 7. Configure Console JSON layer
    let json_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_span_events(FmtSpan::CLOSE);

    // 8. Assemble and initialize
    tracing_subscriber::registry()
        .with(env_filter)
        .with(json_layer)
        .with(otel_trace_layer)
        .with(otel_log_layer)
        .init();
        
    // 9. Set custom panic hook for structured logging
    set_panic_hook();

    println!("Observability: Pipeline initialized successfully.");

    // Log diagnostics through the now-active tracing subscriber so they
    // appear in the OTLP log pipeline (not just container stdout).
    tracing::info!(
        otel_endpoint = %agent_endpoint,
        meter_provider_initialized = METER_PROVIDER.get().is_some(),
        logger_provider_initialized = LOGGER_PROVIDER.get().is_some(),
        "Observability pipeline initialized"
    );

    if METER_PROVIDER.get().is_none() {
        tracing::error!(
            otel_endpoint = %agent_endpoint,
            "OTLP meter provider FAILED to initialize — metrics will NOT reach Grafana. \
             Verify OTEL_EXPORTER_OTLP_ENDPOINT is set and the Alloy agent is reachable."
        );
    }
}

fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string());
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<Any>".to_string()
        };

        tracing::error!(
            panic.message = %message,
            panic.location = %location,
            "Application panicked"
        );
    }));
}

/// Builds an OTLP/HTTP tracer provider.
fn build_otlp_tracer_provider(base_endpoint: &str, resource: Resource) -> Option<SdkTracerProvider> {
    let endpoint = if base_endpoint.ends_with("/v1/traces") {
        base_endpoint.to_string()
    } else {
        format!("{}/v1/traces", base_endpoint)
    };

    let exporter = match SpanExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OTLP trace exporter build failed (endpoint: {}): {:?}", endpoint, e);
                return None;
            }
        };

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

    let exporter = match MetricExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OTLP metric exporter build failed (endpoint: {}): {:?}", endpoint, e);
                return None;
            }
        };

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

    let exporter = match LogExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OTLP log exporter build failed (endpoint: {}): {:?}", endpoint, e);
                return None;
            }
        };

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
    let name = AppConfig::get().otel_service_name.as_deref().unwrap_or("zent-be");
    global::meter(name)
}
