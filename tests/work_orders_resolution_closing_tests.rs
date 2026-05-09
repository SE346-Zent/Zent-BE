use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::post,
    Router, middleware,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};
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
    TEST_TECHNICIAN_ID, seed_work_order, seed_image_link,
};

use std::sync::Once;

static INIT: Once = Once::new();

async fn mock_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:").await.unwrap()
}

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

async fn setup_test_app(db: DatabaseConnection) -> Router {
    INIT.call_once(|| {
        zent_be::core::config::AppConfig::init();
    });
    let _ = tracing_subscriber::fmt::try_init();
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;
    let state = create_test_app_state(db).await;

    let tech_mw = middleware::from_fn_with_state(
        state.clone(),
        zent_be::extractor::role_check::require_role::<zent_be::core::state::AppState>(
            zent_be::entities::roles::Role::Technician,
        ),
    );

    // Complete work order — technician
    let tech_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/complete",
            post(zent_be::handlers::v1::work_orders::complete),
        )
        .layer(tech_mw.clone());

    // Signature upload — technician
    let media_routes = Router::new()
        .route(
            "/api/v1/media/work_orders/{id}/closing_form/signature",
            post(zent_be::handlers::v1::media::upload_closing_form_signature),
        )
        .layer(tech_mw);

    Router::new()
        .merge(tech_routes)
        .merge(media_routes)
        .with_state(state)
}

// ---------------------------------------------------------
// Request Builders
// ---------------------------------------------------------

fn create_json_request(method: http::Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
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

fn create_multipart_signature_request(method: http::Method, uri: &str) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap());
    let boundary = "---------------------------1234567890";
    let mut body = Vec::new();
    
    // file field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"signature.png\"\r\n");
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(b"fake_signature_data");
    body.extend_from_slice(b"\r\n");

    // latitude field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"latitude\"\r\n\r\n");
    body.extend_from_slice(b"10.762622");
    body.extend_from_slice(b"\r\n");

    // longitude field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"longitude\"\r\n\r\n");
    body.extend_from_slice(b"106.660172");
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, format!("multipart/form-data; boundary={}", boundary))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(body))
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
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await; // 3 = In Progress

        let uri = format!("/api/v1/media/work_orders/{}/closing_form/signature", wo_id);
        let req = create_multipart_signature_request(http::Method::POST, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Assert linkage: a link with phase "signature" exists in work_order_image_links
        let link = zent_be::entities::work_order_image_links::Entity::find()
            .filter(zent_be::entities::work_order_image_links::Column::WorkOrderId.eq(wo_id))
            .filter(zent_be::entities::work_order_image_links::Column::Phase.eq("signature"))
            .one(&db)
            .await
            .unwrap();

        assert!(link.is_some(), "Expected a signature photo link for the work order");
    }

    #[tokio::test]
    async fn test_finalize_work_order_success() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await; // 3 = In Progress
        
        // Seed ALL 4 mandatory photos
        seed_image_link(&db, wo_id, "pre-assembly").await;
        seed_image_link(&db, wo_id, "disassembled").await;
        seed_image_link(&db, wo_id, "post-assembly").await;
        seed_image_link(&db, wo_id, "signature").await;

        let uri = format!("/api/v1/work_orders/{}/complete", wo_id);
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

    #[tokio::test]
    async fn test_complete_work_order_without_signature_photo_fails() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await; // 3 = In Progress
        
        // Seed only 3 photos, missing "signature" phase
        seed_image_link(&db, wo_id, "pre-assembly").await;
        seed_image_link(&db, wo_id, "disassembled").await;
        seed_image_link(&db, wo_id, "post-assembly").await;

        let uri = format!("/api/v1/work_orders/{}/complete", wo_id);
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({
                "mtm": "82K2",
                "serialNumber": "PF3B1234",
                "partChanges": [],
                "diagnosis": "Repaired.",
                "latitude": 10.762622,
                "longitude": 106.660172,
                "signatureFileName": "sig.png"
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        
        // Should FAIL with 400 because "signature" photo is missing in business logic
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        
        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("Minimum 1 photo required for phase(s): signature"));
    }

    #[tokio::test]
    async fn test_complete_work_order_without_signature_filename_fails_validation() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await;

        let uri = format!("/api/v1/work_orders/{}/complete", wo_id);
        // Request payload with empty signatureFileName
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({
                "mtm": "82K2",
                "serialNumber": "PF3B1234",
                "partChanges": [],
                "diagnosis": "Repaired.",
                "latitude": 10.762622,
                "longitude": 106.660172,
                "signatureFileName": ""
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        
        // Should FAIL with 400 because signatureFileName is mandatory in CompleteWorkOrderRequest validation
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }
}
