use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router, middleware,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------
// Infrastructure Mocking
// ---------------------------------------------------------

#[path = "common/mod.rs"]
mod common;
use common::{
    seed_test_db, create_test_app_state, create_test_jwt,
    TEST_ADMIN_ID, TEST_TECHNICIAN_ID,
};

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
    let state = create_test_app_state(db).await;

    let admin_mw = middleware::from_fn_with_state(
        state.clone(),
        zent_be::extractor::role_check::require_role::<zent_be::core::state::AppState>(
            zent_be::entities::roles::Role::Admin,
        ),
    );
    let tech_mw = middleware::from_fn_with_state(
        state.clone(),
        zent_be::extractor::role_check::require_role::<zent_be::core::state::AppState>(
            zent_be::entities::roles::Role::Technician,
        ),
    );

    // Admin routes
    let admin_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/assign",
            post(zent_be::handlers::v1::work_orders::assign),
        )
        .layer(admin_mw);

    // Technician routes
    let tech_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/start",
            post(zent_be::handlers::v1::work_orders::start),
        )
        .route(
            "/api/v1/work_orders/{id}/complete",
            post(zent_be::handlers::v1::work_orders::complete),
        )
        .layer(tech_mw);

    // Shared routes (no middleware)
    Router::new()
        .route(
            "/api/v1/work_orders/{id}/history",
            get(zent_be::handlers::v1::work_orders::history),
        )
        .merge(admin_routes)
        .merge(tech_routes)
        .with_state(state)
}

// ---------------------------------------------------------
// Request Builders
// ---------------------------------------------------------

fn create_json_request_admin(method: http::Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_ADMIN_ID).unwrap());
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));
    req
}

fn create_json_request_tech(method: http::Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap());
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));
    req
}

fn create_empty_request_tech(method: http::Method, uri: &str) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap());
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        )));
    req
}

#[allow(dead_code)]
fn create_empty_request_admin(method: http::Method, uri: &str) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_ADMIN_ID).unwrap());
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
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

        // 1. Pending -> Assigned (Admin)
        let uri_assign = format!("/api/v1/work_orders/{}/assign", wo_id);
        let req_assign = create_json_request_admin(
            http::Method::POST,
            &uri_assign,
            &json!({ "technician_id": Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap() }),
        );
        let r_assign = app.clone().oneshot(req_assign).await.unwrap();
        assert_eq!(r_assign.status(), StatusCode::OK);

        // 2. Assigned -> In Progress (Technician)
        let uri_start = format!("/api/v1/work_orders/{}/start", wo_id);
        let req_start = create_json_request_tech(
            http::Method::POST,
            &uri_start,
            &json!({ "latitude": 10.0, "longitude": 106.0 }),
        );
        let r_start = app.clone().oneshot(req_start).await.unwrap();
        assert_eq!(r_start.status(), StatusCode::OK);

        // 3. In Progress -> Completed (Technician)
        let uri_complete = format!("/api/v1/work_orders/{}/complete", wo_id);
        let req_complete = create_json_request_tech(
            http::Method::POST,
            &uri_complete,
            &json!({
                "mtm": "82K2",
                "serialNumber": "PF3B1234",
                "partChanges": [],
                "diagnosis": "Repaired screen.",
                "latitude": 10.762622,
                "longitude": 106.660172,
                "signatureFileName": "sig.png"
            }),
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
        let req_complete = create_json_request_tech(
            http::Method::POST,
            &uri_complete,
            &json!({
                "mtm": "82K2",
                "serialNumber": "PF3B1234",
                "partChanges": [],
                "diagnosis": "Repaired screen.",
                "latitude": 10.762622,
                "longitude": 106.660172,
                "signatureFileName": "sig.png"
            }),
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
        let req_history = create_empty_request_tech(http::Method::GET, &uri_history);

        let r_history = app.oneshot(req_history).await.unwrap();
        // Endpoint should exist
        assert_eq!(r_history.status(), StatusCode::OK);
    }
}
