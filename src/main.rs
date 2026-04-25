use axum::Router;
use tracing::info;

pub mod core;
pub mod entities;
pub mod extractor;
pub mod handlers;
pub mod infrastructure;
pub mod model;
pub mod repository;
pub mod services;

use crate::core::state::AppState;
use crate::core::config::AppConfig;

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
    let valkey = infrastructure::cache::init_cache(cfg)
        .expect("Failed to initialize Valkey cache client");

    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url)
        .await
        .expect("Failed to initialize RabbitMQ Message Queue Architecture");

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::mq::start_email_consumer(rabbitmq.clone()).await;

    // Start background cron scheduler for maintenance tasks
    infrastructure::scheduler::start_scheduler(db.clone()).await
        .expect("Failed to start maintenance scheduler");

    // Load lookup tables (roles, account_statuses, etc.) into memory
    let lookup_tables = core::lookup_tables::LookupTables::load(&db)
        .await
        .expect("Failed to load lookup tables from database");

    let state = AppState::new(
        cfg.jwt_sign_key.as_bytes(),
        db,
        Some(valkey),
        Some(rabbitmq),
        cfg.access_token_ttl_seconds,
        cfg.session_ttl_seconds,
        lookup_tables,
    );

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
