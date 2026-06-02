use futures::stream::StreamExt;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_executor_trait::Tokio as TokioExecutor;
use tracing::{error, info, warn};

use crate::core::config::AppConfig;
use crate::entities::outbox_records;
use crate::infrastructure::mq::fcm::{setup_fcm_topology, FCM_QUEUE};

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
    if let Some(p) = AppConfig::get().google_application_credentials.as_deref() {
        if !p.is_empty() {
            return Ok(p.to_string());
        }
    }

    if let Ok(p) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        if !p.is_empty() {
            return Ok(p);
        }
    }

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
    /// Load credentials from the path specified in configuration layers.
    fn from_config() -> Result<Self, anyhow::Error> {
        let path = resolve_credentials_path().map_err(|_| {
            anyhow::anyhow!("GOOGLE_APPLICATION_CREDENTIALS not found — add it to your .env")
        })?;

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
    async fn get_access_token(
        &self,
        cache: &Mutex<Option<CachedToken>>,
    ) -> Result<String, anyhow::Error> {
        {
            let guard = cache.lock().await;
            if let Some(cached) = guard.as_ref() {
                if Instant::now() < cached.expires_at {
                    return Ok(cached.token.clone());
                }
            }
        }

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
        let expires_at = Instant::now() + Duration::from_secs(expires_in as u64 - 300);

        let mut guard = cache.lock().await;
        *guard = Some(CachedToken {
            token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }

    /// Send a push notification via Firebase Cloud Messaging v1 HTTP API.
    async fn send_push(
        &self,
        token_cache: &Mutex<Option<CachedToken>>,
        msg: &FcmMessage,
    ) -> Result<(), anyhow::Error> {
        let access_token = self.get_access_token(token_cache).await?;

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
                    info!(
                        message = "FCM push notification delivered",
                        user_id = %msg.user_id,
                        notification_id = %msg.notification_id,
                        "FCM gateway accepted delivery request"
                    );
                    Ok(())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    Err(anyhow::anyhow!("FCM push failed ({}): {}", status, body))
                }
            }
            Err(e) => Err(anyhow::anyhow!("FCM HTTP request failed: {:?}", e)),
        }
    }
}

/// Start the FCM consumer background task.
pub async fn start_fcm_consumer(
    connection: Option<Arc<Connection>>,
    db: sea_orm::DatabaseConnection,
) {
    let creds = match FcmCredentials::from_config() {
        Ok(c) => c,
        Err(e) => {
            error!(
                message = "FCM infrastructure initialization failed",
                error.message = %e,
                error.details = ?e,
                "Disabling background push consumer loop"
            );
            return;
        }
    };

    let _ = connection;
    let token_cache: Arc<Mutex<Option<CachedToken>>> = Arc::new(Mutex::new(None));
    let url = AppConfig::get().rabbitmq_url.clone();

    tokio::spawn(async move {
        loop {
            let fresh_url = crate::infrastructure::mq::ensure_heartbeat(&url);
            let conn = match Connection::connect(
                &fresh_url,
                ConnectionProperties::default().with_executor(TokioExecutor::current()),
            )
            .await
            {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    error!(
                        message = "RabbitMQ connection establishment failed for FCM consumer",
                        error.message = %e,
                        error.details = ?e,
                        "Retrying connection fallback sequence in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP channel creation failed for FCM consumer",
                        error.message = %e,
                        error.details = ?e,
                        "Retrying channel recovery loop in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Err(e) = setup_fcm_topology(&channel).await {
                error!(
                    message = "AMQP topology setup failed for FCM queue",
                    error.message = %e,
                    error.details = ?e,
                    "Retrying topology registration in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let mut consumer = match channel
                .basic_consume(
                    FCM_QUEUE,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        message = "AMQP consumer binding failed for FCM queue",
                        error.message = %e,
                        error.details = ?e,
                        queue = %FCM_QUEUE,
                        "Retrying queue consumption in 5 seconds"
                    );
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!(
                message = "FCM consumer stream activated",
                queue = %FCM_QUEUE,
                "Awaiting inbound message frames"
            );

            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload_str) = std::str::from_utf8(&delivery.data) {
                            match serde_json::from_str::<FcmMessage>(payload_str) {
                                Ok(msg) => match creds.send_push(&token_cache, &msg).await {
                                    Ok(()) => {
                                        if let Err(e) =
                                            mark_outbox_delivered(&db, msg.notification_id).await
                                        {
                                            error!(
                                                message = "Transactional outbox status synchronization failed",
                                                error.message = %e,
                                                error.details = ?e,
                                                notification_id = %msg.notification_id,
                                                "Acking delivery payload despite tracking update failure"
                                            );
                                        }
                                        let _ = delivery.ack(BasicAckOptions::default()).await;
                                    }
                                    Err(e) => {
                                        error!(
                                            message = "FCM delivery gateway transaction rejected",
                                            error.message = %e,
                                            error.details = ?e,
                                            notification_id = %msg.notification_id,
                                            user_id = %msg.user_id,
                                            "Sending negative acknowledgment (NACK) without requeue"
                                        );
                                        let _ = delivery
                                            .nack(BasicNackOptions {
                                                requeue: false,
                                                ..Default::default()
                                            })
                                            .await;
                                    }
                                },
                                Err(e) => {
                                    error!(
                                        message = "Inbound payload serialization failed",
                                        error.message = %e,
                                        error.details = ?e,
                                        "Rejecting corrupted queue frame"
                                    );
                                    let _ = delivery
                                        .nack(BasicNackOptions {
                                            requeue: false,
                                            ..Default::default()
                                        })
                                        .await;
                                }
                            }
                        } else {
                            error!(
                                message = "Inbound queue frame contains invalid UTF-8 data",
                                "Rejecting unreadable message payload"
                            );
                            let _ = delivery
                                .nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                })
                                .await;
                        }
                    }
                    Err(e) => {
                        error!(
                            message = "AMQP delivery stream connection severed",
                            error.message = %e,
                            error.details = ?e,
                            "Breaking trace stream consumer loop"
                        );
                        break;
                    }
                }
            }

            warn!(
                message = "FCM consumer transaction loop broken unexpectedly",
                "Initiating restart cooling delay for 5 seconds"
            );
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
