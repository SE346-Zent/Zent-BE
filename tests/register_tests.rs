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

use migration::{Migrator, MigratorTrait};
use sea_orm::*;

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
    let _ = tracing_subscriber::fmt::try_init();
    Migrator::up(&db, None).await.unwrap();
    seed_test_db(&db).await;

    let db_mgr = zent_be::infrastructure::database::DatabaseManager::from_connection(db);
    let valkey_mgr = zent_be::infrastructure::cache::ValkeyManager::stub();
    let rmq_mgr = zent_be::infrastructure::mq::RabbitMQManager::stub();
    
    let mut templates = std::collections::HashMap::new();
    templates.insert("verification_email.html".to_string(), "Template content".to_string());
    let templates_arc = std::sync::Arc::new(templates);

    let auth_service = zent_be::services::v1::auth::AuthService::new(
        db_mgr.clone(),
        valkey_mgr.clone(),
        rmq_mgr.clone(),
        templates_arc.clone(),
        zent_be::core::state::AccessTokenDefaultTTLSeconds(900),
        zent_be::core::state::SessionDefaultTTLSeconds(3600),
        jsonwebtoken::EncodingKey::from_secret(b"integration_test_secret_for_tokens"),
    );

    let state = AppState::new(
        b"integration_test_secret_for_tokens", 
        LookupTables::empty(),
        auth_service
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

// ==========================================
//  Tests
// ==========================================

#[tokio::test]
async fn test_register_new_user() {
    let db: DatabaseConnection = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    let req_body = serde_json::json!({
        "fullName": "New User",
        "email": "new@example.com",
        "password": "password123",
        "phoneNumber": "123456789"
    });

    let req = create_json_request("/register", &req_body);
    let r = app.oneshot(req).await.unwrap();

    assert!(r.status() == StatusCode::CREATED || r.status() == StatusCode::OK);

    // Verify DB state
    let user = users::Entity::find()
        .filter(users::Column::Email.eq("new@example.com"))
        .one(&db)
        .await
        .unwrap()
        .expect("User should be in database");

    assert_eq!(user.full_name, "New User");
    assert_eq!(user.account_status, 1); // Pending
}

#[tokio::test]
async fn test_register_existing_pending_user() {
    let db: DatabaseConnection = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    // Pre-insert a pending user
    let _ = users::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        full_name: Set("Old Name".to_string()),
        email: Set("pending@example.com".to_string()),
        password_hash: Set("oldhash".to_string()),
        phone_number: Set("000".to_string()),
        account_status: Set(1), // Pending
        role_id: Set(1),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let req_body = serde_json::json!({
        "fullName": "Updated Name",
        "email": "pending@example.com",
        "password": "password123",
        "phoneNumber": "123456789"
    });

    let req = create_json_request("/register", &req_body);
    let r = app.oneshot(req).await.unwrap();

    assert!(r.status() == StatusCode::CREATED || r.status() == StatusCode::OK);

    // Verify user was updated
    let user = users::Entity::find()
        .filter(users::Column::Email.eq("pending@example.com"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(user.full_name, "Updated Name");
}

#[tokio::test]
async fn test_register_existing_active_user_conflict() {
    let db: DatabaseConnection = Database::connect("sqlite::memory:").await.unwrap();
    let app = setup_app_with_db(db.clone()).await;

    // Pre-insert an active user
    let _ = users::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        full_name: Set("Active User".to_string()),
        email: Set("active@example.com".to_string()),
        password_hash: Set("hash".to_string()),
        phone_number: Set("111".to_string()),
        account_status: Set(2), // Active
        role_id: Set(1),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let req_body = serde_json::json!({
        "fullName": "Conflict",
        "email": "active@example.com",
        "password": "password123",
        "phoneNumber": "333"
    });

    let req = create_json_request("/register", &req_body);
    let r = app.oneshot(req).await.unwrap();

    assert_eq!(r.status(), StatusCode::CONFLICT);
}
