use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::users::{ProfileUpdateRequest, UserCreateRequest, UserStatusUpdateRequest},
        responses::users::{UserResponseData, UserListResponseData, MeResponseData},
    },
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

pub async fn get_me(_user: users::Model) -> Result<MeResponseData, AppError> {
    unimplemented!()
}

pub async fn update_me(
    _db: &DatabaseConnection,
    _user: users::Model,
    _req: ProfileUpdateRequest,
) -> Result<MeResponseData, AppError> {
    unimplemented!()
}

pub async fn close_account(
    _db: &DatabaseConnection,
    _user: users::Model,
) -> Result<(), AppError> {
    unimplemented!()
}

pub async fn list_users(
    _db: &DatabaseConnection,
    _user: users::Model,
    _query: UserListQuery,
) -> Result<UserListResponseData, AppError> {
    unimplemented!()
}

pub async fn get_user(
    _db: &DatabaseConnection,
    _user: users::Model,
    _id: Uuid,
) -> Result<UserResponseData, AppError> {
    unimplemented!()
}

pub async fn create_user(
    _db: &DatabaseConnection,
    _user: users::Model,
    _req: UserCreateRequest,
) -> Result<UserResponseData, AppError> {
    unimplemented!()
}

pub async fn update_user_status(
    _db: &DatabaseConnection,
    _user: users::Model,
    _id: Uuid,
    _req: UserStatusUpdateRequest,
) -> Result<(), AppError> {
    unimplemented!()
}

use utoipa::IntoParams;

#[derive(Debug, serde::Deserialize, IntoParams)]
pub struct UserListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
