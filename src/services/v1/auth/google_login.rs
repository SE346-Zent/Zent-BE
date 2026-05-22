use crate::{
    core::{
        config::AppConfig,
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::users,
    model::responses::auth::login_response::{AccountStatusEnum, LoginResponseData, UserInfo},
    services::v1::core::token_service,
};
use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::{decode, decode_header, DecodingKey, EncodingKey, Validation, Algorithm};
use serde::Deserialize;
use base64::Engine;

#[derive(Debug, Deserialize, Clone)]
pub struct FirebaseClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub email_verified: Option<bool>,
}

/// Retrieve the Firebase project ID from env or google application credentials.
pub fn get_firebase_project_id() -> Option<String> {
    if let Ok(id) = std::env::var("FIREBASE_PROJECT_ID") {
        if !id.trim().is_empty() {
            return Some(id.trim().to_string());
        }
    }
    if let Some(path) = AppConfig::get().google_application_credentials.as_deref() {
        if !path.is_empty() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(project_id) = json["project_id"].as_str() {
                        return Some(project_id.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Decode and verify Google/Firebase ID tokens.
pub async fn verify_google_or_firebase_token(token: &str, project_id: &str) -> Result<FirebaseClaims, AppError> {
    let header = decode_header(token)
        .map_err(|e| AppError::Unauthorized(format!("Invalid token header: {}", e)))?;
    let kid = header.kid.ok_or_else(|| AppError::Unauthorized("Missing kid in token header".to_string()))?;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::Unauthorized("Invalid JWT format".to_string()));
    }
    
    let payload_decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])
        .map_err(|_| AppError::Unauthorized("Invalid token payload base64".to_string()))?;
    
    #[derive(Deserialize)]
    struct TempClaims {
        iss: String,
    }
    let temp: TempClaims = serde_json::from_slice(&payload_decoded)
        .map_err(|_| AppError::Unauthorized("Invalid claims JSON".to_string()))?;

    let is_firebase = temp.iss.starts_with("https://securetoken.google.com/");
    let is_google = temp.iss == "https://accounts.google.com" || temp.iss == "accounts.google.com";

    if !is_firebase && !is_google {
        return Err(AppError::Unauthorized("Unsupported issuer".to_string()));
    }

    let certs_url = if is_firebase {
        "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com"
    } else {
        "https://www.googleapis.com/oauth2/v1/certs"
    };

    let client = reqwest::Client::new();
    let certs: serde_json::Value = client.get(certs_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to fetch public certificates: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse public certificates: {}", e)))?;

    let cert_pem = certs.get(&kid)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Unauthorized("Public key certificate not found for kid".to_string()))?;

    let decoding_key = DecodingKey::from_rsa_pem(cert_pem.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse public key PEM: {}", e)))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 60;

    if is_firebase {
        if !project_id.is_empty() {
            validation.set_issuer(&[format!("https://securetoken.google.com/{}", project_id)]);
            validation.set_audience(&[project_id]);
        } else {
            validation.validate_aud = false;
        }
    } else {
        validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
        validation.validate_aud = false;
    }

    let token_data = decode::<FirebaseClaims>(token, &decoding_key, &validation)
        .map_err(|e| AppError::Unauthorized(format!("Token validation failed: {}", e)))?;

    if is_firebase && project_id.is_empty() {
        if !token_data.claims.iss.starts_with("https://securetoken.google.com/") {
            return Err(AppError::Unauthorized("Invalid issuer for Firebase token".to_string()));
        }
    }

    Ok(token_data.claims)
}


/// Represents the calculated results and side-effects of a successful Google/Firebase login attempt.
#[derive(Debug)]
pub struct GoogleLoginEffect {
    /// Unique identifier for the newly created or active session.
    pub session_id: Uuid,
    /// The unique identifier of the user.
    pub user_id: Uuid,
    /// A cryptographic hash of the refresh token.
    pub refresh_token_hash: String,
    /// The timestamp when this session will expire.
    pub session_expires_at: chrono::DateTime<Utc>,
    /// The updated or new user's ActiveModel if they need to be persisted/registered.
    pub user_active_model: Option<users::ActiveModel>,
    /// The response payload.
    pub response_data: LoginResponseData,
}

/// Pure business logic for deciding the outcome of a Google/Firebase login.
///
/// If the user exists:
/// - Checks if they are deleted or locked.
/// - If they are pending, auto-activates them (since Google verified the email).
/// - Generates login session and tokens.
/// If the user does not exist:
/// - Auto-registers them as a Customer with Active status.
/// - Generates login session and tokens.
pub fn decide_google_login(
    existing_user: Option<&users::Model>,
    email: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
    active_status_id: i32,
    customer_role_id: i32,
    hashed_placeholder_password: String,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<GoogleLoginEffect, AppError> {
    let user_id;
    let mut user_active_model = None;
    let final_name;
    let final_status;
    let final_phone;
    let final_role;

    if let Some(user_record) = existing_user {
        // 1. Check if user is deleted
        if user_record.deleted_at.is_some() {
            return Err(AppError::Unauthorized("Account is deactivated".to_string()));
        }

        // 2. Google login is restricted only to Customer accounts
        if user_record.role_id != customer_role_id {
            return Err(AppError::Forbidden("Only customer accounts are allowed to authenticate via Google".to_string()));
        }

        // 3. Verify account status
        let account_status = AccountStatusEnum::from(user_record.account_status);

        match account_status {
            AccountStatusEnum::Active => {
                final_status = AccountStatusEnum::Active;
            }
            AccountStatusEnum::Pending => {
                // Auto-activate pending users who log in via Google
                let mut active: users::ActiveModel = user_record.clone().into();
                active.account_status = Set(active_status_id);
                active.updated_at = Set(Utc::now());
                user_active_model = Some(active);
                final_status = AccountStatusEnum::Active;
            }
            _ => {
                return Err(AppError::Forbidden(format!("Account is {:?}", account_status)));
            }
        }

        user_id = user_record.id;
        final_name = user_record.full_name.clone();
        final_phone = user_record.phone_number.clone();
        final_role = user_record.role_id;
    } else {
        // Auto-register a new Customer
        user_id = Uuid::new_v4();
        let name = full_name.unwrap_or_else(|| {
            email.split('@').next().unwrap_or("Google User").to_string()
        });

        let now = Utc::now();
        let active_model = users::ActiveModel {
            id: Set(user_id),
            account_status: Set(active_status_id),
            role_id: Set(customer_role_id),
            email: Set(email.clone()),
            full_name: Set(name.clone()),
            password_hash: Set(hashed_placeholder_password),
            phone_number: Set("".to_string()),
            province: Set(None),
            fcm_token: Set(None),
            installation_id: Set(None),
            avatar_url: Set(avatar_url),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        };

        user_active_model = Some(active_model);
        final_name = name;
        final_status = AccountStatusEnum::Active;
        final_phone = "".to_string();
        final_role = customer_role_id;
    }

    // Generate token bundle
    let token_bundle = token_service::generate_token_bundle(
        &user_id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    let session_id = Uuid::new_v4();
    let session_expires_at = Utc::now() + chrono::Duration::seconds(session_ttl.0);

    Ok(GoogleLoginEffect {
        session_id,
        user_id,
        refresh_token_hash: token_bundle.refresh_token_hash,
        session_expires_at,
        user_active_model,
        response_data: LoginResponseData {
            user: UserInfo {
                full_name: final_name,
                account_status: final_status,
                email,
                phone_number: final_phone,
                role_id: final_role,
            },
            access_token: token_bundle.access_token,
            refresh_token: token_bundle.refresh_token,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use chrono::Utc;

    #[fixture]
    fn mock_key() -> EncodingKey {
        EncodingKey::from_secret(b"secret")
    }

    #[fixture]
    fn mock_existing_user(
        #[default(1)] status: i32,
        #[default(false)] deleted: bool,
    ) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "Existing User".to_string(),
            email: "exist@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+123456789".to_string(),
            account_status: status,
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: if deleted { Some(Utc::now()) } else { None },
        }
    }

    #[rstest]
    fn test_decide_google_login_existing_active(
        mock_existing_user: users::Model,
        mock_key: EncodingKey,
    ) {
        let effect = decide_google_login(
            Some(&mock_existing_user),
            "exist@example.com".to_string(),
            None,
            None,
            1, // Active status ID
            1, // Customer role ID
            "hash".to_string(),
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        )
        .unwrap();

        assert_eq!(effect.user_id, mock_existing_user.id);
        assert!(effect.user_active_model.is_none());
        assert_eq!(effect.response_data.user.full_name, "Existing User");
        assert_eq!(effect.response_data.user.account_status, AccountStatusEnum::Active);
    }

    #[rstest]
    fn test_decide_google_login_existing_pending(
        mock_key: EncodingKey,
    ) {
        // Pending is status ID 2
        let pending_user = users::Model {
            account_status: 2,
            ..mock_existing_user(2, false)
        };

        let effect = decide_google_login(
            Some(&pending_user),
            "exist@example.com".to_string(),
            None,
            None,
            1, // Active status ID
            1, // Customer role ID
            "hash".to_string(),
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        )
        .unwrap();

        assert_eq!(effect.user_id, pending_user.id);
        assert!(effect.user_active_model.is_some());
        assert_eq!(effect.response_data.user.account_status, AccountStatusEnum::Active);
    }

    #[rstest]
    fn test_decide_google_login_existing_deleted(
        mock_key: EncodingKey,
    ) {
        let deleted_user = mock_existing_user(1, true);

        let res = decide_google_login(
            Some(&deleted_user),
            "exist@example.com".to_string(),
            None,
            None,
            1,
            1,
            "hash".to_string(),
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        );

        assert!(matches!(res, Err(AppError::Unauthorized(_))));
    }

    #[rstest]
    fn test_decide_google_login_existing_wrong_role(
        mock_existing_user: users::Model,
        mock_key: EncodingKey,
    ) {
        let res = decide_google_login(
            Some(&mock_existing_user),
            "exist@example.com".to_string(),
            None,
            None,
            1,
            3, // Different from user's role_id (1)
            "hash".to_string(),
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        );

        assert!(matches!(res, Err(AppError::Forbidden(_))));
    }

    #[rstest]
    fn test_decide_google_login_new_user(

        mock_key: EncodingKey,
    ) {
        let effect = decide_google_login(
            None,
            "new@example.com".to_string(),
            Some("New User".to_string()),
            Some("http://pic.jpg".to_string()),
            1, // Active status ID
            3, // Customer role ID
            "placeholder_hash".to_string(),
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        )
        .unwrap();

        assert!(effect.user_active_model.is_some());
        let active = effect.user_active_model.unwrap();
        assert_eq!(active.email, Set("new@example.com".to_string()));
        assert_eq!(active.full_name, Set("New User".to_string()));
        assert_eq!(active.avatar_url, Set(Some("http://pic.jpg".to_string())));
        assert_eq!(effect.response_data.user.email, "new@example.com");
    }
}
