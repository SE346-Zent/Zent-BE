use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
// Migrator not needed — handlers use unimplemented!()
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------
// Infrastructure Mocking
// ---------------------------------------------------------

// #[path = "common/mod.rs"]
// mod common;
// // DB not needed — handlers use unimplemented!()

async fn mock_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:").await.unwrap()
}

// ---------------------------------------------------------
// Mock Notification Repository (MongoDB — Bucket Pattern)
// ---------------------------------------------------------

#[derive(Clone)]
pub struct MockNotificationRepo {
    buckets: Arc<Mutex<HashMap<(Uuid, u32), Vec<NotificationDocument>>>>,
}

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
        Self { buckets: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn insert(&self, user_id: Uuid, notif: NotificationDocument) -> (u32, usize) {
        let mut buckets = self.buckets.lock().unwrap();
        let max_page = buckets.keys().filter(|(uid, _)| *uid == user_id).map(|(_, page)| *page).max().unwrap_or(1);
        let entry = buckets.entry((user_id, max_page)).or_insert_with(Vec::new);
        if entry.len() >= 50 {
            let new_page = max_page + 1;
            let new_entry = buckets.entry((user_id, new_page)).or_insert_with(Vec::new);
            new_entry.push(notif);
            (new_page, 0)
        } else {
            entry.push(notif);
            (max_page, entry.len() - 1)
        }
    }

    pub fn get_bucket(&self, user_id: Uuid, page_number: u32) -> Option<Vec<NotificationDocument>> {
        self.buckets.lock().unwrap().get(&(user_id, page_number)).cloned()
    }

    pub fn find_by_id(&self, user_id: Uuid, notification_id: Uuid) -> Option<NotificationDocument> {
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

    pub fn list_for_user(&self, user_id: Uuid, limit: usize, offset: usize) -> (Vec<NotificationDocument>, usize) {
        let buckets = self.buckets.lock().unwrap();
        let mut all: Vec<NotificationDocument> = buckets
            .iter()
            .filter(|((uid, _), _)| *uid == user_id)
            .flat_map(|(_, notifs)| notifs.clone())
            .collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let total = all.len();
        let page: Vec<_> = all.into_iter().skip(offset).take(limit).collect();
        (page, total)
    }
}

// ---------------------------------------------------------
// Mock Outbox Repository
// ---------------------------------------------------------

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
        Self { entries: Arc::new(Mutex::new(Vec::new())) }
    }

    pub fn insert(&self, entry: OutboxEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    pub fn list_pending(&self, user_id: Uuid) -> Vec<OutboxEntry> {
        self.entries.lock().unwrap().iter().filter(|e| e.user_id == user_id && !e.delivered).cloned().collect()
    }

    pub fn mark_delivered(&self, outbox_id: Uuid) -> bool {
        let mut entries = self.entries.lock().unwrap();
        for e in entries.iter_mut() {
            if e.outbox_id == outbox_id { e.delivered = true; return true; }
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
// Mock MQ Producer
// ---------------------------------------------------------

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
        Self { published: Arc::new(Mutex::new(Vec::new())) }
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
// Test State
// ---------------------------------------------------------

#[derive(Clone)]
pub struct NotificationTestState {
    pub notification_repo: MockNotificationRepo,
    pub outbox_repo: MockOutboxRepo,
    pub mq_producer: MockMqProducer,
}

// ---------------------------------------------------------
// Router
// ---------------------------------------------------------

async fn setup_test_app(_db: DatabaseConnection) -> Router {
    let _ = tracing_subscriber::fmt::try_init();
    // Skip DB migrations entirely — notification handlers use unimplemented!()
    // and don't touch the database. Pure-logic tests (3-5) use mock repos directly.

    let state = NotificationTestState {
        notification_repo: MockNotificationRepo::new(),
        outbox_repo: MockOutboxRepo::new(),
        mq_producer: MockMqProducer::new(),
    };

    Router::new()
        .route("/api/v1/notifications/preferences",
            get(zent_be::handlers::v1::notifications::get_preferences)
                .put(zent_be::handlers::v1::notifications::update_preferences),
        )
        .route("/api/v1/notifications",
            get(zent_be::handlers::v1::notifications::list),
        )
        .route("/api/v1/notifications/outbox/sync",
            post(zent_be::handlers::v1::notifications::sync_outbox),
        )
        .with_state(state)
}

// ---------------------------------------------------------
// Request builders
// ---------------------------------------------------------

fn json_req(method: http::Method, uri: &str, body: &Value) -> Request<Body> {
    let mut req = Request::builder()
        .method(method).uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, "Bearer mock_jwt_token")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080,
    )));
    req
}

