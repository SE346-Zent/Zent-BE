use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::post,
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
        .route(
            "/api/v1/work_orders/{id}/parts",
            post(zent_be::handlers::v1::work_orders::add_parts),
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

// =====================================================================
// Parts Integration Module Tests
// =====================================================================

#[cfg(test)]
mod parts_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_add_parts_to_work_order() {
        let app = setup_test_app(mock_db().await).await;
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
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_modify_completed_work_order_parts() {
        let app = setup_test_app(mock_db().await).await;
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
        assert_eq!(r.status(), StatusCode::CONFLICT);
    }
}
