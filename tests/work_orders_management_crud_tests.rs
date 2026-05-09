use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router, middleware,
};
use chrono::{DateTime, Utc};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    TEST_ADMIN_ID, TEST_TECHNICIAN_ID, TEST_CUSTOMER_ID,
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
    let cust_mw = middleware::from_fn_with_state(
        state.clone(),
        zent_be::extractor::role_check::require_role::<zent_be::core::state::AppState>(
            zent_be::entities::roles::Role::Customer,
        ),
    );

    // Customer routes
    let customer_routes = Router::new()
        .route(
            "/api/v1/work_orders",
            post(zent_be::handlers::v1::work_orders::create),
        )
        .layer(cust_mw);

    // Technician routes
    let tech_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/start",
            post(zent_be::handlers::v1::work_orders::start),
        )
        .route(
            "/api/v1/work_orders/{id}/refuse",
            post(zent_be::handlers::v1::work_orders::refuse),
        )
        .route(
            "/api/v1/work_orders/{id}/complete",
            post(zent_be::handlers::v1::work_orders::complete),
        )
        .layer(tech_mw);

    // Admin routes
    let admin_routes = Router::new()
        .route(
            "/api/v1/work_orders/{id}/assign",
            post(zent_be::handlers::v1::work_orders::assign),
        )
        .route(
            "/api/v1/work_orders/{id}/cancel",
            post(zent_be::handlers::v1::work_orders::cancel),
        )
        .layer(admin_mw);

    // Shared routes (no middleware — auth checked inside handler)
    Router::new()
        .route(
            "/api/v1/work_orders/{id}",
            get(zent_be::handlers::v1::work_orders::get_details),
        )
        .route(
            "/api/v1/work_orders",
            get(zent_be::handlers::v1::work_orders::list),
        )
        .merge(customer_routes)
        .merge(tech_routes)
        .merge(admin_routes)
        .with_state(state)
}

// ---------------------------------------------------------
// Request Builders
// ---------------------------------------------------------

fn create_json_request(method: http::Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
    create_json_request_with_token(method, uri, body, TEST_TECHNICIAN_ID)
}

fn create_json_request_with_token(method: http::Method, uri: &str, body: &serde_json::Value, user_id: &str) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(user_id).unwrap());
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

fn create_empty_request(method: http::Method, uri: &str) -> Request<Body> {
    create_empty_request_with_token(method, uri, TEST_TECHNICIAN_ID)
}