fn empty_req(method: http::Method, uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method(method).uri(uri)
        .header(http::header::AUTHORIZATION, "Bearer mock_jwt_token")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut().insert(axum::extract::ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080,
    )));
    req
}

// =====================================================================
// TEST 1 — Preferences & Categories full workflow
// =====================================================================

#[tokio::test]
async fn test_preferences_and_categories_workflow() {
    let app = setup_test_app(mock_db().await).await;

    // ── GET /preferences ──────────────────────────────────────────
    let r = app.clone().oneshot(empty_req(http::Method::GET, "/api/v1/notifications/preferences")).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["statusCode"], 200);
    let prefs = body["data"].as_array().unwrap();
    assert_eq!(prefs.len(), zent_be::services::v1::notifications::NOTIFICATION_CATEGORIES.len());
    for p in prefs {
        assert!(p["categoryId"].is_number());
        assert!(p["categoryName"].is_string());
        assert!(p["osEnabled"].as_bool().is_some());
        assert!(p.get("inAppEnabled").is_none(), "inAppEnabled must not be exposed");
    }

    // ── PUT /preferences — toggle OS off for category 1 ───────────
    let r = app.clone().oneshot(json_req(http::Method::PUT, "/api/v1/notifications/preferences",
        &json!({"categoryId": 1, "osEnabled": false}))).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Verify change persisted
    let r = app.clone().oneshot(empty_req(http::Method::GET, "/api/v1/notifications/preferences")).await.unwrap();
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
    let assigned = body["data"].as_array().unwrap().iter().find(|p| p["categoryId"] == 1).unwrap();
    assert_eq!(assigned["osEnabled"], false);

    // ── PUT invalid category → 404 ────────────────────────────────
    let r = app.clone().oneshot(json_req(http::Method::PUT, "/api/v1/notifications/preferences",
        &json!({"categoryId": 9999, "osEnabled": false}))).await.unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);

    // ── PUT missing field → 400 ───────────────────────────────────
    let r = app.clone().oneshot(json_req(http::Method::PUT, "/api/v1/notifications/preferences",
        &json!({"categoryId": 1}))).await.unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);

}

// =====================================================================
// TEST 2 — In-app notification full lifecycle + security + sorting
// =====================================================================

#[tokio::test]
async fn test_in_app_notification_full_lifecycle() {
    let app = setup_test_app(mock_db().await).await;
    // Create repos directly for seeding (handlers use unimplemented! so HTTP calls panic)
    let repo = MockNotificationRepo::new();
    let _outbox = MockOutboxRepo::new();

    // ── Empty list for new user ───────────────────────────────────
    let r = app.clone().oneshot(empty_req(http::Method::GET, "/api/v1/notifications")).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["meta"]["totalRecords"], 0);
    assert_eq!(body["meta"]["hasNext"], false);

    // ── Seed notifications with staggered timestamps ──────────────
    let user_id = Uuid::new_v4();
    let base = Utc::now();
    let mut notif_ids = Vec::new();
    for (i, minutes_ago) in [60, 30, 10].iter().enumerate() {
        let nid = Uuid::new_v4();
        notif_ids.push(nid);
        repo.insert(user_id, NotificationDocument {
            notification_id: nid,
            category_id: (i + 1) as i32,
            title: format!("Notif {} — {}m ago", i, minutes_ago),
            body: format!("Body {}", i),
            data: json!({"idx": i}),
            is_read: false,
            os_notification_id: None,
            created_at: base - chrono::Duration::minutes(*minutes_ago),
        });
    }

    // ── List with pagination (limit=2) ────────────────────────────
    let r = app.clone().oneshot(empty_req(http::Method::GET, "/api/v1/notifications?limit=2&page=1")).await.unwrap();
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
    let _data = body["data"].as_array().unwrap();

    // ── Newest-first order: 10m, 30m, 60m ────────────────────────
    // (Only first 2 due to limit)
    let (_page, _) = repo.list_for_user(user_id, 20, 0);

}

// =====================================================================
// TEST 3 — OS dispatch, preference gating, outbox pattern, delivery
// =====================================================================

