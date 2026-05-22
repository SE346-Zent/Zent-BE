use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::responses::base::ApiResponse,
    model::responses::users::UserResponseData,
    services::v1::users::get_user,
    infrastructure::cache::ValkeyClient,
};
use redis::AsyncCommands;

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "users",
    responses(
        (status = 200, description = "Retrieve user successful", body = ApiResponse<UserResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn get_user_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    let target_user = users::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let effect = get_user::decide_get_user(current_user, target_user.clone())?;
    let mut response_data = effect.response_data;

    if response_data.role_id == 4 { // Technician
        let mut counts = std::collections::HashMap::new();
        for i in 1..=5 {
            counts.insert(i.to_string(), 0i64);
        }

        let mut fetched_from_cache = false;
        let cache_key = format!("ratings:tech:{}", target_user.id);

        if let Some(client) = valkey_client.as_ref() {
            if let Ok(mut conn) = client.get_connection().await {
                let exists: bool = conn.exists(&cache_key).await.unwrap_or(false);
                if exists {
                    if let Ok(cached_map) = conn.hgetall::<_, std::collections::HashMap<String, i64>>(&cache_key).await {
                        if !cached_map.is_empty() {
                            for (k, v) in cached_map {
                                counts.insert(k, v);
                            }
                            fetched_from_cache = true;
                        }
                    }
                }
            }
        }

        if !fetched_from_cache {
            // Fetch from DB
            use sea_orm::{QuerySelect, ColumnTrait, QueryFilter};
            let ratings_list = crate::entities::work_order_ratings::Entity::find()
                .inner_join(crate::entities::work_orders::Entity)
                .filter(crate::entities::work_orders::Column::TechnicianId.eq(target_user.id))
                .all(db.as_ref())
                .await?;

            for r in ratings_list {
                let score_str = r.rating.to_string();
                *counts.entry(score_str).or_insert(0) += 1;
            }

            // Cache in Valkey
            if let Some(client) = valkey_client.as_ref() {
                if let Ok(mut conn) = client.get_connection().await {
                    for (k, v) in &counts {
                        let _: Result<(), redis::RedisError> = conn.hset(&cache_key, k, v).await;
                    }
                    let _: Result<(), redis::RedisError> = conn.expire(&cache_key, 3600).await;
                }
            }
        }

        response_data.rating_counts = Some(counts);
    }

    Ok(Json(ApiResponse::success(200, "Retrieve user successful", response_data)))
}
