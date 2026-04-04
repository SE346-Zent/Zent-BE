use axum::Router;
use sea_orm::{Database, ConnectOptions};
use std::time::Duration;
use tracing::info;


#[macro_use]
pub mod macros;
pub mod config;
pub mod entities;
pub mod extractor;
pub mod handlers;
pub mod infrastructure;
pub mod model;
pub mod services;
pub mod state;

use crate::state::AppState;
use crate::config::AppConfig;
use migration::{MigratorTrait, Migrator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured JSON logs + OpenTelemetry pipeline
    infrastructure::tracing::init_tracing();

    tracing::info!("Server starting...");
    
    // Initialize central configuration manager
    AppConfig::init();
    let cfg = AppConfig::get();

    // Connect to database
    let mut opt = ConnectOptions::new(&cfg.database_url);

    opt.max_connections(100)
       .min_connections(5)
       .connect_timeout(Duration::from_secs(8))
       .acquire_timeout(Duration::from_secs(8))
       .idle_timeout(Duration::from_secs(8))
       .max_lifetime(Duration::from_secs(8))
       .sqlx_logging(false);
    
    let db = Database::connect(opt).await?;
    tracing::info!("Running db migrations");
    Migrator::up(&db, None).await.expect("Failed to run db migrations");
    tracing::info!("DB migrations applied successfully");


    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url)
        .await
        .expect("Failed to initialize RabbitMQ Message Queue Architecture");

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::mq::start_email_consumer(rabbitmq.clone()).await;

    let state = AppState::new(
        cfg.jwt_sign_key.as_bytes(),
        db,
        Some(rabbitmq),
        cfg.access_token_ttl_seconds,
        cfg.session_ttl_seconds,
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
