use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
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
    
    let work_order_service = std::sync::Arc::new(zent_be::services::v1::work_orders::WorkOrderService::new(
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
        .route("/api/v1/work_orders/{id}/refuse", post(zent_be::handlers::v1::work_orders::refuse))
        .route("/api/v1/work_orders/{id}/refusal/approve", post(zent_be::handlers::v1::work_orders::approve_refusal))
        .route("/api/v1/work_orders/{id}/refusal/deny", post(zent_be::handlers::v1::work_orders::deny_refusal))
        .route("/api/v1/photos/work_orders/{id}/upload", post(zent_be::handlers::v1::media::upload_work_order_photo))
        .route("/api/v1/photos/work_orders/{id}", get(zent_be::handlers::v1::media::get_work_order_photo))
        .route("/api/v1/photos/work_orders", get(zent_be::handlers::v1::media::list_work_order_photos))
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
        let app = setup_test_app(mock_db().await).await;
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
        let app = setup_test_app(mock_db().await).await;
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
        let app = setup_test_app(mock_db().await).await;
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
        let app = setup_test_app(mock_db().await).await;
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
        let app = setup_test_app(mock_db().await).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/photos/work_orders/{}", wo_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
