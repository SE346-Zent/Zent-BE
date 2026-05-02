use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::post,
    Router,
};
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use migration::{Migrator, MigratorTrait};
use zent_be::entities::{roles, account_status};
use sea_orm::{Database, DatabaseConnection};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------
// Infrastructure Mocking
// ---------------------------------------------------------

#[derive(Clone, Default)]
pub struct MockRabbitMQManager {
    pub is_stub: bool,
    pub published_messages: Arc<Mutex<Vec<(String, Value)>>>,
}

async fn mock_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:").await.unwrap()
}

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

async fn seed_test_db(db: &DatabaseConnection) {
    let _ = roles::ActiveModel { id: Set(1), name: Set("Admin".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(2), name: Set("Manager".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(3), name: Set("Technician".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(4), name: Set("Dispatcher".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(5), name: Set("Customer".to_string()) }.insert(db).await;

    let _ = account_status::ActiveModel { id: Set(1), name: Set("Pending".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(2), name: Set("Active".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(3), name: Set("Inactive".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(4), name: Set("Locked".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(5), name: Set("Terminated".to_string()) }.insert(db).await;
}

async fn setup_test_app(db: DatabaseConnection, _mq: Arc<MockRabbitMQManager>) -> Router {
    let _ = tracing_subscriber::fmt::try_init();
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;

    let mut templates = std::collections::HashMap::new();
    templates.insert("verification_email.html".to_string(), "Template content".to_string());

    let auth_service = zent_be::services::v1::auth::AuthService::new(
        db.clone(),
        None,
        None,
        std::sync::Arc::new(templates),
        zent_be::core::state::AccessTokenDefaultTTLSeconds(900),
        zent_be::core::state::SessionDefaultTTLSeconds(3600),
        jsonwebtoken::EncodingKey::from_secret(b"integration_test_secret_for_tokens"),
    );

    let luts = std::sync::Arc::new(zent_be::core::lookup_tables::LookupTables::empty());
    
    let work_order_service = zent_be::services::v1::work_orders::WorkOrderService::new(
        db.clone(),
        luts.clone(),
        None,
        None,
    );
    
    let media_service = zent_be::services::v1::media::MediaService::new(
        db.clone(),
        None,
        None,
    );

    let state = zent_be::core::state::AppState::new(
        b"integration_test_secret_for_tokens",
        zent_be::core::lookup_tables::LookupTables::empty(),
        auth_service,
        work_order_service,
        media_service,
    );

    Router::new()
        .route("/api/v1/work_orders/{id}/parts", post(zent_be::handlers::v1::work_orders::add_parts))
        .with_state(state)
}

// ---------------------------------------------------------
// Request Builders
// ---------------------------------------------------------

fn create_json_request(method: http::Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, "Bearer mock_jwt_token")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));
    req
}

// =====================================================================
// Parts Integration Module Tests
// =====================================================================

#[cfg(test)]
mod parts_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_add_parts_to_work_order() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/parts", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({
                "parts": [
                    {
                        "part_id": "part_123",
                        "action": "installed",
                        "quantity": 1
                    },
                    {
                        "part_id": "part_456",
                        "action": "removed",
                        "quantity": 1
                    }
                ]
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_modify_completed_work_order_parts() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/parts", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({
                "parts": [
                    {
                        "part_id": "part_123",
                        "action": "installed",
                        "quantity": 1
                    }
                ]
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        // In actual implementation, we'd mock the DB state to simulate a Completed order
        // Here we just test routing
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
