use axum::Router;
use tracing::info;
use std::sync::Arc;
use std::collections::HashMap;

use zent_be::core::state::AppState;
use zent_be::core::config::AppConfig;
use zent_be::infrastructure::cache::ValkeyClient;
use zent_be::infrastructure::scheduler::AppScheduler;
use zent_be::{core, handlers, infrastructure};
use sea_orm::DatabaseConnection;
use lapin::Connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize central configuration manager
    AppConfig::init();
    let cfg = AppConfig::get();

    // Structured JSON logs + OpenTelemetry pipeline
    infrastructure::observability::init_tracing();

    tracing::info!("Server starting...");
    
    // Initialize database (MySQL) via infrastructure layer
    let db: DatabaseConnection = infrastructure::database::init_database(cfg).await?;

    // Initialize MongoDB via infrastructure layer
    let mongodb_client = infrastructure::mongodb::init_mongodb(cfg).await?;
    let db_name = mongodb_client
        .default_database()
        .map(|db| db.name().to_string())
        .ok_or("MongoDB connection string must include a database name")?;
    let mongodb_database = mongodb_client.database(&db_name);

    // Run MongoDB migrations
    mongodb_migration::run_migrations(&mongodb_database).await?;

    // Initialize Valkey cache via infrastructure layer
    let valkey: Option<Arc<ValkeyClient>> = match infrastructure::cache::init_cache(cfg).await {
        Ok(v) => Some(Arc::new(v)),
        Err(e) => {
            tracing::error!("Failed to initialize Valkey cache: {}. Continuing in degraded mode.", e);
            None
        }
    };

    // Connect to RabbitMQ using configured URI mapping efficiently
    let rabbitmq: Option<Arc<Connection>> = infrastructure::mq::init_rabbitmq(&cfg.rabbitmq_url).await
        .map(Arc::new)
        .ok();
    if rabbitmq.is_none() {
        tracing::warn!("RabbitMQ client not initialized - continuing in degraded mode");
    }

    // Start background asynchronous AMQP email consumer pool globally
    infrastructure::consumers::email::start_email_consumer(rabbitmq.clone()).await;

    // Load lookup tables (roles, account_statuses, etc.) into memory
    let lookup_tables = core::lookup_tables::LookupTables::load(&db)
        .await
        .expect("Failed to load lookup tables from database");

    // Pre-load email templates into memory cache from the configured directory
    let templates: HashMap<String, String> = infrastructure::templates::load_templates(&cfg.template_dir).await;

    // Initialize AppState with directly injected infrastructure
    let state = AppState::new(
        cfg.jwt_sign_key.as_bytes(),
        lookup_tables.clone(),
        db.clone(),
        mongodb_database.clone(),
        valkey.clone(),
        rabbitmq.clone(),
        templates.clone(),
        core::state::AccessTokenDefaultTTLSeconds(cfg.access_token_ttl_seconds),
        core::state::SessionDefaultTTLSeconds(cfg.session_ttl_seconds),
    );

    // Start background asynchronous AMQP work order consumer pool
    infrastructure::consumers::work_order::start_work_order_consumer(state.clone()).await;

    // Start background FCM push notification consumer
    infrastructure::consumers::fcm::start_fcm_consumer(rabbitmq.clone(), db.clone()).await;

    // Start background notification consumer (Phase 2 of outbox pattern)
    infrastructure::consumers::notification::start_notification_consumer(state.clone()).await;

    // Start background cron scheduler for maintenance tasks using pre-loaded LUT
    let app_scheduler: AppScheduler = infrastructure::scheduler::AppScheduler::new()
        .await
        .expect("Failed to initialize scheduler");

    let user_cleanup_job = infrastructure::cron_tasks::cleanup_pending_users::build_cleanup_job(
        db.clone(),
        state.lookup_tables.clone(),
    )
    .expect("Failed to build cleanup job");

    let session_cleanup_job = infrastructure::cron_tasks::cleanup_sessions::build_cleanup_job(
        db.clone(),
    )
    .expect("Failed to build session cleanup job");

    let metrics_job = infrastructure::cron_tasks::observability_metrics::build_metrics_job()
        .expect("Failed to build metrics collection job");
        
    let auto_assign_job = infrastructure::cron_tasks::cleanup_work_order::clean_up_work_order_job(
        db.clone(),
        state.lookup_tables.clone(),
        state.valkey.clone(),
        state.rabbitmq.clone(),
    ).expect("Failed to build auto assign job");

    let escalation_job = infrastructure::cron_tasks::escalation::build_escalation_job(
        db.clone(),
        state.lookup_tables.clone(),
        state.mongodb.clone(),
        state.valkey.clone(),
    ).expect("Failed to build escalation job");
    
    app_scheduler.register_job(user_cleanup_job)
        .await
        .expect("Failed to register cleanup job");

    app_scheduler.register_job(session_cleanup_job)
        .await
        .expect("Failed to register session cleanup job");

    app_scheduler.register_job(metrics_job)
        .await
        .expect("Failed to register metrics job");
        
    app_scheduler.register_job(auto_assign_job)
        .await
        .expect("Failed to register auto assign job");

    app_scheduler.register_job(escalation_job)
        .await
        .expect("Failed to register escalation job");

    let outbox_cleanup_job = infrastructure::cron_tasks::cleanup_outbox::clean_up_outbox_job(
        db.clone(),
    ).expect("Failed to build outbox cleanup job");

    app_scheduler.register_job(outbox_cleanup_job)
        .await
        .expect("Failed to register outbox cleanup job");

    // Register outbox relay job (runs every 10 seconds)
    let relay_job = infrastructure::cron_tasks::relay_outbox::relay_outbox_job(
        db.clone(),
        rabbitmq.clone(),
    ).expect("Failed to build outbox relay job");

    app_scheduler.register_job(relay_job)
        .await
        .expect("Failed to register outbox relay job");
        
    app_scheduler.start()
        .await
        .expect("Failed to start scheduler");

    // Apply strict nested modular Router mapping with dynamic dispatch boundaries safely inside axum
    let meter = infrastructure::observability::meter();
    let requests_counter = meter
        .u64_counter("http_req_total")
        .with_description("Total number of HTTP requests")
        .build();

    let request_duration = meter
        .f64_histogram("http_req_duration")
        .with_description("Time taken to process HTTP requests")
        .with_unit("s")
        .build();

    let active_requests = meter
        .i64_up_down_counter("http_active_req")
        .with_description("Number of active HTTP requests")
        .build();

    let app = Router::new()
        .route("/chat", axum::routing::get(handlers::v1::chat::ws::ws_handler))
        .nest("/api/v1", handlers::v1::router(state.clone()))
        .route_layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let requests_counter = requests_counter.clone();
            let request_duration = request_duration.clone();
            let active_requests = active_requests.clone();
            
            let start = std::time::Instant::now();
            let path = req
                .extensions()
                .get::<axum::extract::MatchedPath>()
                .map_or_else(|| "unmatched".to_string(), |mp| mp.as_str().to_string());
            let method = req.method().to_string();

            async move {
                active_requests.add(1, &[]);

                let response = next.run(req).await;
                
                let latency = start.elapsed().as_secs_f64();
                let status = response.status().as_u16().to_string();

                let labels = [
                    opentelemetry::KeyValue::new("method", method),
                    opentelemetry::KeyValue::new("route", path),
                    opentelemetry::KeyValue::new("status", status),
                ];

                requests_counter.add(1, &labels);
                request_duration.record(latency, &labels);
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
