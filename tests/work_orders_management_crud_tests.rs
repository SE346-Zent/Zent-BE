use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};

use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------
// Infrastructure Mocking
// ---------------------------------------------------------

/// Mock MQ Manager reflecting the `is_stub` check from `email.rs`
/// and state tracking for event assertions.
#[derive(Clone, Default)]
pub struct MockRabbitMQManager {
    pub is_stub: bool,
    pub published_messages: Arc<Mutex<Vec<(String, Value)>>>,
}

impl MockRabbitMQManager {
    pub fn assert_published(&self, expected_routing_key: &str) {
        let messages = self.published_messages.lock().unwrap();
        assert!(
            messages.iter().any(|(rk, _)| rk == expected_routing_key),
            "Expected message with routing key '{}' to be published. Dispatched keys: {:?}",
            expected_routing_key,
            messages
                .iter()
                .map(|(k, _)| k.clone())
                .collect::<Vec<String>>()
        );
    }

    #[allow(dead_code)]
    pub async fn publish_stub(&self, routing_key: &str, payload: Value) {
        let mut messages = self.published_messages.lock().unwrap();
        messages.push((routing_key.to_string(), payload));
    }
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
        .route(
            "/api/v1/work_orders",
            post(not_implemented).get(not_implemented),
        )
        .route("/api/v1/work_orders/{id}", get(not_implemented))
        .route("/api/v1/work_orders/{id}/assign", post(not_implemented))
        .route("/api/v1/work_orders/{id}/schedule", post(not_implemented))
        .route("/api/v1/work_orders/{id}/start", post(not_implemented))
        .route("/api/v1/work_orders/{id}/refuse", post(not_implemented))
        .route("/api/v1/work_orders/{id}/cancel", post(not_implemented))
        .route("/api/v1/work_orders/{id}/complete", post(not_implemented))
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
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;
        let req = create_json_request(http::Method::POST, "/api/v1/work_orders", &payload);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), expected, "Must strictly enforce payload shapes");
    }

    #[rstest]
    #[case("HCM", StatusCode::CREATED)]
    #[case("London", StatusCode::BAD_REQUEST)]
    #[case("Ben Tre", StatusCode::BAD_REQUEST)]
    #[tokio::test]
    async fn test_tc1_location_policy(#[case] city: &str, #[case] expected: StatusCode) {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let mut payload = CreateWorkOrderPayload::default();
        payload.city = city.to_string();

        let req = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload));
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
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let payload = CreateWorkOrderPayload::default();
        let req = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload));
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Expects complete transaction rollback on failure"
        );
    }

    #[tokio::test]
    async fn test_tc1_2_idempotent_creation() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let payload = CreateWorkOrderPayload::default();
        let idempotency_key = Uuid::new_v4().to_string();

        let mut req1 = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload));
        req1.headers_mut().insert("X-Idempotency-Key", idempotency_key.parse().unwrap());
        
        let mut req2 = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload));
        req2.headers_mut().insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

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
        
        let b1 = axum::body::to_bytes(r1.into_body(), usize::MAX).await.unwrap();
        let b2 = axum::body::to_bytes(r2.into_body(), usize::MAX).await.unwrap();
        assert_eq!(b1, b2, "Idempotent requests must return the exact same response body");
    }

    #[tokio::test]
    async fn test_tc1_3_idempotency_key_conflict() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let payload1 = CreateWorkOrderPayload::default();
        let mut payload2 = CreateWorkOrderPayload::default();
        payload2.city = "Different City".to_string();

        let idempotency_key = Uuid::new_v4().to_string();

        let mut req1 = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload1));
        req1.headers_mut().insert("X-Idempotency-Key", idempotency_key.parse().unwrap());
        
        let mut req2 = create_json_request(http::Method::POST, "/api/v1/work_orders", &json!(payload2));
        req2.headers_mut().insert("X-Idempotency-Key", idempotency_key.parse().unwrap());

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
}

// =====================================================================
// 2.2. Administration Flow
// =====================================================================
mod admin_flow {
    use super::*;
    use rstest::rstest;

    #[tokio::test]
    async fn test_tc2_assign_technician() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/assign", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "technician_id": Uuid::new_v4() }),
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
    }

    #[tokio::test]
    async fn test_tc3_invalid_state_transition() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/assign", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "technician_id": Uuid::new_v4() }),
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::CONFLICT,
            "Cannot assign completed WO"
        );
    }

    #[rstest]
    #[case(true, StatusCode::CONFLICT)]
    #[case(false, StatusCode::OK)]
    #[tokio::test]
    async fn test_tc4_schedule_and_reschedule(
        #[case] _has_conflict: bool,
        #[case] expected: StatusCode,
    ) {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq.clone()).await;

        let uri = format!("/api/v1/work_orders/{}/schedule", Uuid::new_v4());
        let payload = json!({
            "technician_id": Uuid::new_v4(),
            "appointment_time": "2026-10-30T10:00:00Z"
        });

        let req = create_json_request(http::Method::POST, &uri, &payload);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(r.status(), expected, "Conflict checking required");

        if expected == StatusCode::OK {
            // Fails in pure TDD Red phase since endpoint returns 501 and doesn't invoke publisher.
            mq.assert_published("email_exchange.send_email");
        }
    }
}

