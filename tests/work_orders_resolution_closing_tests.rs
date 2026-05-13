use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::post,
    Router,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
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
use common::seed_test_db;

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

    let luts = zent_be::core::lookup_tables::LookupTables::empty();

    let state = zent_be::core::state::AppState::new(
        b"test_secret",
        luts,
        db.clone(),
        None,
        None,
        std::collections::HashMap::new(),
        zent_be::core::state::AccessTokenDefaultTTLSeconds(900),
        zent_be::core::state::SessionDefaultTTLSeconds(3600),
    );

    Router::new()
        .route(
            "/api/v1/work_orders/{id}/complete",
            post(zent_be::handlers::v1::work_orders::complete),
        )
        .route(
            "/api/v1/signatures/work_orders/{id}/upload",
            post(zent_be::handlers::v1::media::upload_work_order_signature),
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
// Resolution and Closing Module Tests
// =====================================================================

#[cfg(test)]
mod resolution_closing_tests {
    use super::*;

    #[tokio::test]
    async fn test_upload_customer_signature() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/signatures/work_orders/{}/upload", wo_id);
        let req = create_empty_request(http::Method::POST, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Assert linkage: signature_file_name is updated in WorkOrderClosingForms
        let form = zent_be::entities::work_order_closing_forms::Entity::find()
            .filter(zent_be::entities::work_order_closing_forms::Column::WorkOrderId.eq(wo_id))
            .one(&db)
            .await
            .unwrap();

        assert!(form.is_some(), "Expected a closing form for the work order");
        assert!(
            !form.unwrap().signature_file_name.is_empty(),
            "Expected a signature file name linkage to be made"
        );
    }

    #[tokio::test]
    async fn test_finalize_work_order_success() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/complete", wo_id);
        // Complete payload
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({
                "mtm": "82K2",
                "serialNumber": "PF3B1234",
                "partChanges": [],
                "diagnosis": "Repaired screen. System passed tests.",
                "latitude": 10.762622,
                "longitude": 106.660172,
                "signatureFileName": "sig_file_123.png"
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let form = zent_be::entities::work_order_closing_forms::Entity::find()
            .filter(zent_be::entities::work_order_closing_forms::Column::WorkOrderId.eq(wo_id))
            .one(&db)
            .await
            .unwrap();

        assert!(form.is_some(), "WorkOrderClosingForm must be created");
        assert_eq!(form.unwrap().signature_file_name, "sig_file_123.png");
    }
}
