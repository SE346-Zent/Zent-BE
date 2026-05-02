use std::collections::HashMap;
use std::sync::Arc;
use jsonwebtoken::EncodingKey;

use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    model::{
        requests::auth::{
            user_login_request::UserLoginRequest,
            user_registration_request::UserRegistrationRequest,
            verify_otp_request::VerifyOtpRequest,
            resend_otp_request::ResendOtpRequest,
            forgot_password_request::ForgotPasswordRequest,
            verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
            reset_password_request::ResetPasswordRequest,
            refresh_token_request::RefreshTokenRequest,
        },
        responses::{
            base::ApiResponse,
            auth::login_response::LoginResponseData,
            auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
        },
    },
    infrastructure::cache::ValkeyClient,
};
use sea_orm::DatabaseConnection;
use lapin::Connection;

// Internal logic modules
mod login;
mod register;
mod verify_otp;
mod resend_otp;
mod refresh_token;
mod forgot_password;
mod verify_forgot_password_otp;
mod reset_password;

pub struct AuthService {
    db: DatabaseConnection,
    valkey: Option<Arc<ValkeyClient>>,
    rabbitmq: Option<Arc<Connection>>,
    templates: Arc<HashMap<String, String>>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
}

impl AuthService {
    pub fn new(
        db: DatabaseConnection,
        valkey: Option<Arc<ValkeyClient>>,
        rabbitmq: Option<Arc<Connection>>,
        templates: Arc<HashMap<String, String>>,
        access_token_ttl: AccessTokenDefaultTTLSeconds,
        session_ttl: SessionDefaultTTLSeconds,
        encoding_key: EncodingKey,
    ) -> Self {
        Self {
            db,
            valkey,
            rabbitmq,
            templates,
            access_token_ttl,
            session_ttl,
            encoding_key,
        }
    }

    #[tracing::instrument(skip(self, req, ip_address), fields(user_email = %req.email))]
    pub async fn login(&self, req: UserLoginRequest, ip_address: String) -> Result<ApiResponse<LoginResponseData>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        login::handle_login(db, valkey, self.access_token_ttl, self.session_ttl, self.encoding_key.clone(), req, ip_address).await
    }

    #[tracing::instrument(skip(self, req), fields(user_email = %req.email))]
    pub async fn register(&self, req: UserRegistrationRequest) -> Result<ApiResponse<()>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        let rmq = self.rabbitmq.clone();
        register::handle_register(db, valkey, rmq, &self.templates, req).await
    }

    #[tracing::instrument(skip(self, req), fields(user_email = %req.email))]
    pub async fn verify_otp(&self, req: VerifyOtpRequest) -> Result<ApiResponse<()>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        let rmq = self.rabbitmq.clone();
        let script_hashes = self.valkey.as_ref().map(|v| v.get_script_hashes()).unwrap_or_default();
        verify_otp::handle_verify_otp(db, valkey, rmq, &self.templates, &script_hashes, req).await
    }

    #[tracing::instrument(skip(self, req), fields(user_email = %req.email))]
    pub async fn resend_otp(&self, req: ResendOtpRequest) -> Result<ApiResponse<()>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        let rmq = self.rabbitmq.clone();
        resend_otp::handle_resend_otp(db, valkey, rmq, &self.templates, req).await
    }

    #[tracing::instrument(skip(self, req))]
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<ApiResponse<LoginResponseData>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        refresh_token::handle_refresh_token(db, valkey, self.access_token_ttl, self.encoding_key.clone(), req).await
    }

    #[tracing::instrument(skip(self, req), fields(user_email = %req.email))]
    pub async fn forgot_password(&self, req: ForgotPasswordRequest) -> Result<ApiResponse<()>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        let rmq = self.rabbitmq.clone();
        forgot_password::handle_forgot_password(db, valkey, rmq, &self.templates, req).await
    }

    #[tracing::instrument(skip(self, req), fields(user_email = %req.email))]
    pub async fn verify_forgot_password_otp(&self, req: VerifyForgotPasswordOtpRequest) -> Result<ApiResponse<VerifyForgotPasswordOtpResponseData>, AppError> {
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        let script_hashes = self.valkey.as_ref().map(|v| v.get_script_hashes()).unwrap_or_default();
        verify_forgot_password_otp::handle_verify_forgot_password_otp(valkey, &script_hashes, req).await
    }

    #[tracing::instrument(skip(self, req))]
    pub async fn reset_password(&self, req: ResetPasswordRequest) -> Result<ApiResponse<()>, AppError> {
        let db = self.db.clone();
        let valkey = self.valkey.as_ref().map(|v| v.get_connection());
        reset_password::handle_reset_password(db, valkey, req).await
    }
}
