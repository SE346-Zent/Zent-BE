use axum::Router;
use tracing::info;
use std::sync::Arc;

use zent_be::core::state::AppState;
use zent_be::core::config::AppConfig;
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
    let db = infrastructure::database::init_database(cfg).await?;

    // Initialize Valkey cache via infrastructure layer
    let valkey = infrastructure::cache::init_cache(cfg).await
        .expect("Failed to initialize Valkey cache client");

    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url).await;

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::consumers::email::start_email_consumer(rabbitmq.clone()).await;

    // Load lookup tables (roles, account_statuses, etc.) into memory
    let db_conn = db.get_connection().await.expect("Failed to get DB connection for LUT");
    let lookup_tables = core::lookup_tables::LookupTables::load(&db_conn)
        .await
        .expect("Failed to load lookup tables from database");

    // Pre-load email templates into memory cache
    let templates = infrastructure::templates::load_templates().await;

    // Initialize AuthService with dependencies
    let auth_service = services::v1::auth::AuthService::new(
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
    let app_scheduler = infrastructure::scheduler::AppScheduler::new()
        .await
        .expect("Failed to initialize scheduler");

    let user_cleanup_job = infrastructure::cron_tasks::cleanup_pending_users::build_cleanup_job(
        db_conn,
        state.lookup_tables.clone(),
    )
    .expect("Failed to build cleanup job");
    
    app_scheduler.register_job(user_cleanup_job)
        .await
        .expect("Failed to register cleanup job");
        
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

    let app = Router::new()
        .nest("/api/v1", handlers::v1::router())
        .layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let requests_counter = requests_counter.clone();
            let request_duration = request_duration.clone();
            let start = std::time::Instant::now();
            let path = req.uri().path().to_string();
            let method = req.method().to_string();

            async move {
                let response = next.run(req).await;
                let latency = start.elapsed().as_secs_f64();
                let status = response.status().as_u16().to_string();

                let labels = [
                    opentelemetry::KeyValue::new("http.method", method),
                    opentelemetry::KeyValue::new("http.route", path),
                    opentelemetry::KeyValue::new("http.status_code", status),
                ];

                requests_counter.add(1, &labels);
                request_duration.record(latency, &labels);

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
