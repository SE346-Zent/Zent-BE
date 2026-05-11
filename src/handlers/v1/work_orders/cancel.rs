use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/cancel",
    responses(
        (status = 200, description = "Work order cancelled"),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Not Found"), (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel(
    Extension(_auth): Extension<AuthUser>,
    State(_db): State<Arc<DatabaseConnection>>,
    State(_luts): State<Arc<LookupTables>>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}
