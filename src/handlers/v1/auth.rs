use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use std::net::SocketAddr;
use validator::Validate;
use crate::{
    model::{
        requests::auth::{
            user_login_request::UserLoginRequest,
            user_registration_request::UserRegistrationRequest,
        },
        responses::auth::{login_response::LoginResponse, register_response::RegisterResponse},
        responses::error::AppError,
    },
    services::v1::auth::{login_service, register_service},
    state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
};
use sea_orm::DatabaseConnection;
use jsonwebtoken::EncodingKey;

pub async fn login_handler(
    State(db): State<DatabaseConnection>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(session_ttl): State<SessionDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UserLoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let result = login_service::perform_login(db, access_token_ttl, session_ttl, encoding_key, payload, addr).await?;
    Ok(Json(result))
}

pub async fn register_handler(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    if let Err(errors) = payload.validate() {
        // Collect errors into a message
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let result = register_service::perform_register(db, payload).await?;
    Ok(Json(result))
}