fn create_empty_request_with_token(method: http::Method, uri: &str, user_id: &str) -> Request<Body> {
    let token = create_test_jwt(Uuid::parse_str(user_id).unwrap());
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
// Domain Models
// =====================================================================

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateWorkOrderPayload {
    pub product_id: Uuid,
    pub work_order_symptom_id: i32,
    pub reference_ticket_id: Option<Uuid>,
    pub description: String,
    pub appointment: DateTime<Utc>,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub country: String,
    pub state: String,
    pub city: String,
    pub address: String,
    pub building: Option<String>,
}

impl Default for CreateWorkOrderPayload {
    fn default() -> Self {
        Self {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Screen flickering".to_string(),
            appointment: Utc::now(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: Some("john.doe@example.com".to_string()),
            phone_number: Some("+84123456789".to_string()),
            country: "VN".to_string(),
            state: "HCM".to_string(),
            city: "HCM".to_string(),
            address: "123 Hoa Binh".to_string(),
            building: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CancelReason {
    IncorrectInformation,
    DuplicateRequest,
    PartsUnavailable,
    CustomerRequest,
    Other,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CancelWorkOrderPayload {
    pub reason: CancelReason,
    pub additional_comments: Option<String>,
}

// =====================================================================
// 2.1. Customer Flow
// =====================================================================
#[cfg(test)]
mod customer_flow {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(json!({ "product_id": Uuid::new_v4() }), StatusCode::BAD_REQUEST)]
    #[case(json!({ "city": "HCM", "country": "VN" }), StatusCode::BAD_REQUEST)]
    #[tokio::test]
    async fn test_tc1_payload_validation(
        #[case] payload: serde_json::Value,
        #[case] expected: StatusCode,
    ) {
        let app = setup_test_app(mock_db().await).await;
        let req = create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &payload, TEST_CUSTOMER_ID);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), expected, "Must strictly enforce payload shapes");
    }

    #[rstest]
    #[case("HCM", StatusCode::CREATED)]
    #[case("London", StatusCode::BAD_REQUEST)]
    #[case("Ben Tre", StatusCode::BAD_REQUEST)]
    #[tokio::test]
    async fn test_tc1_location_policy(#[case] city: &str, #[case] expected: StatusCode) {
        let app = setup_test_app(mock_db().await).await;

        let mut payload = CreateWorkOrderPayload::default();
        payload.city = city.to_string();

        let req = create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload), TEST_CUSTOMER_ID);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            expected,
            "Location validation failure modes expected"
        );

        if expected == StatusCode::CREATED {
            let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
                .await
                .unwrap();
            let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
            assert_eq!(
                response_json["status"], "Pending assignment",
                "Guardrail: newly created Work Orders MUST start with 'Pending assignment' status"
            );
        }
    }

    #[tokio::test]
    async fn test_tc1_1_transactional_rollback() {
        let app = setup_test_app(mock_db().await).await;

        let payload = CreateWorkOrderPayload::default();
        let req = create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload), TEST_CUSTOMER_ID);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Expects complete transaction rollback on failure"
        );
    }

    #[tokio::test]
    async fn test_tc1_2_idempotent_creation() {
        let app = setup_test_app(mock_db().await).await;

        let payload = CreateWorkOrderPayload::default();
        let idempotency_key = Uuid::new_v4().to_string();

        let mut req1 =
            create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload), TEST_CUSTOMER_ID);
        req1.headers_mut()
            .insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

        let mut req2 =
            create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload), TEST_CUSTOMER_ID);
        req2.headers_mut()
            .insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

        let app_clone = app.clone();
        let r1 = app_clone.oneshot(req1).await.unwrap();
        let r2 = app.oneshot(req2).await.unwrap();

        assert_eq!(
            r1.status(),
            StatusCode::CREATED,
            "First request should succeed"
        );
        assert_eq!(
            r2.status(),
            StatusCode::CREATED,
            "Idempotency key must prevent duplicate errors and return the same successful status"
        );

        let b1 = axum::body::to_bytes(r1.into_body(), usize::MAX)
            .await
            .unwrap();
        let b2 = axum::body::to_bytes(r2.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            b1, b2,
            "Idempotent requests must return the exact same response body"
        );
    }

    #[tokio::test]
    async fn test_tc1_3_idempotency_key_conflict() {
        let app = setup_test_app(mock_db().await).await;

        let payload1 = CreateWorkOrderPayload::default();
        let mut payload2 = CreateWorkOrderPayload::default();
        payload2.city = "Different City".to_string();

        let idempotency_key = Uuid::new_v4().to_string();

        let mut req1 =
            create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload1), TEST_CUSTOMER_ID);
        req1.headers_mut()
            .insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

        let mut req2 =
            create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload2), TEST_CUSTOMER_ID);
        req2.headers_mut()
            .insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

        let app_clone = app.clone();
        let r1 = app_clone.oneshot(req1).await.unwrap();
        let r2 = app.oneshot(req2).await.unwrap();

        assert_eq!(
            r1.status(),
            StatusCode::CREATED,
            "First request should succeed"
        );
        assert_eq!(
            r2.status(),
            StatusCode::CONFLICT,
            "Reused idempotency key with different payload must fail"
        );
    }

    #[tokio::test]
    async fn test_tc1_4_create_with_reference_ticket() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;

        let mut payload = CreateWorkOrderPayload::default();
        let ref_id = Uuid::new_v4();
        payload.reference_ticket_id = Some(ref_id);

        let req = create_json_request_with_token(http::Method::POST, "/api/v1/work_orders", &json!(payload), TEST_CUSTOMER_ID);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            StatusCode::CREATED,
            "Must allow creation with reference ticket"
        );

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        let wo_id_str = response_json["id"].as_str().expect("id must be string");
        let wo_id = Uuid::parse_str(wo_id_str).unwrap();

        let wo = zent_be::entities::work_orders::Entity::find_by_id(wo_id)
            .one(&db)
            .await
            .unwrap();

        assert!(wo.is_some(), "Work order must be created in db");
        assert_eq!(
            wo.unwrap().reference_ticket_id,
            Some(ref_id),
            "Reference ticket ID must be properly linked in the database"
        );
    }
}

// =====================================================================
// 2.2. Administration Flow
// =====================================================================
#[cfg(test)]
mod admin_flow {
    use super::*;

    #[tokio::test]
    async fn test_tc2_assign_technician() {
        let db = mock_db().await;
        let app = setup_test_app(db.clone()).await;

        let wo_id = Uuid::new_v4();
        let tech_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}/assign", wo_id);
        let req = create_json_request_with_token(
            http::Method::POST,
            &uri,
            &json!({ "technician_id": tech_id }),
            TEST_ADMIN_ID,
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Assigned transition");

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            response_json["status"], "Assigned",
            "Guardrail: Assigned technician must transition WO to 'Assigned' status"
        );

        let wo = zent_be::entities::work_orders::Entity::find_by_id(wo_id)
            .one(&db)
            .await
            .unwrap();

