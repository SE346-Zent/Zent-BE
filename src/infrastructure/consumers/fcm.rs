use std::sync::Arc;
use std::time::{Duration, Instant};
use lapin::{
    options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use sea_orm::{EntityTrait, Set, ActiveModelTrait};
use tracing::{info, error, warn};
use tokio::sync::Mutex;
use tokio::time::sleep;
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};

use crate::core::config::AppConfig;
use crate::entities::outbox_records;
use crate::infrastructure::mq::fcm::{FCM_QUEUE, setup_fcm_topology};

/// Payload received from the FCM queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FcmMessage {
    pub notification_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub fcm_token: String,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
}

/// Firebase service account credentials loaded from the JSON file.
struct FcmCredentials {
    project_id: String,
    client_email: String,
    private_key: String,
}

/// Cached OAuth2 access token with its expiry.
struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// Try to resolve the path to the Firebase service account credentials file.
/// Checks (in order): AppConfig, process environment, .env file.
fn resolve_credentials_path() -> Result<String, anyhow::Error> {
    // 1. AppConfig (envy deserialization)
    if let Some(p) = AppConfig::get().google_application_credentials.as_deref() {
        if !p.is_empty() {
            return Ok(p.to_string());
        }
    }

    // 2. Process environment (std::env::var)
    if let Ok(p) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        if !p.is_empty() {
            return Ok(p);
        }
    }

    // 3. Parse .env file directly (fallback in case dotenvy didn't load it)
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || !trimmed.contains('=') {
                continue;
            }
            if let Some(key_end) = trimmed.find('=') {
                let key = trimmed[..key_end].trim();
                if key.eq_ignore_ascii_case("GOOGLE_APPLICATION_CREDENTIALS") {
                    let val = trimmed[key_end + 1..].trim().to_string();
                    if !val.is_empty() {
                        return Ok(val);
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("GOOGLE_APPLICATION_CREDENTIALS not found"))
}

impl FcmCredentials {
    /// Load credentials from the path specified in `google_application_credentials` config.
    /// Tries multiple sources in order:
    /// 1. AppConfig (envy deserialization)
    /// 2. `std::env::var("GOOGLE_APPLICATION_CREDENTIALS")` (process environment)
    /// 3. Parse `.env` file directly (in case dotenvy didn't load it)
    fn from_config() -> Result<Self, anyhow::Error> {
        let path = resolve_credentials_path()
            .map_err(|_| anyhow::anyhow!(
                "GOOGLE_APPLICATION_CREDENTIALS not found — add it to your .env"
            ))?;

        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read credentials file '{}': {}", path, e))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse credentials JSON: {}", e))?;

        Ok(FcmCredentials {
            project_id: json["project_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'project_id' in credentials"))?
                .to_string(),
            client_email: json["client_email"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'client_email' in credentials"))?
                .to_string(),
            private_key: json["private_key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'private_key' in credentials"))?
                .to_string(),
        })
    }

    /// Obtain (or refresh) an OAuth2 access token for the Firebase Messaging scope.
    async fn get_access_token(&self, cache: &Mutex<Option<CachedToken>>) -> Result<String, anyhow::Error> {
        // Check if we have a cached token that's still valid (> 5 min buffer)
        {
            let guard = cache.lock().await;
            if let Some(cached) = guard.as_ref() {
                if Instant::now() < cached.expires_at {
                    return Ok(cached.token.clone());
                }
            }
        }

        // 1. Create the JWT assertion (RFC 7523)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as usize;

        #[derive(Serialize)]
        struct JwtClaims {
            iss: String,
            scope: String,
            aud: String,
            exp: usize,
            iat: usize,
        }

        let claims = JwtClaims {
            iss: self.client_email.clone(),
            scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
            aud: "https://oauth2.googleapis.com/token".to_string(),
            exp: now + 3600,
            iat: now,
        };

        let assertion = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(self.private_key.as_bytes())?,
        )?;

        // 2. Exchange assertion for access token
        let client = reqwest::Client::new();
        let body = format!(
            "grant_type={}&assertion={}",
            urlencoding::encode("urn:ietf:params:oauth:grant-type:jwt-bearer"),
            urlencoding::encode(&assertion),
        );

        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OAuth2 token request failed: {}", e))?;

        let token_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse OAuth2 token response: {}", e))?;

        let access_token = token_body["access_token"]
            .as_str()
            .ok_or_else(|| {
                let err_desc = token_body["error_description"]
                    .as_str()
                    .unwrap_or("unknown OAuth error");
                anyhow::anyhow!("Token exchange failed: {}", err_desc)
            })?
            .to_string();

        let expires_in = token_body["expires_in"].as_i64().unwrap_or(3600);
        let expires_at = Instant::now() + Duration::from_secs(expires_in as u64 - 300); // 5 min buffer

        // Cache the token
        let mut guard = cache.lock().await;
        *guard = Some(CachedToken {
            token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }

    /// Send a push notification via Firebase Cloud Messaging v1 HTTP API.
    async fn send_push(&self, token_cache: &Mutex<Option<CachedToken>>, msg: &FcmMessage) -> bool {
        let access_token = match self.get_access_token(token_cache).await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to obtain FCM access token: {:?}", e);
                return false;
            }
        };

        let fcm_payload = serde_json::json!({
            "message": {
                "token": msg.fcm_token,
                "notification": {
                    "title": msg.title,
                    "body": msg.body,
                },
                "data": {
                    "notificationId": msg.notification_id.to_string(),
                    "userId": msg.user_id.to_string(),
                    "payload": serde_json::to_string(&msg.data).unwrap_or_default(),
                },
            }
        });

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        );

        let client = reqwest::Client::new();
        match client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&fcm_payload)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    info!("FCM push sent to user {}", msg.user_id);
                    true
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    error!("FCM push failed ({}): {}", status, body);
                    false
                }
            }
            Err(e) => {
                error!("FCM HTTP request failed: {:?}", e);
                false
            }
        }
    }
}