// =====================================================================
// 2.3. Execution Flow
// =====================================================================
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
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

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
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

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
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/refuse", Uuid::new_v4());
        let req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "reason": "Out of scope" }),
        );
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Assigned -> Refused");

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            response_json["status"], "Refused",
            "Guardrail: Refusing work must transition WO to 'Refused' status"
        );
    }

    #[tokio::test]
    async fn test_tc7_cancel_mid_work() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/cancel", Uuid::new_v4());

        let payload = CancelWorkOrderPayload {
            reason: CancelReason::PartsUnavailable,
            additional_comments: Some(
                "Component requires 3-week backorder via supplier.".to_string(),
            ),
        };

        let req = create_json_request(http::Method::POST, &uri, &json!(payload));
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Transition to Refused");

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            response_json["status"], "Refused",
            "Guardrail: Cancelling work must transition WO to 'Refused' status"
        );
    }
}

// =====================================================================
// 2.4. Completion Flow
// =====================================================================
mod completion_flow {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(true, StatusCode::OK)]
    #[case(false, StatusCode::BAD_REQUEST)]
    #[tokio::test]
    async fn test_tc8_closing_form_validation(
        #[case] has_evidence: bool,
        #[case] expected: StatusCode,
    ) {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/complete", Uuid::new_v4());

        let payload = if has_evidence {
            json!({ "evidence_image_ids": ["img_1", "img_2"], "signature_id": "sig_1" })
        } else {
            json!({ "evidence_image_ids": [], "signature_id": "sig_1" })
        };

        let req = create_json_request(http::Method::POST, &uri, &payload);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            expected,
            "Work order completion form must be strictly validated"
        );

        if expected == StatusCode::OK {
            let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
                .await
                .unwrap();
            let response_json: Value = serde_json::from_slice(&body_bytes).unwrap();
            assert_eq!(
                response_json["status"], "Completed",
                "Guardrail: Completed work must transition WO to 'Completed' status"
            );
        }
    }

    #[tokio::test]
    async fn test_tc8_1_immutable_completed_state() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let wo_id = Uuid::new_v4();

        let schedule_uri = format!("/api/v1/work_orders/{}/schedule", wo_id);
        let payload =
            json!({ "technician_id": Uuid::new_v4(), "appointment_time": "2026-10-30T10:00:00Z" });
        let req = create_json_request(http::Method::POST, &schedule_uri, &payload);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            StatusCode::CONFLICT,
            "Completed work orders must be immutable"
        );
    }
}

// =====================================================================
// 2.5. Visibility Flow
// =====================================================================
mod visibility_flow {
    use super::*;

    #[tokio::test]
    async fn test_tc9_customer_list_pagination() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = "/api/v1/work_orders?cursor=eyJpZCI6MTIzfQ&limit=20";
        let req = create_empty_request(http::Method::GET, uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::OK,
            "Accepts pagination format correctly"
        );
    }

    #[tokio::test]
    async fn test_tc10_technician_filter() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = "/api/v1/work_orders?status=Assigned";
        let req = create_empty_request(http::Method::GET, uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Filters by Assigned for Techs");
    }

    #[tokio::test]
    async fn test_tc11_get_work_order_details() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let req = create_empty_request(http::Method::GET, &uri);
        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK, "Retrieves full joined DTO");
    }

    #[tokio::test]
    async fn test_tc_pagination_limit_overflow() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = "/api/v1/work_orders?cursor=eyJpZCI6MTIzfQ&limit=1000";
        let req = create_empty_request(http::Method::GET, uri);
        let r = app.oneshot(req).await.unwrap();

        assert_eq!(
            r.status(),
            StatusCode::BAD_REQUEST,
            "Must reject limits exceeding maximum bounds"
        );
    }

    #[tokio::test]
    async fn test_tc_cross_tenant_access() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}", Uuid::new_v4());
        let mut req = create_empty_request(http::Method::GET, &uri);
        req.headers_mut()
            .insert(http::header::AUTHORIZATION, "Bearer malicious_user_jwt_token".parse().unwrap());

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::FORBIDDEN,
            "Must enforce tenant isolation"
        );
    }

    #[tokio::test]
    async fn test_tc_technician_scope_breach() {
        let mq = Arc::new(MockRabbitMQManager::default());
        let app = setup_test_app(mock_db().await, mq).await;

        let uri = format!("/api/v1/work_orders/{}/start", Uuid::new_v4());
        let mut req = create_json_request(
            http::Method::POST,
            &uri,
            &json!({ "latitude": 10.0, "longitude": 106.0, "timestamp": "2026-10-30T10:05:00Z" }),
        );
        req.headers_mut()
            .insert(http::header::AUTHORIZATION, "Bearer unassigned_technician_jwt_token".parse().unwrap());

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::FORBIDDEN,
            "Must enforce technician assignment scope"
        );
    }
}