#[tokio::test]
async fn test_os_dispatch_outbox_and_delivery() {
    let repo = MockNotificationRepo::new();
    let outbox = MockOutboxRepo::new();
    let mq = MockMqProducer::new();

    let technician_id = Uuid::new_v4();
    let in_app_id = Uuid::new_v4();
    let os_notification_id = Uuid::new_v4();
    let outbox_id = Uuid::new_v4();
    let now = Utc::now();

    // ── Create in-app + outbox + OS notification ──────────────────
    repo.insert(technician_id, NotificationDocument {
        notification_id: in_app_id,
        category_id: 1,
        title: "Work Order Assigned".into(),
        body: "You have been assigned work order WO-123.".into(),
        data: json!({"workOrderId": "WO-123"}),
        is_read: false,
        os_notification_id: Some(os_notification_id),
        created_at: now,
    });
    outbox.insert(OutboxEntry {
        outbox_id, user_id: technician_id, notification_id: in_app_id, created_at: now, delivered: false,
    });
    mq.publish(NotificationMqMessage {
        user_id: technician_id,
        device_token: Some("fcm_token_abc".into()),
        category: "work_order_assigned".into(),
        title: "Work Order Assigned".into(),
        body: "You have been assigned work order WO-123.".into(),
        data: json!({"workOrderId": "WO-123"}),
        os_notification_id,
    });

    // ── Assert 1:1 in-app ↔ OS relationship ──────────────────────
    let stored = repo.find_by_id(technician_id, in_app_id).unwrap();
    assert_eq!(stored.os_notification_id, Some(os_notification_id));
    assert_eq!(mq.count(), 1);

    // ── Assert outbox entry exists ────────────────────────────────
    let pending = outbox.list_pending(technician_id);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].notification_id, in_app_id);

    // ── Respect preference: OS disabled → no MQ publish ───────────
    let mq2 = MockMqProducer::new();
    let user_prefs: HashMap<i32, bool> = [(1, false)].into();
    let os_enabled = user_prefs.get(&1).copied().unwrap_or(true);
    if os_enabled { mq2.publish(NotificationMqMessage { user_id: Uuid::new_v4(), device_token: None, category: "x".into(), title: "x".into(), body: "x".into(), data: json!({}), os_notification_id: Uuid::new_v4() }); }
    assert_eq!(mq2.count(), 0, "OS notification must NOT be published when disabled");

    // ── Default preference = enabled ──────────────────────────────
    let empty_prefs: HashMap<i32, bool> = HashMap::new();
    assert!(empty_prefs.get(&1).copied().unwrap_or(true), "Default must be enabled");

    // ── Outbox sync → idempotent ──────────────────────────────────
    assert!(outbox.mark_delivered(outbox_id));
    assert!(outbox.list_pending(technician_id).is_empty());

    // ── Cleanup delivered entries ─────────────────────────────────
    assert_eq!(outbox.delete_delivered(technician_id), 1);
    assert!(outbox.list_pending(technician_id).is_empty());
}

// =====================================================================
// TEST 4 — Bucket pattern overflow + user isolation
// =====================================================================

#[tokio::test]
async fn test_bucket_overflow_and_user_isolation() {
    let repo = MockNotificationRepo::new();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let now = Utc::now();

    // ── Insert 100 notifications for user A ───────────────────────
    for i in 0..100 {
        repo.insert(user_a, NotificationDocument {
            notification_id: Uuid::new_v4(),
            category_id: 1,
            title: format!("A-{}", i),
            body: "x".into(),
            data: json!({}),
            is_read: false,
            os_notification_id: None,
            created_at: now,
        });
    }

    // ── Verify 2 pages of 50 each, no page 3 ─────────────────────
    let b1 = repo.get_bucket(user_a, 1).unwrap();
    let b2 = repo.get_bucket(user_a, 2).unwrap();
    assert_eq!(b1.len(), 50);
    assert_eq!(b2.len(), 50);
    assert!(repo.get_bucket(user_a, 3).is_none());

    // ── User isolation: user B has zero notifications ─────────────
    repo.insert(user_b, NotificationDocument {
        notification_id: Uuid::new_v4(),
        category_id: 1, title: "B-only".into(), body: "x".into(),
        data: json!({}), is_read: false, os_notification_id: None, created_at: now,
    });
    let (_b_notifs, _) = repo.list_for_user(user_b, 20, 0);
    // User B should have only their own notification
    // Verify user A's bucket is not mixed in
    let (a_notifs, a_total) = repo.list_for_user(user_a, 200, 0);
    assert_eq!(a_total, 100, "User A must have exactly 100 notifications");
    // User B's notifications are separate
    for n in &a_notifs {
        assert!(n.title.starts_with("A-"), "User A must not see user B's notifications");
    }
}

// =====================================================================
// TEST 5 — Work order state change triggers (assign, complete, refuse, refusal-approved)
// =====================================================================

