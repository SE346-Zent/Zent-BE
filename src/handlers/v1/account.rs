use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;

use crate::{
    model::auth::jwt_claims::Claims,
    model::{
        responses::account::profile_response::{ProfileListQuery, ProfileListResponse, ProfileResponse},
        responses::error::AppError,
    },
    services::v1::account::profile_service::{get_profile_service, get_profiles_service},
};

pub async fn get_profile(
    State(db): State<DatabaseConnection>,
    claims: Claims,
) -> Result<Json<ProfileResponse>, AppError> {
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid token subject".to_string()))?;
    
    let result = get_profile_service(db, user_id).await?;
    Ok(Json(result))
}

pub async fn get_profiles(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ProfileListQuery>,
    _claims: Claims,
) -> Result<Json<ProfileListResponse>, AppError> {
    let result = get_profiles_service(db, query).await?;
    Ok(Json(result))
}
