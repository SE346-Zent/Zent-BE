use axum::{extract::State, http::StatusCode};
use crate::core::state::AppState;

pub async fn upload_work_order_photo(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn get_work_order_photo(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn list_work_order_photos(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn upload_work_order_signature(State(_state): State<AppState>) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
