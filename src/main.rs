use axum::Router;
use tracing::info;
use std::sync::Arc;
use std::collections::HashMap;

use zent_be::core::state::AppState;
use zent_be::core::config::AppConfig;
use zent_be::infrastructure::database::DatabasePool;
use zent_be::infrastructure::cache::ValkeyClient;
use zent_be::infrastructure::mq::RabbitMQClient;
use zent_be::infrastructure::scheduler::AppScheduler;
use zent_be::{core, handlers, infrastructure, services};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize central configuration manager
    AppConfig::init();
    let cfg = AppConfig::get();

    // Structured JSON logs + OpenTelemetry pipeline
    infrastructure::observability::init_tracing();

    tracing::info!("Server starting...");
    
    // Initialize database (MySQL) via infrastructure layer
    let db: Arc<DatabasePool> = infrastructure::database::init_database(cfg).await?;

    // Initialize Valkey cache via infrastructure layer
    let valkey: Arc<ValkeyClient> = infrastructure::cache::init_cache(cfg).await
        .expect("Failed to initialize Valkey cache client");

    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq: Arc<RabbitMQClient> = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url).await
        .expect("Failed to initialize RabbitMQ client");

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::consumers::email::start_email_consumer(rabbitmq.clone()).await;

    // Load lookup tables (roles, account_statuses, etc.) into memory
    let db_conn = db.get_connection().await.expect("Failed to get DB connection for LUT");
    let lookup_tables = core::lookup_tables::LookupTables::load(&db_conn)
        .await
        .expect("Failed to load lookup tables from database");

    // Pre-load email templates into memory cache
    let templates: HashMap<String, String> = infrastructure::templates::load_templates().await;

    // Initialize AuthService with dependencies
    let auth_service = services::v1::init_auth_service(
        db.clone(),
        valkey.clone(),
        rabbitmq.clone(),
        Arc::new(templates.clone()),
        core::state::AccessTokenDefaultTTLSeconds(cfg.access_token_ttl_seconds),
        core::state::SessionDefaultTTLSeconds(cfg.session_ttl_seconds),
        jsonwebtoken::EncodingKey::from_secret(cfg.jwt_sign_key.as_bytes()),
    );

    let state = AppState::new(
        cfg.jwt_sign_key.as_bytes(),
        db.clone(),
        valkey.clone(),
        rabbitmq.clone(),
        cfg.access_token_ttl_seconds,
        cfg.session_ttl_seconds,
        lookup_tables.clone(),
        templates,
        auth_service,
    );

    // Start background cron scheduler for maintenance tasks using pre-loaded LUT
    let app_scheduler: AppScheduler = infrastructure::scheduler::AppScheduler::new()
        .await
        .expect("Failed to initialize scheduler");

    let user_cleanup_job = infrastructure::cron_tasks::cleanup_pending_users::build_cleanup_job(
        db_conn,
        state.lookup_tables.clone(),
    )
    .expect("Failed to build cleanup job");

    let metrics_job = infrastructure::cron_tasks::observability_metrics::build_metrics_job()
        .expect("Failed to build metrics collection job");
    
    app_scheduler.register_job(user_cleanup_job)
        .await
        .expect("Failed to register cleanup job");

    app_scheduler.register_job(metrics_job)
        .await
        .expect("Failed to register metrics job");
        
    app_scheduler.start()
        .await
        .expect("Failed to start scheduler");

    // Apply strict nested modular Router mapping with dynamic dispatch boundaries safely inside axum
    let meter = infrastructure::observability::meter();
    let requests_counter = meter
        .u64_counter("http.server.request.count")
        .with_description("Total number of HTTP requests")
        .build();

    let request_duration = meter
        .f64_histogram("http.server.request.duration")
        .with_description("Time taken to process HTTP requests")
        .with_unit("s")
        .build();

    let active_requests = meter
        .i64_up_down_counter("http.server.active_requests")
        .with_description("Number of active HTTP requests")
        .build();

    let request_size = meter
        .u64_histogram("http.server.request.size")
        .with_description("Size of HTTP request bodies")
        .with_unit("By")
        .build();

    let response_size = meter
        .u64_histogram("http.server.response.size")
        .with_description("Size of HTTP response bodies")
        .with_unit("By")
        .build();

    let app = Router::new()
        .nest("/api/v1", handlers::v1::router())
        .route_layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let requests_counter = requests_counter.clone();
            let request_duration = request_duration.clone();
            let active_requests = active_requests.clone();
            let request_size = request_size.clone();
            let response_size = response_size.clone();
            
            let start = std::time::Instant::now();
            let path = req
                .extensions()
                .get::<axum::extract::MatchedPath>()
                .map_or_else(|| req.uri().path().to_string(), |mp| mp.as_str().to_string());
            let method = req.method().to_string();
            
            // Capture request size
            let req_content_length = req.headers()
                .get(http::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            async move {
                active_requests.add(1, &[]);
                request_size.record(req_content_length, &[opentelemetry::KeyValue::new("http.method", method.clone())]);

                let response = next.run(req).await;
                
                let latency = start.elapsed().as_secs_f64();
                let status = response.status().as_u16().to_string();
                
                // Capture response size
                let res_content_length = response.headers()
                    .get(http::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let labels = [
                    opentelemetry::KeyValue::new("http.method", method),
                    opentelemetry::KeyValue::new("http.route", path),
                    opentelemetry::KeyValue::new("http.status_code", status),
                ];

                requests_counter.add(1, &labels);
                request_duration.record(latency, &labels);
                response_size.record(res_content_length, &labels);
                active_requests.add(-1, &[]);

                response
            }
        }))
        .layer(tower_http::trace::TraceLayer::new_for_http()
            .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
            .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO))
        )
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    // Starting Server with Graceful Shutdown hooks
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Flush remaining OTel spans
    infrastructure::observability::shutdown_tracing();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        },
    }
}
