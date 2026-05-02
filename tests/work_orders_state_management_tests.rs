use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::*;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use sea_orm::{Database, DatabaseConnection};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
use zent_be::entities::{account_status, roles};

// ---------------------------------------------------------
// Infrastructure Mocking
// ---------------------------------------------------------

#[path = "common/mod.rs"]
mod common;
use common::{seed_test_db, WorkOrderTestState};

async fn mock_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:").await.unwrap()
}

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

async fn setup_test_app(db: DatabaseConnection) -> Router {
    let _ = tracing_subscriber::fmt::try_init();
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;

    let luts = std::sync::Arc::new(zent_be::core::lookup_tables::LookupTables::empty());

    let work_order_service =
        std::sync::Arc::new(zent_be::services::v1::work_orders::WorkOrderService::new(
            db.clone(),
            luts.clone(),
            None,
            None,
        ));

    let media_service = std::sync::Arc::new(zent_be::services::v1::core::media::MediaService::new(
        db.clone(),
        None,
        None,
    ));

    let state = WorkOrderTestState {
        work_order_service,
        media_service,
    };

    Router::new()
        // Core status transitions endpoints
        .route(
            "/api/v1/work_orders/{id}/assign",
            post(zent_be::handlers::v1::work_orders::assign),
        )
        .route(
            "/api/v1/work_orders/{id}/start",
            post(zent_be::handlers::v1::work_orders::start),
        )
        .route(
            "/api/v1/work_orders/{id}/complete",
            post(zent_be::handlers::v1::work_orders::complete),
        )
        // New endpoint: State History
        .route(
            "/api/v1/work_orders/{id}/history",
            get(zent_be::handlers::v1::work_orders::history),
        )
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
        let app = setup_test_app(mock_db().await).await;
        let wo_id = Uuid::new_v4();

        // 1. Pending -> Assigned
        let uri_assign = format!("/api/v1/work_orders/{}/assign", wo_id);
        let req_assign = create_json_request(
            http::Method::POST,
            &uri_assign,
            &json!({ "technician_id": Uuid::new_v4() }),
        );
        let r_assign = app.clone().oneshot(req_assign).await.unwrap();
        assert_eq!(r_assign.status(), StatusCode::OK);

        // 2. Assigned -> In Progress
        let uri_start = format!("/api/v1/work_orders/{}/start", wo_id);
        let req_start = create_json_request(
            http::Method::POST,
            &uri_start,
            &json!({ "latitude": 10.0, "longitude": 106.0 }),
        );
        let r_start = app.clone().oneshot(req_start).await.unwrap();
        assert_eq!(r_start.status(), StatusCode::OK);

        // 3. In Progress -> Completed
        let uri_complete = format!("/api/v1/work_orders/{}/complete", wo_id);
        let req_complete = create_json_request(
            http::Method::POST,
            &uri_complete,
            &json!({ "evidence_image_ids": ["img_1"], "signature_id": "sig_1" }),
        );
        let r_complete = app.clone().oneshot(req_complete).await.unwrap();
        assert_eq!(r_complete.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_state_transitions() {
        let app = setup_test_app(mock_db().await).await;
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
        assert_eq!(r_complete.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_state_history_recording() {
        let app = setup_test_app(mock_db().await).await;
        let wo_id = Uuid::new_v4();

        // Get History
        let uri_history = format!("/api/v1/work_orders/{}/history", wo_id);
        let req_history = create_empty_request(http::Method::GET, &uri_history);

        let r_history = app.oneshot(req_history).await.unwrap();
        // Endpoint should exist
        assert_eq!(r_history.status(), StatusCode::OK);
    }
}
