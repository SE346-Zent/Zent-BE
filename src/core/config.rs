use std::sync::OnceLock;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_sign_key: String,
    pub port: u16,
    pub smtp_password: String,
    pub smtp_username: String,
    pub rabbitmq_url: String,
    pub valkey_url: String,
    pub mongodb_url: String,

    #[serde(rename = "nominatim_user_agent")]
    pub nominatim_user_agent: String,

    pub par_write_work_orders: String,
    pub par_read_work_orders: String,

    #[serde(rename = "docs_username")]
    pub docs_username: String,

    #[serde(rename = "docs_password")]
    pub docs_password: String,

    #[serde(default = "default_app_stage")]
    pub app_stage: String,

    pub system_user_id: uuid::Uuid,

    #[serde(default = "default_access_token_ttl")]
    pub access_token_ttl_seconds: i64,

    #[serde(default = "default_session_ttl")]
    pub session_ttl_seconds: i64,

    #[serde(default = "default_db_max_connections")]
    pub db_max_connections: u32,

    #[serde(default = "default_db_min_connections")]
    pub db_min_connections: u32,

    #[serde(default = "default_db_connect_timeout")]
    pub db_connect_timeout_seconds: u64,

    #[serde(default = "default_db_acquire_timeout")]
    pub db_acquire_timeout_seconds: u64,

    #[serde(default = "default_db_idle_timeout")]
    pub db_idle_timeout_seconds: u64,

    #[serde(default = "default_db_max_lifetime")]
    pub db_max_lifetime_seconds: u64,

    pub otel_exporter_otlp_endpoint: Option<String>,
    pub otel_exporter_otlp_headers: Option<String>,
    pub otel_service_name: Option<String>,

    /// Path to Firebase service account JSON credentials.
    /// Used by the FCM consumer to authenticate via the Firebase Admin SDK v1 API.
    /// This is the same as the `GOOGLE_APPLICATION_CREDENTIALS` env var.
    pub google_application_credentials: Option<String>,

    /// Path to save completed checklists.
    #[serde(default = "default_checklist_save_path")]
    pub checklist_save_path: String,

    /// TTL (seconds) for the short-lived idempotency claim while the DB write is in-flight.
    #[serde(default = "default_idempotency_claim_ttl")]
    pub idempotency_claim_ttl_seconds: u64,

    /// TTL (seconds) for the finalised idempotency response cache.
    #[serde(default = "default_idempotency_final_ttl")]
    pub idempotency_final_ttl_seconds: u64,

    /// Max number of poll retries when a concurrent request holds the claim.
    #[serde(default = "default_idempotency_poll_retries")]
    pub idempotency_poll_retries: u32,

    /// Delay (milliseconds) between poll retries.
    #[serde(default = "default_idempotency_poll_delay")]
    pub idempotency_poll_delay_ms: u64,

    /// Directory containing Lua scripts (verify_otp.lua, check_idempotency.lua).
    /// Loaded at startup and registered into Valkey/Redis.
    #[serde(default = "default_lua_script_dir")]
    pub lua_script_dir: String,

    /// Directory containing HTML email templates.
    /// Scanned at startup; each .html file becomes a template by filename.
    #[serde(default = "default_template_dir")]
    pub template_dir: String,
}

fn default_access_token_ttl() -> i64 { 3600 }
fn default_session_ttl() -> i64 { 86400 }
fn default_app_stage() -> String { "local".to_string() }

fn default_db_max_connections() -> u32 { 100 }
fn default_db_min_connections() -> u32 { 5 }
fn default_db_connect_timeout() -> u64 { 30 }
fn default_db_acquire_timeout() -> u64 { 30 }
fn default_db_idle_timeout() -> u64 { 600 }
fn default_db_max_lifetime() -> u64 { 1800 }

fn default_checklist_save_path() -> String { "zent_checklist".to_string() }

fn default_idempotency_claim_ttl() -> u64 { 30 }
fn default_idempotency_final_ttl() -> u64 { 3600 }
fn default_idempotency_poll_retries() -> u32 { 6 }
fn default_idempotency_poll_delay() -> u64 { 500 }
fn default_lua_script_dir() -> String { "lua_script".to_string() }
fn default_template_dir() -> String { "templates".to_string() }

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    /// Initializes the application configuration reading natively from the environment using Envy structurally.
    pub fn init() {
        dotenvy::dotenv().ok();

        CONFIG.get_or_init(|| {
            envy::from_env::<AppConfig>()
                .expect("Failed to parse configuration variables from environment!")
        });
    }

    /// Retrieve the statically loaded global configuration natively
    pub fn get() -> &'static AppConfig {
        CONFIG.get().expect("AppConfig is not initialized! Call init() first.")
    }
}
