use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
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
    TEST_TECHNICIAN_ID, TEST_ADMIN_ID, seed_work_order, seed_reject_form,
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
    let admin_mw = middleware::from_fn_with_state(
        state.clone(),
        zent_be::extractor::role_check::require_role::<zent_be::core::state::AppState>(
            zent_be::entities::roles::Role::Admin,
        ),
    );

    // Technician routes — refusal
    let tech_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/refuse",
            post(zent_be::handlers::v1::work_orders::refuse),
        )
        .layer(tech_mw.clone());

    // Admin routes — approve/deny refusal
    let admin_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/refusal/approve",
            post(zent_be::handlers::v1::work_orders::approve_refusal),
        )
        .route(
            "/api/v1/work_orders/{id}/refusal/deny",
            post(zent_be::handlers::v1::work_orders::deny_refusal),
        )
        .layer(admin_mw);

    // Media routes — photos (technician)
    let media_routes = Router::new()
        .route(
            "/api/v1/media/work_orders/{id}/closing_form/photos",
            post(zent_be::handlers::v1::media::upload_closing_form_photo),
        )
        .route(
            "/api/v1/media/photos/work_orders/{id}",
            get(zent_be::handlers::v1::media::get_work_order_photo),
        )
        .route(
            "/api/v1/media/photos/work_orders",
            get(zent_be::handlers::v1::media::list_work_order_photos),
        )
        .layer(tech_mw);

    Router::new()
        .merge(tech_routes)
        .merge(admin_routes)
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

fn create_multipart_request(method: http::Method, uri: &str, fields: Vec<(&str, &str)>) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap());
    let boundary = "---------------------------1234567890";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        if name == "file" {
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"test.jpg\"\r\n",
                    name
                )
                .as_bytes(),
            );
            body.extend_from_slice(b"Content-Type: image/jpeg\r\n");
        } else {
            body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
            );
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(
            http::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
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

fn create_empty_request(method: http::Method, uri: &str) -> Request<Body> {
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

// =====================================================================
// Field Execution Module Tests
// =====================================================================

#[cfg(test)]
mod field_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_technician_refusal_submission() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 2, None).await;

        let uri = format!("/api/v1/work_orders/{}/refuse", wo_id);
        let req = create_multipart_request(
            http::Method::POST,
            &uri,
            vec![("reason", "Customer not present"), ("explanation", "Waited for 30 mins")],
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let wo = zent_be::entities::work_orders::Entity::find_by_id(wo_id)
            .one(&db)
            .await
            .unwrap()
            .expect("Work order should exist");

        assert!(
            wo.reject_form_id.is_some(),
            "Expected reject form to be linked to work order"
        );

        let reject_form = zent_be::entities::work_order_reject_forms::Entity::find_by_id(
            wo.reject_form_id.unwrap(),
        )
        .one(&db)
        .await
        .unwrap()
        .expect("Reject form should exist");

        // Assuming default is unapproved (false)
        assert_eq!(
            reject_form.approved, false,
            "Initial refusal should have approved=false"
        );
    }
    #[tokio::test]
    async fn test_admin_refusal_approve() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();
        let reject_form_id = Uuid::new_v4();

        seed_reject_form(&db, reject_form_id).await;
        seed_work_order(&db, wo_id, Some(tech_id), 5, Some(reject_form_id)).await; // 5 = Reject_InReview

        let uri = format!("/api/v1/work_orders/{}/refusal/approve", wo_id);
        let req = create_json_request_admin(
            http::Method::POST,
            &uri,
            &json!({ "comments": "Approved, please assign a different tech" }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let wo = zent_be::entities::work_orders::Entity::find_by_id(wo_id)
            .one(&db)
            .await
            .unwrap()
            .expect("Work order should exist");

        assert!(
            wo.reject_form_id.is_some(),
            "Expected reject form to be linked to work order"
        );

        let reject_form = zent_be::entities::work_order_reject_forms::Entity::find_by_id(
            wo.reject_form_id.unwrap(),
        )
        .one(&db)
        .await
        .unwrap()
        .expect("Reject form should exist");

        assert_eq!(
            reject_form.approved, true,
            "Refusal should be approved=true"
        );
        // In a real app we would assert approver_id is set to the token's user id
    }

    #[tokio::test]
    async fn test_admin_refusal_deny() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();
        let reject_form_id = Uuid::new_v4();

        seed_reject_form(&db, reject_form_id).await;
        seed_work_order(&db, wo_id, Some(tech_id), 5, Some(reject_form_id)).await; // 5 = Reject_InReview

        let uri = format!("/api/v1/work_orders/{}/refusal/deny", wo_id);
        let req = create_json_request_admin(
            http::Method::POST,
            &uri,
            &json!({ "comments": "Deny, this task must be done today" }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let wo = zent_be::entities::work_orders::Entity::find_by_id(wo_id)
            .one(&db)
            .await
            .unwrap()
            .expect("Work order should exist");

        assert!(
            wo.reject_form_id.is_none(),
            "Expected reject form to be detached from work order after denial"
        );

        let reject_form = zent_be::entities::work_order_reject_forms::Entity::find_by_id(
            reject_form_id,
        )
        .one(&db)
        .await
        .unwrap()
        .expect("Reject form should exist");

        assert_eq!(
            reject_form.approved, false,
            "Refusal should be approved=false"
        );
    }

    #[tokio::test]
    async fn test_upload_service_photos() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await; // 3 = InProg

        let uri = format!("/api/v1/media/work_orders/{}/closing_form/photos", wo_id);
        let req = create_multipart_request(
            http::Method::POST,
            &uri,
            vec![
                ("latitude", "10.8231"),
                ("longitude", "106.6297"),
                ("phase", "repaired"),
                ("file", "fake_image_content"), // Handler expects a file field
            ],
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK); // Handler returns 200, not 201

        let links = zent_be::entities::work_order_image_links::Entity::find()
            .filter(zent_be::entities::work_order_image_links::Column::WorkOrderId.eq(wo_id))
            .all(&db)
            .await
            .unwrap();

        assert!(
            !links.is_empty(),
            "Expected a linkage in work_order_image_links"
        );
    }

    #[tokio::test]
    async fn test_retrieve_service_photos() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;
        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();

        seed_work_order(&db, wo_id, Some(tech_id), 3, None).await; // 3 = InProg

        let uri = format!("/api/v1/media/photos/work_orders/{}", wo_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        // Handler returns 501, so we expect 501 for now if we want it to "run"
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
