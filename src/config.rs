use std::sync::OnceLock;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_sign_key: String,
    pub port: u16,

    #[serde(default = "default_access_token_ttl")]
    pub access_token_ttl_seconds: i64,

    #[serde(default = "default_session_ttl")]
    pub session_ttl_seconds: i64,
}

fn default_access_token_ttl() -> i64 { 3600 }
fn default_session_ttl() -> i64 { 86400 }

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    /// Initializes the application configuration reading natively from the environment using Envy structurally.
    pub fn init() {
        dotenvy::dotenv().ok();
        
        let config = envy::from_env::<AppConfig>()
            .expect("Failed to parse configuration variables from environment!");
        
        CONFIG.set(config).expect("Config is already initialized");
    }

    /// Retrieve the statically loaded global configuration natively
    pub fn get() -> &'static AppConfig {
        CONFIG.get().expect("AppConfig is not initialized! Call init() first.")
    }
}
