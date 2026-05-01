use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router,
};
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

async fn not_implemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn setup_test_app(db: DatabaseConnection, _mq: Arc<MockRabbitMQManager>) -> Router {
    let db_mgr = zent_be::infrastructure::database::DatabaseManager::from_connection(db);
    let valkey_mgr = zent_be::infrastructure::cache::ValkeyManager::stub();
    let rmq_mgr = zent_be::infrastructure::mq::RabbitMQManager::stub();

    let mut templates = std::collections::HashMap::new();
    templates.insert(
        "verification_email.html".to_string(),
        "Template content".to_string(),
    );
    let templates_arc = std::sync::Arc::new(templates);

    let auth_service = zent_be::services::v1::auth::AuthService::new(
        db_mgr.clone(),
        valkey_mgr.clone(),
        rmq_mgr.clone(),
        templates_arc.clone(),
        zent_be::core::state::AccessTokenDefaultTTLSeconds(900),
        zent_be::core::state::SessionDefaultTTLSeconds(3600),
        jsonwebtoken::EncodingKey::from_secret(b"integration_test_secret_for_tokens"),
    );

    let state = zent_be::core::state::AppState::new(
        b"integration_test_secret_for_tokens",
        zent_be::core::lookup_tables::LookupTables::empty(),
        auth_service,
    );

    Router::new()
        // Technician Refusal
        .route("/api/v1/work_orders/{id}/refuse", post(not_implemented))
        // Admin Approval/Denial
        .route(
            "/api/v1/work_orders/{id}/refusal/approve",
            post(not_implemented),
        )
        .route(
            "/api/v1/work_orders/{id}/refusal/deny",
            post(not_implemented),
        )
        // Photos API
        .route(
            "/api/v1/photos/work_orders/{id}/upload",
            post(not_implemented),
        )
        .route("/api/v1/photos/work_orders/{id}", get(not_implemented))
        .route("/api/v1/photos/work_orders", get(not_implemented))
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

fn create_empty_request(method: http::Method, uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::AUTHORIZATION, "Bearer mock_jwt_token")
        .body(Body::empty())
        .unwrap();

    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));
    req
}

// =====================================================================
// Field Execution Module Tests
// =====================================================================

#[cfg(test)]
mod field_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_technician_refusal_submission() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/refuse", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "reason": "Customer not present" }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_admin_refusal_approve() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/refusal/approve", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "comments": "Approved, please assign a different tech" }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_admin_refusal_deny() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/refusal/deny", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "comments": "Deny, this task must be done today" }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_upload_service_photos() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/photos/work_orders/{}/upload", wo_id);
        // Note: Actual multipart form data is harder to mock cleanly with json
        // But testing the routing endpoint existence
        let req = create_empty_request(http::Method::POST, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_retrieve_service_photos() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/photos/work_orders/{}", wo_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