        assert!(wo.is_some(), "Work order must exist");
        let wo = wo.unwrap();
        assert_eq!(
            wo.technician_id,
            Some(tech_id),
            "Assignee ID must be properly set"
        );
        assert!(
            wo.admin_id.is_some(),
            "Assigner ID must be set to the user performing the assignment"
        );
    }

    #[tokio::test]
    async fn test_tc3_invalid_state_transition() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/assign", Uuid::new_v4());
        let req = create_json_request_with_token(
            http::Method::POST,
            &uri,
            &json!({ "technician_id": Uuid::new_v4() }),
            TEST_ADMIN_ID,
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::CONFLICT,
            "Cannot assign completed WO"
        );
    }

    // schedule is no longer an HTTP endpoint — this test is skipped
    // #[rstest]
    // #[case(true, StatusCode::CONFLICT)]
    // #[case(false, StatusCode::OK)]
    // #[tokio::test]
    // async fn test_tc4_schedule_and_reschedule(
    //     #[case] _has_conflict: bool,
    //     #[case] expected: StatusCode,
    // ) {
    //     let app = setup_test_app(mock_db().await).await;
    //
    //     let uri = format!("/api/v1/work_orders/{}/schedule", Uuid::new_v4());
    //     let payload = json!({
    //         "technician_id": Uuid::new_v4(),
    //         "appointment_time": "2026-10-30T10:00:00Z"
    //     });
    //
    //     let req = create_json_request(http::Method::POST, &uri, &payload);
    //     let r = app.oneshot(req).await.unwrap();
    //
    //     assert_eq!(r.status(), expected, "Conflict checking required");
    // }
}

// =====================================================================
// 2.3. Execution Flow
// =====================================================================
#[cfg(test)]
mod execution_flow {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(10.762622, 106.660172, StatusCode::OK)]
    #[case(40.712776, -74.005974, StatusCode::FORBIDDEN)]
    #[tokio::test]
    async fn test_tc_geo_fencing_constraint(
        #[case] lat: f64,
        #[case] lng: f64,
        #[case] expected: StatusCode,
    ) {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/start", Uuid::new_v4());

        let payload = json!({
            "latitude": lat,
            "longitude": lng,
            "timestamp": "2026-10-30T10:00:00Z"
        });

        let req = create_json_request(http::Method::POST, &uri, &payload);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), expected, "Must enforce geofencing boundary");
    }

    #[tokio::test]
    async fn test_tc5_start_work_order() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/start", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "latitude": 10.0, "longitude": 106.0 }),
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Assigned -> In Progress");

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            response_json["status"], "In Progress",
            "Guardrail: Starting work must transition WO to 'In Progress' status"
        );
    }

    #[tokio::test]
    async fn test_tc6_refuse_work_order() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/refuse", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "reason": "Customer absent" }),
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Assigned -> Rejected_InReview");

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            response_json["status"], "Rejected",
            "Guardrail: Refusing work must transition WO to 'Rejected' status"
        );
    }
}

// =====================================================================
// 2.4. Validation Layer (Shared)
// =====================================================================
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_tc7_cancel_work_order() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/cancel", Uuid::new_v4());
        let payload = json!({
            "reason": "CustomerRequest",
            "additional_comments": "No longer needed"
        });

        let req = create_json_request_with_token(http::Method::POST, &uri, &payload, TEST_ADMIN_ID);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_tc8_malformed_json() {
        let app = setup_test_app(mock_db().await).await;
        let uri = "/api/v1/work_orders";
        let req = create_json_request_with_token(http::Method::POST, uri, &json!("not_an_object"), TEST_CUSTOMER_ID);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_tc9_method_not_allowed() {
        let app = setup_test_app(mock_db().await).await;
        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let req = create_empty_request(http::Method::DELETE, &uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}

// =====================================================================
// 2.5. Listing & Retrieval
// =====================================================================
#[cfg(test)]
mod listing_tests {
    use super::*;

    #[tokio::test]
    async fn test_tc10_list_work_orders() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/work_orders";
        let req = create_empty_request(http::Method::GET, uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_tc11_get_work_order_details() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let req = create_empty_request(http::Method::GET, &uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }
}

// =====================================================================
// 2.6. Security Tests
// =====================================================================
#[cfg(test)]
mod security_tests {
    use super::*;

    #[tokio::test]
    async fn test_tc_missing_auth_header() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let req = Request::builder()
            .method(http::Method::GET)
            .uri(&uri)
            .body(Body::empty())
            .unwrap();

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::UNAUTHORIZED,
            "Must require authentication"
        );
    }

    #[tokio::test]
    async fn test_tc_cross_tenant_access() {
        let app = setup_test_app(mock_db().await).await;

        // Seed a second customer with a different UUID for the "malicious" user
        let malicious_id = Uuid::new_v4();

        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let req = create_empty_request_with_token(http::Method::GET, &uri, &malicious_id.to_string());
        // Override the token with one for the malicious user
        // (the helper already sets a valid token for that user id)

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::FORBIDDEN,
            "Must enforce tenant isolation"
        );
    }

    #[tokio::test]
    async fn test_tc_technician_scope_breach() {
        let app = setup_test_app(mock_db().await).await;

        let uri = format!("/api/v1/work_orders/{}/start", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "latitude": 10.0, "longitude": 106.0, "timestamp": "2026-10-30T10:05:00Z" }),
        );
        // The default token uses TEST_TECHNICIAN_ID which is a valid technician
        // but not assigned to any work order, so they should get FORBIDDEN

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::FORBIDDEN,
            "Must enforce technician assignment scope"
        );
    }
}
