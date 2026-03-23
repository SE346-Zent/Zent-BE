use std::env;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_sign_key: String,
    pub port: u16,
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    /// Initializes the application configuration.
    /// This should be called exactly once at the beginning of the application loop.
    pub fn init() {
        dotenvy::dotenv().ok();
        
        let config = AppConfig {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data.db?mode=rwc".to_string()),
            jwt_sign_key: env::var("JWT_SIGN_KEY")
                .unwrap_or_else(|_| "super_secret_key_change_me".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a valid u16 integer"),
        };
        
        CONFIG.set(config).expect("Config is already initialized");
    }

    /// Retrieve the statically loaded global configuration natively
    pub fn get() -> &'static AppConfig {
        CONFIG.get().expect("AppConfig is not initialized! Call init() first.")
    }
}
