use axum::{extract::Path, Json};
use uuid::Uuid;

use crate::core::errors::AppError;
use crate::model::{
    requests::notifications::{
        list_query::NotificationListQuery,
        update_preference_request::UpdateNotificationPreferenceRequest,
    },
    responses::{
        base::ApiResponse,
        notifications::{
            notification_category_response::NotificationCategoryResponse,
            notification_detail_response::NotificationDetailResponse,
            notification_list_response::NotificationListItem,
            preference_response::NotificationPreferenceResponse,
        },
    },
};

/// GET /api/v1/notifications/preferences
pub async fn get_preferences(
) -> Result<Json<ApiResponse<Vec<NotificationPreferenceResponse>>>, AppError> {
    unimplemented!()
}

/// PUT /api/v1/notifications/preferences
pub async fn update_preferences(
    Json(_body): Json<UpdateNotificationPreferenceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}

/// GET /api/v1/notifications
pub async fn list(
    axum::extract::Query(_query): axum::extract::Query<NotificationListQuery>,
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {
    unimplemented!()
}

/// GET /api/v1/notifications/{id}
pub async fn get_detail(
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<NotificationDetailResponse>>, AppError> {
    unimplemented!()
}

/// POST /api/v1/notifications/{id}/read
pub async fn mark_read(
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}

/// POST /api/v1/notifications/read-all
pub async fn mark_all_read(
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}

/// POST /api/v1/notifications/outbox/sync
pub async fn sync_outbox(
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {
    unimplemented!()
}

/// GET /api/v1/notifications/categories
pub async fn list_categories(
) -> Result<Json<ApiResponse<Vec<NotificationCategoryResponse>>>, AppError> {
    unimplemented!()
}
