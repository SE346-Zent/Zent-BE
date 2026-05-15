use mongodb::{options::ClientOptions, Client};

use crate::core::config::AppConfig;

/// Initialize MongoDB: connect to the cluster and run migrations.
pub async fn init_mongodb(cfg: &AppConfig) -> Result<Client, Box<dyn std::error::Error>> {
    let client_options = ClientOptions::parse(&cfg.mongodb_url).await?;
    let client = Client::with_options(client_options)?;

    // We expect the database name to be present in the connection string.
    let db_name = client
        .default_database()
        .map(|db| db.name().to_string())
        .ok_or("MongoDB connection string must include a database name")?;

    tracing::info!("Connected to MongoDB, database: {}", db_name);

    Ok(client)
}