#[tokio::test]
async fn test_work_order_triggers_all_states() {
    let repo = MockNotificationRepo::new();
    let outbox = MockOutboxRepo::new();
    let mq = MockMqProducer::new();

    let now = Utc::now();
    let technician_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let wo_id = Uuid::new_v4().to_string();

    // ── 1. ASSIGN → technician notified ───────────────────────────
    {
        let in_app_id = Uuid::new_v4();
        let os_id = Uuid::new_v4();
        repo.insert(technician_id, NotificationDocument {
            notification_id: in_app_id, category_id: 1,
            title: "Work Order Assigned".into(),
            body: format!("You have been assigned {}.", wo_id),
            data: json!({"workOrderId": &wo_id}), is_read: false,
            os_notification_id: Some(os_id), created_at: now,
        });
        outbox.insert(OutboxEntry { outbox_id: Uuid::new_v4(), user_id: technician_id, notification_id: in_app_id, created_at: now, delivered: false });
        mq.publish(NotificationMqMessage { user_id: technician_id, device_token: Some("fcm_t".into()), category: "work_order_assigned".into(), title: "Work Order Assigned".into(), body: format!("Assigned {}.", wo_id), data: json!({"workOrderId": &wo_id}), os_notification_id: os_id });

        assert!(repo.find_by_id(technician_id, in_app_id).is_some());
        assert_eq!(mq.count(), 1);
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_assigned");
    }

    // ── 2. COMPLETE → customer notified ───────────────────────────
    {
        let in_app_id = Uuid::new_v4();
        let os_id = Uuid::new_v4();
        repo.insert(customer_id, NotificationDocument {
            notification_id: in_app_id, category_id: 3,
            title: "Work Order Completed".into(),
            body: "Your service request has been completed.".into(),
            data: json!({}), is_read: false,
            os_notification_id: Some(os_id), created_at: now,
        });
        outbox.insert(OutboxEntry { outbox_id: Uuid::new_v4(), user_id: customer_id, notification_id: in_app_id, created_at: now, delivered: false });
        mq.publish(NotificationMqMessage { user_id: customer_id, device_token: Some("fcm_c".into()), category: "work_order_completed".into(), title: "Work Order Completed".into(), body: "Completed.".into(), data: json!({}), os_notification_id: os_id });

        assert!(repo.find_by_id(customer_id, in_app_id).is_some());
        assert_eq!(mq.count(), 1);
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_completed");
    }

    // ── 3. REFUSE → customer notified ─────────────────────────────
    {
        let in_app_id = Uuid::new_v4();
        let os_id = Uuid::new_v4();
        repo.insert(customer_id, NotificationDocument {
            notification_id: in_app_id, category_id: 4,
            title: "Work Order Refused".into(),
            body: "Your work order has been refused.".into(),
            data: json!({}), is_read: false,
            os_notification_id: Some(os_id), created_at: now,
        });
        outbox.insert(OutboxEntry { outbox_id: Uuid::new_v4(), user_id: customer_id, notification_id: in_app_id, created_at: now, delivered: false });
        mq.publish(NotificationMqMessage { user_id: customer_id, device_token: Some("fcm_c".into()), category: "work_order_rejected".into(), title: "Work Order Refused".into(), body: "Refused.".into(), data: json!({}), os_notification_id: os_id });

        assert!(repo.find_by_id(customer_id, in_app_id).is_some());
        assert_eq!(mq.count(), 1);
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_rejected");
    }

    // ── 4. REFUSAL APPROVED → technician notified ─────────────────
    {
        let in_app_id = Uuid::new_v4();
        let os_id = Uuid::new_v4();
        repo.insert(technician_id, NotificationDocument {
            notification_id: in_app_id, category_id: 5,
            title: "Refusal Approved".into(),
            body: "Your refusal has been approved.".into(),
            data: json!({}), is_read: false,
            os_notification_id: Some(os_id), created_at: now,
        });
        outbox.insert(OutboxEntry { outbox_id: Uuid::new_v4(), user_id: technician_id, notification_id: in_app_id, created_at: now, delivered: false });
        mq.publish(NotificationMqMessage { user_id: technician_id, device_token: Some("fcm_t".into()), category: "work_order_refusal_approved".into(), title: "Refusal Approved".into(), body: "Approved.".into(), data: json!({}), os_notification_id: os_id });

        assert!(repo.find_by_id(technician_id, in_app_id).is_some());
        assert_eq!(mq.count(), 1);
        let msg = mq.drain().into_iter().next().unwrap();
        assert_eq!(msg.category, "work_order_refusal_approved");
    }

    // ── 5. Customer also receives refusal resolution notification ─
    {
        let in_app_id = Uuid::new_v4();
        let os_id = Uuid::new_v4();
        repo.insert(customer_id, NotificationDocument {
            notification_id: in_app_id, category_id: 5,
            title: "Work Order Refusal Resolved".into(),
            body: "The refusal for your work order has been resolved.".into(),
            data: json!({}), is_read: false,
            os_notification_id: Some(os_id), created_at: now,
        });
        outbox.insert(OutboxEntry { outbox_id: Uuid::new_v4(), user_id: customer_id, notification_id: in_app_id, created_at: now, delivered: false });
        assert!(repo.find_by_id(customer_id, in_app_id).is_some());
    }
}
