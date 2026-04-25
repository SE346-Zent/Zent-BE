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

    #[serde(rename = "docs_username")]
    pub docs_username: String,

    #[serde(rename = "docs_password")]
    pub docs_password: String,

    #[serde(default = "default_app_stage")]
    pub app_stage: String,

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
