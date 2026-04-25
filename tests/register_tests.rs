use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::post,
    Router,
};
use tower::ServiceExt;

use zent_be::entities::{account_status, roles, users};
use zent_be::handlers::v1::auth::register_handler;
use zent_be::core::lookup_tables::LookupTables;
use zent_be::core::state::AppState;

use chrono::Utc;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait};
use uuid::Uuid;

// ==========================================
//  Helpers
// ==========================================

async fn seed_test_db(db: &DatabaseConnection) {
    let _ = roles::ActiveModel {
        id: Set(1),
        name: Set("Customer".to_string()),
    }
    .insert(db)
    .await;
    let _ = account_status::ActiveModel {
        id: Set(1),
        name: Set("Pending".to_string()),
    }
    .insert(db)
    .await;
    let _ = account_status::ActiveModel {
        id: Set(2),
        name: Set("Active".to_string()),
    }
    .insert(db)
    .await;
}

async fn setup_app_with_db(db: DatabaseConnection) -> Router {
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;

    // For testing registration, we need to mock Valkey and RabbitMQ if possible, 
    // but here we are testing the handler/service integration.
    // In a real scenario, we'd use a mock for state.rabbitmq and state.valkey.
    // Since we can't easily mock lapin::Connection in this environment without complex traits,
    // these tests might fail if they hit the actual RMQ/Redis logic.
    // However, I will set them to None and the handler should handle it (or we mock the service).
    
    let state = AppState::new(
        b"integration_test_secret_for_tokens", 
        db, 
        None, // Valkey None
        None, // RabbitMQ None
        900, 
        3600, 
        LookupTables::empty()
    );

    Router::new()
        .route("/register", post(register_handler))
        .with_state(state)
}

fn create_json_request(uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap()
}

#[tokio::test]
async fn test_register_new_user() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    let req_body = serde_json::json!({
        "fullName": "New User",
        "email": "new@example.com",
        "password": "password123",
        "phoneNumber": "123456789"
    });

    let req = create_json_request("/register", &req_body);
    let r = app.oneshot(req).await.unwrap();
    
    // Handler returns 200 OK by default even if the payload says 201
    assert_eq!(r.status(), StatusCode::OK);
    
    // Verify user exists in DB
    let user = users::Entity::find()
        .filter(users::Column::Email.eq("new@example.com"))
        .one(&db)
        .await
        .unwrap();
    assert!(user.is_some());
}

#[tokio::test]
async fn test_register_existing_pending_user() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    // Pre-insert a pending user
    let user_id = Uuid::new_v4();
    users::ActiveModel {
        id: Set(user_id),
        full_name: Set("Old Name".to_string()),
        email: Set("pending@example.com".to_string()),
        password_hash: Set("old_hash".to_string()),
        phone_number: Set("000".to_string()),
        account_status: Set(1), // Pending
        role_id: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
    }.insert(&db).await.unwrap();

    let req_body = serde_json::json!({
        "fullName": "Updated Name",
        "email": "pending@example.com",
        "password": "newpassword123",
        "phoneNumber": "111"
    });

    let req = create_json_request("/register", &req_body);
    let _ = app.oneshot(req).await.unwrap();

    // Verify user details are updated
    let user = users::Entity::find()
        .filter(users::Column::Email.eq("pending@example.com"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(user.full_name, "Updated Name");
    assert_eq!(user.phone_number, "111");
}

#[tokio::test]
async fn test_register_existing_active_user_conflict() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    // Pre-insert an active user
    users::ActiveModel {
        id: Set(Uuid::new_v4()),
        full_name: Set("Active User".to_string()),
        email: Set("active@example.com".to_string()),
        password_hash: Set("hash".to_string()),
        phone_number: Set("222".to_string()),
        account_status: Set(2), // Active
        role_id: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
    }.insert(&db).await.unwrap();

    let req_body = serde_json::json!({
        "fullName": "Another Name",
        "email": "active@example.com",
        "password": "password123",
        "phoneNumber": "333"
    });

    let req = create_json_request("/register", &req_body);
    let r = app.oneshot(req).await.unwrap();

    assert_eq!(r.status(), StatusCode::CONFLICT);
}
