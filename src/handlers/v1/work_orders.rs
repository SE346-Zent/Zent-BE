use axum::{extract::State, http::StatusCode};
use crate::core::state::AppState;

pub async fn create(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn list(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn get_details(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn assign(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn schedule(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn start(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn refuse(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn cancel(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn complete(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn history(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn add_parts(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn approve_refusal(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn deny_refusal(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
