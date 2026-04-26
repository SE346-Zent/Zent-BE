use axum::Router;
use tracing::info;
use std::sync::Arc;

use zent_be::core::state::AppState;
use zent_be::core::config::AppConfig;
use zent_be::{core, handlers, infrastructure, services};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured JSON logs + OpenTelemetry pipeline
    infrastructure::tracing::init_tracing();

    tracing::info!("Server starting...");
    
    // Initialize central configuration manager
    AppConfig::init();
    let cfg = AppConfig::get();

    // Initialize database (MySQL) via infrastructure layer
    let db = infrastructure::database::init_database(cfg).await?;

    // Initialize Valkey cache via infrastructure layer
    let valkey = infrastructure::cache::init_cache(cfg).await
        .expect("Failed to initialize Valkey cache client");

    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url).await;

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::mq::start_email_consumer(rabbitmq.clone()).await;

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
    infrastructure::scheduler::start_scheduler(db_conn, state.lookup_tables.clone()).await
        .expect("Failed to start maintenance scheduler");

    // Apply strict nested modular Router mapping with dynamic dispatch boundaries safely inside axum
    let app = Router::new()
        .nest("/api/v1", handlers::v1::router())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    // Starting Server with Graceful Shutdown hooks
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Flush remaining OTel spans
    infrastructure::tracing::shutdown_tracing();

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
