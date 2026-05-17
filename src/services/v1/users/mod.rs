use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::{ProfileUpdateRequest, UserCreateRequest, UserStatusUpdateRequest},
};
use uuid::Uuid;
use sea_orm::DatabaseConnection;

pub mod get_me;
pub mod update_me;
pub mod close_account;
pub mod list_users;
pub mod get_user;
pub mod create_user;
pub mod update_status;

/// Orchestration for user-related service entry points.

pub async fn get_me(user: users::Model) -> Result<get_me::GetMeEffect, AppError> {
    let _ = user;
    unimplemented!()
}

pub async fn update_me(
    _db: &DatabaseConnection,
    _user: users::Model,
    _req: ProfileUpdateRequest,
) -> Result<update_me::UpdateMeEffect, AppError> {
    unimplemented!()
}

pub async fn close_account(
    _db: &DatabaseConnection,
    _user: users::Model,
) -> Result<close_account::CloseAccountEffect, AppError> {
    unimplemented!()
}

pub async fn list_users(
    _db: &DatabaseConnection,
    _current_user: users::Model,
    _query: UserListQuery,
) -> Result<list_users::ListUsersEffect, AppError> {
    unimplemented!()
}

pub async fn get_user(
    _db: &DatabaseConnection,
    _current_user: users::Model,
    _id: Uuid,
) -> Result<get_user::GetUserEffect, AppError> {
    unimplemented!()
}

pub async fn create_user(
    _db: &DatabaseConnection,
    _current_user: users::Model,
    _req: UserCreateRequest,
) -> Result<create_user::CreateUserEffect, AppError> {
    unimplemented!()
}

pub async fn update_user_status(
    _db: &DatabaseConnection,
    _current_user: users::Model,
    _id: Uuid,
    _req: UserStatusUpdateRequest,
) -> Result<update_status::UpdateStatusEffect, AppError> {
    unimplemented!()
}

use utoipa::IntoParams;

#[derive(Debug, serde::Deserialize, IntoParams)]
pub struct UserListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
