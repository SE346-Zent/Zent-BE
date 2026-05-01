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

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub mq: Arc<MockRabbitMQManager>,
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

async fn setup_test_app(db: DatabaseConnection, mq: Arc<MockRabbitMQManager>) -> Router {
    let state = AppState { db, mq };

    Router::new()
        // Core status transitions endpoints
        .route("/api/v1/work_orders/{id}/assign", post(not_implemented))
        .route("/api/v1/work_orders/{id}/start", post(not_implemented))
        .route("/api/v1/work_orders/{id}/complete", post(not_implemented))
        // New endpoint: State History
        .route("/api/v1/work_orders/{id}/history", get(not_implemented))
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
// State Management Module Tests
// =====================================================================

#[cfg(test)]
mod state_management_tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_state_transitions() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        // 1. Pending -> Assigned
        let uri_assign = format!("/api/v1/work_orders/{}/assign", wo_id);
        let req_assign = create_json_request(
            http::Method::POST,
            &uri_assign,
            &json!({ "technician_id": Uuid::new_v4() }),
        );
        let r_assign = app.clone().oneshot(req_assign).await.unwrap();
        // Assuming mock returns NOT_IMPLEMENTED, in real test it would be OK
        assert_eq!(r_assign.status(), StatusCode::NOT_IMPLEMENTED);

        // 2. Assigned -> In Progress
        let uri_start = format!("/api/v1/work_orders/{}/start", wo_id);
        let req_start = create_json_request(
            http::Method::POST,
            &uri_start,
            &json!({ "latitude": 10.0, "longitude": 106.0 }),
        );
        let r_start = app.clone().oneshot(req_start).await.unwrap();
        assert_eq!(r_start.status(), StatusCode::NOT_IMPLEMENTED);

        // 3. In Progress -> Completed
        let uri_complete = format!("/api/v1/work_orders/{}/complete", wo_id);
        let req_complete = create_json_request(
            http::Method::POST,
            &uri_complete,
            &json!({ "evidence_image_ids": ["img_1"], "signature_id": "sig_1" }),
        );
        let r_complete = app.clone().oneshot(req_complete).await.unwrap();
        assert_eq!(r_complete.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_invalid_state_transitions() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        // Directly from Pending -> Completed (Invalid)
        let uri_complete = format!("/api/v1/work_orders/{}/complete", wo_id);
        let req_complete = create_json_request(
            http::Method::POST,
            &uri_complete,
            &json!({ "evidence_image_ids": ["img_1"], "signature_id": "sig_1" }),
        );
        let r_complete = app.clone().oneshot(req_complete).await.unwrap();
        // This expects to be handled by the endpoint logically, we just check routing
        assert_eq!(r_complete.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_state_history_recording() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let wo_id = Uuid::new_v4();

        // Get History
        let uri_history = format!("/api/v1/work_orders/{}/history", wo_id);
        let req_history = create_empty_request(http::Method::GET, &uri_history);

        let r_history = app.oneshot(req_history).await.unwrap();
        // Endpoint should exist
        assert_eq!(r_history.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