/// Start the FCM consumer background task.
///
/// Listens on `fcm_queue`, sends push via Firebase Cloud Messaging v1 API
/// (using service account credentials from `GOOGLE_APPLICATION_CREDENTIALS`),
/// and updates the MySQL `outbox_records` row to `delivered = true` on success.
pub async fn start_fcm_consumer(
    connection: Option<Arc<Connection>>,
    db: sea_orm::DatabaseConnection,
) {
    let mut conn_opt = match connection {
        Some(c) => c,
        None => return,
    };

    // Load service account credentials at startup
    let creds = match FcmCredentials::from_config() {
        Ok(c) => c,
        Err(e) => {
            error!("FCM consumer: Failed to load credentials: {:?}. FCM pushes will be DISABLED.", e);
            return;
        }
    };
    let token_cache: Arc<Mutex<Option<CachedToken>>> = Arc::new(Mutex::new(None));

    let url = AppConfig::get().rabbitmq_url.clone();

    tokio::spawn(async move {
        loop {
            let channel = match conn_opt.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!("FCM consumer: Failed to create channel: {:?}. Reconnecting...", e);
                    // Try to create a fresh connection
                    match Connection::connect(
                        &url,
                        ConnectionProperties::default(),
                    ).await {
                        Ok(new_conn) => {
                            conn_opt = Arc::new(new_conn);
                            continue;
                        }
                        Err(re) => {
                            error!("FCM consumer: Failed to reconnect: {:?}. Retrying in 5s...", re);
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            };

            if let Err(e) = setup_fcm_topology(&channel).await {
                error!("FCM consumer: Failed to setup topology: {:?}. Retrying in 5s...", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel.basic_consume(
                FCM_QUEUE,
                "fcm_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await {
                Ok(c) => c,
                Err(e) => {
                    error!("FCM consumer: Failed to attach: {:?}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("FCM Consumer listening on {}", FCM_QUEUE);

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<FcmMessage>(payload_str) {
                                Ok(msg) => {
                                    let success = creds.send_push(&token_cache, &msg).await;
                                    if success {
                                        // Update outbox record to delivered=true
                                        if let Err(e) = mark_outbox_delivered(&db, msg.notification_id).await {
                                            error!("FCM consumer: Failed to update outbox: {:?}", e);
                                        }
                                        let _ = delivery.ack(BasicAckOptions::default()).await;
                                    } else {
                                        error!("FCM consumer: Push failed for notif {}", msg.notification_id);
                                        let _ = delivery.nack(BasicNackOptions {
                                            requeue: false,
                                            ..Default::default()
                                        }).await;
                                    }
                                }
                                Err(e) => {
                                    error!("FCM consumer: Invalid message: {:?}", e);
                                    let _ = delivery.nack(BasicNackOptions {
                                        requeue: false,
                                        ..Default::default()
                                    }).await;
                                }
                            }
                        } else {
                            let _ = delivery.nack(BasicNackOptions {
                                requeue: false,
                                ..Default::default()
                            }).await;
                        }
                    }
                    Err(e) => {
                        error!("FCM consumer: Stream error: {:?}", e);
                        break;
                    }
                }
            }

            warn!("FCM consumer loop exited, reconnecting in 5s...");
            sleep(Duration::from_secs(5)).await;
        }
    });
}

/// Mark the outbox record for this notification as delivered.
async fn mark_outbox_delivered(
    db: &sea_orm::DatabaseConnection,
    notification_id: uuid::Uuid,
) -> Result<(), anyhow::Error> {
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let entries = outbox_records::Entity::find()
        .filter(outbox_records::Column::NotificationId.eq(notification_id))
        .all(db)
        .await?;

    for entry in entries {
        let mut active: outbox_records::ActiveModel = entry.into();
        active.delivered = Set(true);
        active.update(db).await?;
    }

    Ok(())
}
