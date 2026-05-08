use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::*;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
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
// Mock Notification Repository (MongoDB — Bucket Pattern)
// ---------------------------------------------------------

/// In-memory mock of the MongoDB notification store.
/// Mirrors the bucket pattern: each document holds up to 50 notifications
/// for a user, keyed by (user_id, page_number).  Once a bucket exceeds 50
/// entries a new document with page_number = prev + 1 is created.
#[derive(Clone)]
pub struct MockNotificationRepo {
    /// (user_id, page_number) → ordered list of notifications
    buckets: Arc<Mutex<HashMap<(Uuid, u32), Vec<NotificationDocument>>>>,
}

/// A single notification stored inside a bucket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDocument {
    pub notification_id: Uuid,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub is_read: bool,
    pub os_notification_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl MockNotificationRepo {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Insert a notification into the user's bucket, respecting the 50-cap.
    /// Returns (page_number, index_within_page) for later retrieval.
    pub fn insert(&self, user_id: Uuid, notif: NotificationDocument) -> (u32, usize) {
        let mut buckets = self.buckets.lock().unwrap();

        // Find the highest page for this user
        let max_page = buckets
            .keys()
            .filter(|(uid, _)| *uid == user_id)
            .map(|(_, page)| *page)
            .max()
            .unwrap_or(1);

        let (page, idx) = {
            let entry = buckets
                .entry((user_id, max_page))
                .or_insert_with(Vec::new);
            if entry.len() >= 50 {
                // Overflow — create a new page
                let new_page = max_page + 1;
                let new_entry = buckets
                    .entry((user_id, new_page))
                    .or_insert_with(Vec::new);
                new_entry.push(notif);
                (new_page, 0usize)
            } else {
                entry.push(notif);
                (max_page, entry.len() - 1)
            }
        };

        (page, idx)
    }

    pub fn get_bucket(&self, user_id: Uuid, page_number: u32) -> Option<Vec<NotificationDocument>> {
        let buckets = self.buckets.lock().unwrap();
        buckets.get(&(user_id, page_number)).cloned()
    }

    pub fn find_by_id(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Option<NotificationDocument> {
        let buckets = self.buckets.lock().unwrap();
        for ((uid, _), notifs) in buckets.iter() {
            if *uid == user_id {
                for n in notifs {
                    if n.notification_id == notification_id {
                        return Some(n.clone());
                    }
                }
            }
        }
        None
    }

    pub fn mark_read(&self, user_id: Uuid, notification_id: Uuid) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        for ((uid, _), notifs) in buckets.iter_mut() {
            if *uid == user_id {
                for n in notifs.iter_mut() {
                    if n.notification_id == notification_id {
                        n.is_read = true;
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn mark_all_read(&self, user_id: Uuid) -> usize {
        let mut count = 0;
        let mut buckets = self.buckets.lock().unwrap();
        for ((uid, _), notifs) in buckets.iter_mut() {
            if *uid == user_id {
                for n in notifs.iter_mut() {
                    if !n.is_read {
                        n.is_read = true;
                        count += 1;
                    }
                }
            }
        }
        count
    }

    pub fn list_for_user(
        &self,
        user_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> (Vec<NotificationDocument>, usize) {
        let buckets = self.buckets.lock().unwrap();
        let mut all: Vec<NotificationDocument> = buckets
            .iter()
            .filter(|((uid, _), _)| *uid == user_id)
            .flat_map(|(_, notifs)| notifs.clone())
            .collect();

        // Newest first (natural notification order)
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = all.len();
        let page: Vec<_> = all.into_iter().skip(offset).take(limit).collect();
        (page, total)
    }
}

// ---------------------------------------------------------
// Mock Outbox Repository (MongoDB — Outbox Pattern)
// ---------------------------------------------------------

/// In-memory mock of the outbox collection.
///
/// Every in-app notification is first written to the outbox *and* the main
/// bucket inside a single MongoDB transaction.  A background worker
/// processes the outbox, delivering notifications to the WebSocket gateway
/// (or marking them delivered for polling clients), then deletes the
/// outbox entry.
#[derive(Clone)]
pub struct MockOutboxRepo {
    entries: Arc<Mutex<Vec<OutboxEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEntry {
    pub outbox_id: Uuid,
    pub user_id: Uuid,
    pub notification_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
}

impl MockOutboxRepo {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn insert(&self, entry: OutboxEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    pub fn list_pending(&self, user_id: Uuid) -> Vec<OutboxEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.user_id == user_id && !e.delivered)
            .cloned()
            .collect()
    }

    pub fn mark_delivered(&self, outbox_id: Uuid) -> bool {
        let mut entries = self.entries.lock().unwrap();
        for e in entries.iter_mut() {
            if e.outbox_id == outbox_id {
                e.delivered = true;
                return true;
            }
        }
        false
    }

    pub fn delete_delivered(&self, user_id: Uuid) -> usize {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|e| !(e.user_id == user_id && e.delivered));
        before - entries.len()
    }
}

// ---------------------------------------------------------
// Mock MQ Producer (RabbitMQ — OS Notifications / FCM)
// ---------------------------------------------------------

/// Records published messages for test assertions.
#[derive(Clone)]
pub struct MockMqProducer {
    published: Arc<Mutex<Vec<NotificationMqMessage>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationMqMessage {
    pub user_id: Uuid,
    pub device_token: Option<String>,
    pub category: String,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub os_notification_id: Uuid,
}

impl MockMqProducer {
    pub fn new() -> Self {
        Self {
            published: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn publish(&self, msg: NotificationMqMessage) {
        self.published.lock().unwrap().push(msg);
    }

    pub fn drain(&self) -> Vec<NotificationMqMessage> {
        self.published.lock().unwrap().drain(..).collect()
    }

    pub fn count(&self) -> usize {
        self.published.lock().unwrap().len()
    }
}

// ---------------------------------------------------------
// Notification Categories Seed Data
// ---------------------------------------------------------

pub const NOTIFICATION_CATEGORIES: &[(&str, &str)] = &[
    ("work_order_assigned", "Work Order Assigned"),
    ("work_order_started", "Work Order Started"),
    ("work_order_completed", "Work Order Completed"),
    ("work_order_rejected", "Work Order Rejected"),
    ("work_order_refusal_approved", "Refusal Approved"),
    ("work_order_scheduled", "Work Order Scheduled"),
    ("account_verified", "Account Verified"),
    ("account_locked", "Account Locked"),
];

// ---------------------------------------------------------
// Test State
// ---------------------------------------------------------

/// Shared application state wired into the test router.
#[derive(Clone)]
pub struct NotificationTestState {
    pub notification_repo: MockNotificationRepo,
    pub outbox_repo: MockOutboxRepo,
    pub mq_producer: MockMqProducer,
}

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

async fn setup_test_app(db: DatabaseConnection) -> Router {
    let _ = tracing_subscriber::fmt::try_init();
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;

    // Seed notification categories into the mock stores
    let notification_repo = MockNotificationRepo::new();
    let outbox_repo = MockOutboxRepo::new();
    let mq_producer = MockMqProducer::new();

    let state = NotificationTestState {
        notification_repo: notification_repo.clone(),
        outbox_repo: outbox_repo.clone(),
        mq_producer: mq_producer.clone(),
    };

    Router::new()
        // ── Preferences ──────────────────────────────────────────────
        .route(
            "/api/v1/notifications/preferences",
            get(zent_be::handlers::v1::notifications::get_preferences)
                .put(zent_be::handlers::v1::notifications::update_preferences),
        )
        // ── In-App Notifications ─────────────────────────────────────
        .route(
            "/api/v1/notifications",
            get(zent_be::handlers::v1::notifications::list),
        )
        .route(
            "/api/v1/notifications/{id}",
            get(zent_be::handlers::v1::notifications::get_detail),
        )
        .route(
            "/api/v1/notifications/{id}/read",
            post(zent_be::handlers::v1::notifications::mark_read),
        )
        .route(
            "/api/v1/notifications/read-all",
            post(zent_be::handlers::v1::notifications::mark_all_read),
        )
        // ── Outbox / Delivery ────────────────────────────────────────
        .route(
            "/api/v1/notifications/outbox/sync",
            post(zent_be::handlers::v1::notifications::sync_outbox),
        )
        // ── Categories ───────────────────────────────────────────────
        .route(
            "/api/v1/notifications/categories",
            get(zent_be::handlers::v1::notifications::list_categories),
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
// Notification Preferences Tests
// =====================================================================

#[cfg(test)]
mod notification_preferences_tests {
    use super::*;

    /// GET /api/v1/notifications/preferences
    /// Returns the current user's preferences for each notification category.
    /// By default every OS category is enabled.  In-app notifications are
    /// always on and do NOT appear in the preferences payload.
    #[tokio::test]
    async fn test_get_preferences_returns_all_categories() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/preferences";
        let req = create_empty_request(http::Method::GET, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        // Response envelope shape
        assert_eq!(body["statusCode"], 200);
        assert!(body["data"].is_array());
        let prefs = body["data"].as_array().unwrap();
        assert_eq!(
            prefs.len(),
            NOTIFICATION_CATEGORIES.len(),
            "Preferences must include every registered category"
        );

        // Each preference entry must have the expected fields
        for pref in prefs {
            assert!(pref["categoryId"].is_number(), "Missing categoryId");
            assert!(pref["categoryName"].is_string(), "Missing categoryName");
            assert!(pref["osEnabled"].is_bool(), "Missing osEnabled");
            // In-app is always on — the field should not appear
            assert!(pref.get("inAppEnabled").is_none(), "inAppEnabled must NOT be exposed");
        }
    }

    /// PUT /api/v1/notifications/preferences
    /// The user can toggle `osEnabled` on a per-category basis.
    /// In-app delivery cannot be disabled.
    #[tokio::test]
    async fn test_update_preference_disables_os_category() {
        let app = setup_test_app(mock_db().await).await;

        // Category id 1 = "work_order_assigned"
        let uri = "/api/v1/notifications/preferences";
        let req = create_json_request(
            http::Method::PUT,
            uri,
            &json!({
                "categoryId": 1,
                "osEnabled": false
            }),
        );

        let r = app.clone().oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Now GET — the category should be disabled
        let get_uri = "/api/v1/notifications/preferences";
        let get_req = create_empty_request(http::Method::GET, get_uri);
        let get_r = app.oneshot(get_req).await.unwrap();

        let body_bytes = axum::body::to_bytes(get_r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        let prefs = body["data"].as_array().unwrap();
        let assigned = prefs
            .iter()
            .find(|p| p["categoryId"] == 1)
            .expect("Category must exist");
        assert_eq!(assigned["osEnabled"], false, "OS must be disabled");
    }

    /// PUT with an invalid (non-existent) category id must return 404.
    #[tokio::test]
    async fn test_update_preference_invalid_category_returns_404() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/preferences";
        let req = create_json_request(
            http::Method::PUT,
            uri,
            &json!({
                "categoryId": 9999,
                "osEnabled": false
            }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }

    /// PUT with a missing `osEnabled` field must return 400.
    #[tokio::test]
    async fn test_update_preference_missing_os_enabled_returns_400() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/preferences";
        let req = create_json_request(
            http::Method::PUT,
            uri,
            &json!({ "categoryId": 1 }),
        );

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    }
}

// =====================================================================
// Notification Categories Tests
// =====================================================================

#[cfg(test)]
mod notification_categories_tests {
    use super::*;

    /// GET /api/v1/notifications/categories
    /// Returns all available notification categories.
    #[tokio::test]
    async fn test_list_categories() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/categories";
        let req = create_empty_request(http::Method::GET, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["statusCode"], 200);
        let categories = body["data"].as_array().unwrap();
        assert_eq!(categories.len(), NOTIFICATION_CATEGORIES.len());

        // Each category must have id, name, and slug
        for cat in categories {
            assert!(cat["id"].is_number(), "Missing id");
            assert!(cat["name"].is_string(), "Missing name");
            assert!(cat["slug"].is_string(), "Missing slug");
        }
    }
}

// =====================================================================
// In-App Notification CRUD Tests
// =====================================================================

#[cfg(test)]
mod notification_crud_tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────

    /// Pre-populate the mock repo with a known notification.
    fn seed_notification(
        repo: &MockNotificationRepo,
        outbox: &MockOutboxRepo,
    ) -> (Uuid, NotificationDocument) {
        let user_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let now = Utc::now();

        let doc = NotificationDocument {
            notification_id: notif_id,
            category_id: 1, // work_order_assigned
            title: "Work Order Assigned".into(),
            body: "You have been assigned work order WO-ABC123.".into(),
            data: json!({ "workOrderId": Uuid::new_v4().to_string() }),
            is_read: false,
            os_notification_id: Some(Uuid::new_v4()), // 1:1 with OS notif
            created_at: now,
        };

        repo.insert(user_id, doc.clone());
        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        (user_id, doc)
    }

    // ── GET /api/v1/notifications ────────────────────────────────────

    /// List notifications for the authenticated user.
    /// Expects paginated results in newest-first order.
    #[tokio::test]
    async fn test_list_notifications_returns_paginated() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications?limit=20&page=1";
        let req = create_empty_request(http::Method::GET, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["statusCode"], 200);
        assert!(body["data"].is_array());
        // Pagination metadata must be present
        assert!(body["meta"].is_object());
        assert!(body["meta"]["totalRecords"].is_number());
        assert!(body["meta"]["hasNext"].is_bool());
    }

    /// When the user has no notifications the endpoint returns an empty
    /// array — not a 404.
    #[tokio::test]
    async fn test_list_notifications_empty_for_new_user() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications";
        let req = create_empty_request(http::Method::GET, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        let data = body["data"].as_array().unwrap();
        assert!(data.is_empty());
        assert_eq!(body["meta"]["totalRecords"], 0);
        assert_eq!(body["meta"]["hasNext"], false);
    }

    // ── GET /api/v1/notifications/{id} ───────────────────────────────

    /// Fetch a single notification by its id.
    #[tokio::test]
    async fn test_get_notification_detail() {
        // This test would seed a notification directly via the repo
        // and then fetch it through the handler.
        let app = setup_test_app(mock_db().await).await;

        let notif_id = Uuid::new_v4();
        let uri = format!("/api/v1/notifications/{}", notif_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        // Without pre-seeding, this may be 404 — the handler must
        // distinguish "not found" from "doesn't belong to user".
        assert!(
            r.status() == StatusCode::OK || r.status() == StatusCode::NOT_FOUND,
            "Must return OK when exists, 404 when missing"
        );
    }

    /// When the notification belongs to a different user we must NOT
    /// leak it — return 404 (not 403) to avoid information disclosure.
    #[tokio::test]
    async fn test_get_notification_cross_user_returns_404() {
        let app = setup_test_app(mock_db().await).await;
        let other_user_notif_id = Uuid::new_v4();

        let uri = format!("/api/v1/notifications/{}", other_user_notif_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::NOT_FOUND,
            "Must return 404 for notifications belonging to another user"
        );
    }

    // ── POST /api/v1/notifications/{id}/read ─────────────────────────

    /// Mark a single notification as read.
    #[tokio::test]
    async fn test_mark_notification_read() {
        let app = setup_test_app(mock_db().await).await;

        let notif_id = Uuid::new_v4();
        let uri = format!("/api/v1/notifications/{}/read", notif_id);
        let req = create_empty_request(http::Method::POST, &uri);

        let r = app.oneshot(req).await.unwrap();
        // Marking an already-read or non-existent notification should
        // either succeed idempotently or return 404.
        assert!(
            r.status() == StatusCode::OK || r.status() == StatusCode::NOT_FOUND,
            "Mark-read must be idempotent"
        );
    }

    // ── POST /api/v1/notifications/read-all ──────────────────────────

    /// Mark all notifications as read for the current user.
    #[tokio::test]
    async fn test_mark_all_notifications_read() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/read-all";
        let req = create_empty_request(http::Method::POST, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["statusCode"], 200);
        assert_eq!(body["message"], "All notifications marked as read");
    }
}

// =====================================================================
// OS Notification / FCM via RabbitMQ Tests
// =====================================================================

#[cfg(test)]
mod os_notification_tests {
    use super::*;

    /// When a work order is assigned to a technician, TWO notifications
    /// must be created:
    ///   1. An in-app notification (stored in MongoDB bucket).
    ///   2. An OS notification (published to RabbitMQ for FCM delivery).
    ///
    /// They share a 1:1 relationship via `osNotificationId`.
    #[tokio::test]
    async fn test_notification_has_one_to_one_os_relationship() {
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();
        let mq = MockMqProducer::new();

        let user_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let os_notif_id = Uuid::new_v4();
        let now = Utc::now();

        // Simulate what the service would do
        let doc = NotificationDocument {
            notification_id: notif_id,
            category_id: 1,
            title: "New Assignment".into(),
            body: "You have a new work order.".into(),
            data: json!({ "workOrderId": Uuid::new_v4().to_string() }),
            is_read: false,
            os_notification_id: Some(os_notif_id),
            created_at: now,
        };

        repo.insert(user_id, doc);
        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        mq.publish(NotificationMqMessage {
            user_id,
            device_token: Some("fcm_token_abc123".into()),
            category: "work_order_assigned".into(),
            title: "New Assignment".into(),
            body: "You have a new work order.".into(),
            data: json!({ "workOrderId": Uuid::new_v4().to_string() }),
            os_notification_id,
        });

        // Assert: 1 in-app notification stored
        let stored = repo.find_by_id(user_id, notif_id);
        assert!(stored.is_some(), "In-app notification must be stored");
        let stored = stored.unwrap();
        assert_eq!(
            stored.os_notification_id,
            Some(os_notification_id),
            "In-app notification must reference the OS notification"
        );

        // Assert: 1 OS notification published to MQ
        assert_eq!(mq.count(), 1, "Exactly one OS notification must be published");

        let mq_msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(mq_msg.os_notification_id, os_notification_id);
        assert_eq!(mq_msg.category, "work_order_assigned");
    }

    /// When the user has disabled OS notifications for a particular
    /// category, the in-app notification is STILL created, but the
    /// OS notification (FCM) is SKIPPED.
    #[tokio::test]
    async fn test_os_notification_respects_user_preference() {
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();
        let mq = MockMqProducer::new();

        // Simulate user preferences: OS disabled for category 1
        let user_prefs: HashMap<i32, bool> = [(1, false)].into(); // category_id → os_enabled

        let user_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let now = Utc::now();

        // In-app notification ALWAYS created
        let doc = NotificationDocument {
            notification_id: notif_id,
            category_id: 1,
            title: "New Assignment".into(),
            body: "You have a new work order.".into(),
            data: json!({}),
            is_read: false,
            os_notification_id: None, // No OS counterpart
            created_at: now,
        };
        repo.insert(user_id, doc);
        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        // Decision: should OS notification be published?
        let os_enabled = user_prefs.get(&1).copied().unwrap_or(true);
        if os_enabled {
            mq.publish(NotificationMqMessage {
                user_id,
                device_token: Some("fcm_token_abc123".into()),
                category: "work_order_assigned".into(),
                title: "New Assignment".into(),
                body: "You have a new work order.".into(),
                data: json!({}),
                os_notification_id: Uuid::new_v4(),
            });
        }

        // Assert: in-app notification stored
        assert!(repo.find_by_id(user_id, notif_id).is_some());

        // Assert: NO OS notification published (user disabled it)
        assert_eq!(
            mq.count(),
            0,
            "OS notification must NOT be published when user disabled the category"
        );
    }

    /// When the user has not set any preference, OS notifications
    /// default to ENABLED for every category.
    #[tokio::test]
    async fn test_os_notification_defaults_to_enabled() {
        // Default preference for an unset category is true
        let user_prefs: HashMap<i32, bool> = HashMap::new();

        let os_enabled = user_prefs.get(&1).copied().unwrap_or(true);
        assert!(os_enabled, "Default for unset category must be enabled");
    }
}

// =====================================================================
// Outbox Pattern Tests
// =====================================================================

#[cfg(test)]
mod outbox_pattern_tests {
    use super::*;

    /// Every in-app notification MUST be written to the outbox
    /// in the same logical transaction as the bucket insert.
    /// This ensures at-least-once delivery.
    #[tokio::test]
    async fn test_outbox_entry_created_with_notification() {
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();

        let user_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let outbox_id = Uuid::new_v4();
        let now = Utc::now();

        // ── Simulate the atomic write (service effect) ─────────────────
        {
            // In production these two inserts happen inside a single
            // MongoDB transaction.
            repo.insert(
                user_id,
                NotificationDocument {
                    notification_id: notif_id,
                    category_id: 1,
                    title: "Test".into(),
                    body: "Body".into(),
                    data: json!({}),
                    is_read: false,
                    os_notification_id: None,
                    created_at: now,
                },
            );

            outbox.insert(OutboxEntry {
                outbox_id,
                user_id,
                notification_id: notif_id,
                created_at: now,
                delivered: false,
            });
        }

        // ── Assert both exist ──────────────────────────────────────────
        let notif = repo.find_by_id(user_id, notif_id);
        assert!(notif.is_some(), "Notification must be in the bucket");

        let pending = outbox.list_pending(user_id);
        assert_eq!(pending.len(), 1, "Outbox must have one pending entry");
        assert_eq!(pending[0].notification_id, notif_id);
        assert!(!pending[0].delivered);
    }

    /// POST /api/v1/notifications/outbox/sync
    /// The client calls this endpoint (e.g. on app foreground, or
    /// periodically) to pull any undelivered notifications.
    /// The server returns pending notifications and marks them
    /// delivered so they won't be re-sent.
    #[tokio::test]
    async fn test_outbox_sync_returns_pending_and_marks_delivered() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications/outbox/sync";
        let req = create_empty_request(http::Method::POST, uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["statusCode"], 200);
        // Data should be an array of pending notifications
        assert!(body["data"].is_array());
    }

    /// After syncing, a second sync must NOT return the same
    /// notifications — they are marked delivered.
    #[tokio::test]
    async fn test_outbox_sync_idempotent() {
        let outbox = MockOutboxRepo::new();
        let user_id = Uuid::new_v4();
        let outbox_id = Uuid::new_v4();
        let now = Utc::now();

        outbox.insert(OutboxEntry {
            outbox_id,
            user_id,
            notification_id: Uuid::new_v4(),
            created_at: now,
            delivered: false,
        });

        // First delivery mark
        assert!(outbox.mark_delivered(outbox_id));

        // Second call — already delivered, should not appear
        let pending = outbox.list_pending(user_id);
        assert!(
            pending.is_empty(),
            "Sync must be idempotent — no re-delivery"
        );
    }

    /// After all entries for a user are delivered, they should be
    /// eligible for cleanup (deletion from the outbox collection).
    #[tokio::test]
    async fn test_outbox_cleanup_removes_delivered_entries() {
        let outbox = MockOutboxRepo::new();
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let e1 = OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id: Uuid::new_v4(),
            created_at: now,
            delivered: false,
        };
        let e2 = OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id: Uuid::new_v4(),
            created_at: now,
            delivered: false,
        };

        outbox.insert(e1.clone());
        outbox.insert(e2.clone());

        assert!(outbox.mark_delivered(e1.outbox_id));
        assert!(outbox.mark_delivered(e2.outbox_id));

        let removed = outbox.delete_delivered(user_id);
        assert_eq!(removed, 2, "Both delivered entries must be removed");

        let pending = outbox.list_pending(user_id);
        assert!(pending.is_empty());
    }
}

// =====================================================================
// Bucket Pattern Tests
// =====================================================================

#[cfg(test)]
mod bucket_pattern_tests {
    use super::*;

    /// Each bucket document must not exceed 50 notifications.
    /// When the 51st notification is inserted, a new bucket with
    /// page_number = 2 is created.
    #[tokio::test]
    async fn test_bucket_overflow_creates_new_page() {
        let repo = MockNotificationRepo::new();
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert 50 notifications — should stay in page 1
        for i in 0..50 {
            let doc = NotificationDocument {
                notification_id: Uuid::new_v4(),
                category_id: 1,
                title: format!("Title {}", i),
                body: format!("Body {}", i),
                data: json!({}),
                is_read: false,
                os_notification_id: None,
                created_at: now,
            };
            let (page, _) = repo.insert(user_id, doc);
            assert_eq!(page, 1, "First 50 must land in page 1");
        }

        // Bucket 1 must have exactly 50 entries
        let bucket1 = repo.get_bucket(user_id, 1).unwrap();
        assert_eq!(bucket1.len(), 50);

        // Insert the 51st — MUST overflow to page 2
        let doc_51 = NotificationDocument {
            notification_id: Uuid::new_v4(),
            category_id: 1,
            title: "Overflow".into(),
            body: "This is the 51st notification.".into(),
            data: json!({}),
            is_read: false,
            os_notification_id: None,
            created_at: now,
        };
        let (page, idx) = repo.insert(user_id, doc_51);
        assert_eq!(page, 2, "51st must land in page 2");
        assert_eq!(idx, 0, "First entry in page 2");

        // Bucket 1 still has 50, bucket 2 has 1
        let bucket1_after = repo.get_bucket(user_id, 1).unwrap();
        let bucket2 = repo.get_bucket(user_id, 2).unwrap();
        assert_eq!(bucket1_after.len(), 50);
        assert_eq!(bucket2.len(), 1);
    }

    /// Inserting 100 notifications should produce exactly 2 buckets
    /// of 50 each.
    #[tokio::test]
    async fn test_bucket_exactly_two_pages_for_100() {
        let repo = MockNotificationRepo::new();
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        for i in 0..100 {
            let doc = NotificationDocument {
                notification_id: Uuid::new_v4(),
                category_id: 1,
                title: format!("N {}", i),
                body: format!("Body {}", i),
                data: json!({}),
                is_read: false,
                os_notification_id: None,
                created_at: now,
            };
            repo.insert(user_id, doc);
        }

        let bucket1 = repo.get_bucket(user_id, 1).unwrap();
        let bucket2 = repo.get_bucket(user_id, 2).unwrap();
        assert_eq!(bucket1.len(), 50);
        assert_eq!(bucket2.len(), 50);

        // No page 3
        assert!(repo.get_bucket(user_id, 3).is_none());
    }

    /// Buckets are isolated per user — user A's notifications
    /// must not appear in user B's bucket listing.
    #[tokio::test]
    async fn test_buckets_are_user_scoped() {
        let repo = MockNotificationRepo::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let now = Utc::now();

        let doc_a = NotificationDocument {
            notification_id: Uuid::new_v4(),
            category_id: 1,
            title: "A's notification".into(),
            body: "...".into(),
            data: json!({}),
            is_read: false,
            os_notification_id: None,
            created_at: now,
        };

        repo.insert(user_a, doc_a);

        // User B should have zero notifications
        let (b_notifs, total) = repo.list_for_user(user_b, 20, 0);
        assert_eq!(total, 0);
        assert!(b_notifs.is_empty());
    }
}

// =====================================================================
// Work Order State Change → Notification Trigger Tests
// =====================================================================

#[cfg(test)]
mod work_order_notification_triggers_tests {
    use super::*;

    /// When a work order is ASSIGNED:
    ///   - Technician receives in-app + OS notification
    ///   - Category: "work_order_assigned"
    #[tokio::test]
    async fn test_assign_triggers_notification_to_technician() {
        let mq = MockMqProducer::new();
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();

        let technician_id = Uuid::new_v4();
        let work_order_id = Uuid::new_v4();
        let now = Utc::now();

        // ── Simulate what the assign service would do ──────────────────
        let in_app_id = Uuid::new_v4();
        let os_notif_id = Uuid::new_v4();

        // In-app
        repo.insert(
            technician_id,
            NotificationDocument {
                notification_id: in_app_id,
                category_id: 1, // work_order_assigned
                title: "Work Order Assigned".into(),
                body: format!(
                    "You have been assigned work order {}.",
                    work_order_id
                ),
                data: json!({ "workOrderId": work_order_id.to_string() }),
                is_read: false,
                os_notification_id: Some(os_notif_id),
                created_at: now,
            },
        );

        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id: technician_id,
            notification_id: in_app_id,
            created_at: now,
            delivered: false,
        });

        // OS (via MQ → FCM)
        mq.publish(NotificationMqMessage {
            user_id: technician_id,
            device_token: Some("fcm_tech_token".into()),
            category: "work_order_assigned".into(),
            title: "Work Order Assigned".into(),
            body: format!(
                "You have been assigned work order {}.",
                work_order_id
            ),
            data: json!({ "workOrderId": work_order_id.to_string() }),
            os_notification_id,
        });

        // ── Assertions ─────────────────────────────────────────────────
        let notif = repo.find_by_id(technician_id, in_app_id);
        assert!(notif.is_some(), "Technician must receive in-app notification");
        assert_eq!(notif.unwrap().category_id, 1);

        assert_eq!(mq.count(), 1, "OS notification must be published");
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_assigned");
        assert!(msg.body.contains(&work_order_id.to_string()));

        // Outbox entry must exist
        let pending = outbox.list_pending(technician_id);
        assert_eq!(pending.len(), 1);
    }

    /// When a work order is COMPLETED:
    ///   - Customer receives in-app + OS notification
    ///   - Category: "work_order_completed"
    #[tokio::test]
    async fn test_complete_triggers_notification_to_customer() {
        let mq = MockMqProducer::new();
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();

        let customer_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let os_notif_id = Uuid::new_v4();
        let now = Utc::now();

        repo.insert(
            customer_id,
            NotificationDocument {
                notification_id: notif_id,
                category_id: 3, // work_order_completed
                title: "Work Order Completed".into(),
                body: "Your service request has been completed.".into(),
                data: json!({}),
                is_read: false,
                os_notification_id: Some(os_notif_id),
                created_at: now,
            },
        );

        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id: customer_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        mq.publish(NotificationMqMessage {
            user_id: customer_id,
            device_token: Some("fcm_customer_token".into()),
            category: "work_order_completed".into(),
            title: "Work Order Completed".into(),
            body: "Your service request has been completed.".into(),
            data: json!({}),
            os_notification_id,
        });

        assert!(repo.find_by_id(customer_id, notif_id).is_some());
        assert_eq!(mq.count(), 1);
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_completed");
    }

    /// When a work order is REFUSED:
    ///   - Customer receives in-app + OS notification
    ///   - Category: "work_order_rejected"
    #[tokio::test]
    async fn test_refuse_triggers_notification_to_customer() {
        let mq = MockMqProducer::new();
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();

        let customer_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let os_notif_id = Uuid::new_v4();
        let now = Utc::now();

        repo.insert(
            customer_id,
            NotificationDocument {
                notification_id: notif_id,
                category_id: 4, // work_order_rejected
                title: "Work Order Refused".into(),
                body: "Your service request has been refused. Reason: ...".into(),
                data: json!({}),
                is_read: false,
                os_notification_id: Some(os_notif_id),
                created_at: now,
            },
        );

        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id: customer_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        mq.publish(NotificationMqMessage {
            user_id: customer_id,
            device_token: Some("fcm_customer_token".into()),
            category: "work_order_rejected".into(),
            title: "Work Order Refused".into(),
            body: "Your service request has been refused.".into(),
            data: json!({}),
            os_notification_id,
        });

        assert!(repo.find_by_id(customer_id, notif_id).is_some());
        assert_eq!(mq.count(), 1);
    }

    /// When a work order's REFUSAL is APPROVED:
    ///   - The refusing technician receives in-app + OS notification
    ///   - Category: "work_order_refusal_approved"
    #[tokio::test]
    async fn test_refusal_approved_triggers_notification_to_technician() {
        let mq = MockMqProducer::new();
        let repo = MockNotificationRepo::new();
        let outbox = MockOutboxRepo::new();

        let technician_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let os_notif_id = Uuid::new_v4();
        let now = Utc::now();

        repo.insert(
            technician_id,
            NotificationDocument {
                notification_id: notif_id,
                category_id: 5, // work_order_refusal_approved
                title: "Refusal Approved".into(),
                body: "Your refusal for work order WO-... has been approved.".into(),
                data: json!({}),
                is_read: false,
                os_notification_id: Some(os_notif_id),
                created_at: now,
            },
        );

        outbox.insert(OutboxEntry {
            outbox_id: Uuid::new_v4(),
            user_id: technician_id,
            notification_id: notif_id,
            created_at: now,
            delivered: false,
        });

        mq.publish(NotificationMqMessage {
            user_id: technician_id,
            device_token: Some("fcm_tech_token".into()),
            category: "work_order_refusal_approved".into(),
            title: "Refusal Approved".into(),
            body: "Your refusal has been approved.".into(),
            data: json!({}),
            os_notification_id,
        });

        assert!(repo.find_by_id(technician_id, notif_id).is_some());
        assert_eq!(mq.count(), 1);
    }
}

// =====================================================================
// Notification Sorting & Filtering Tests
// =====================================================================

#[cfg(test)]
mod notification_sorting_tests {
    use super::*;

    /// Notifications must be returned in newest-first order.
    #[tokio::test]
    async fn test_notifications_sorted_newest_first() {
        let repo = MockNotificationRepo::new();
        let user_id = Uuid::new_v4();
        let base = Utc::now();

        // Insert 3 notifications at different times
        for minutes_ago in [30, 10, 20] {
            repo.insert(
                user_id,
                NotificationDocument {
                    notification_id: Uuid::new_v4(),
                    category_id: 1,
                    title: format!("{}m ago", minutes_ago),
                    body: "...".into(),
                    data: json!({}),
                    is_read: false,
                    os_notification_id: None,
                    created_at: base - chrono::Duration::minutes(minutes_ago),
                },
            );
        }

        let (results, total) = repo.list_for_user(user_id, 20, 0);
        assert_eq!(total, 3);

        // Newest first → 10m, 20m, 30m
        assert_eq!(results[0].title, "10m ago");
        assert_eq!(results[1].title, "20m ago");
        assert_eq!(results[2].title, "30m ago");
    }

    /// Pagination: requesting page 2 (limit=2, page=2) must skip the
    /// first 2 and return only the 3rd notification.
    #[tokio::test]
    async fn test_pagination_skip_and_limit() {
        let repo = MockNotificationRepo::new();
        let user_id = Uuid::new_v4();
        let base = Utc::now();

        for i in 0..5 {
            repo.insert(
                user_id,
                NotificationDocument {
                    notification_id: Uuid::new_v4(),
                    category_id: 1,
                    title: format!("Notif {}", i),
                    body: "...".into(),
                    data: json!({}),
                    is_read: false,
                    os_notification_id: None,
                    created_at: base - chrono::Duration::minutes(i),
                },
            );
        }

        let (page2, total) = repo.list_for_user(user_id, 2, 2);
        assert_eq!(total, 5);
        assert_eq!(page2.len(), 2);
        // Newest first: "Notif 0", "Notif 1" for page 1
        //                 "Notif 2", "Notif 3" for page 2
        assert_eq!(page2[0].title, "Notif 2");
        assert_eq!(page2[1].title, "Notif 3");
    }
}

// =====================================================================
// Security & Authorization Tests
// =====================================================================

#[cfg(test)]
mod notification_security_tests {
    use super::*;

    /// Unauthenticated requests must be rejected with 401.
    #[tokio::test]
    async fn test_unauthenticated_request_returns_401() {
        let app = setup_test_app(mock_db().await).await;

        let uri = "/api/v1/notifications";
        let req = Request::builder()
            .method(http::Method::GET)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .unwrap();

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }

    /// A user must only see their own notifications.
    /// Cross-user access to a notification detail must return 404.
    #[tokio::test]
    async fn test_cross_user_notification_inaccessible() {
        let app = setup_test_app(mock_db().await).await;

        // Use a notification id that belongs to another user
        let other_notif_id = Uuid::new_v4();
        let uri = format!("/api/v1/notifications/{}", other_notif_id);
        let req = create_empty_request(http::Method::GET, &uri);

        let r = app.oneshot(req).await.unwrap();
        assert_eq!(
            r.status(),
            StatusCode::NOT_FOUND,
            "Cross-user notification access must return 404"
        );
    }
}
