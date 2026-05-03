use axum::{extract::State, http::StatusCode};
use std::sync::Arc;
use crate::services::v1::core::media::MediaService;

pub async fn upload_work_order_photo(State(_service): State<Arc<MediaService>>) -> StatusCode { unimplemented!() }
pub async fn get_work_order_photo(State(_service): State<Arc<MediaService>>) -> StatusCode { unimplemented!() }
pub async fn list_work_order_photos(State(_service): State<Arc<MediaService>>) -> StatusCode { unimplemented!() }
pub async fn upload_work_order_signature(State(_service): State<Arc<MediaService>>) -> StatusCode { unimplemented!() }
