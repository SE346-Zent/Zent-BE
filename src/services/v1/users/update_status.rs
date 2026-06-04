use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserStatusUpdateRequest,
};

/// Represents the side-effects for updating a user's status.
#[derive(Debug)]
pub struct UpdateStatusEffect {
    pub user_active_model: users::ActiveModel,
}

/// Validate and prepare status update.
///
/// RBAC rules:
/// - SuperAdmin: can update any Admin or Technician status.
/// - Admin: can only update Technician users in their own province.
/// - Others: forbidden.
pub fn decide_can_update_status(
    current_user: users::Model,
    target_user: users::Model,
    req: UserStatusUpdateRequest,
) -> Result<UpdateStatusEffect, AppError> {
    let current_role = current_user.role_id;

    match current_role {
        2 => {
            // SuperAdmin: can update Admin or Technician
            if target_user.role_id != 1 && target_user.role_id != 4 {
                return Err(AppError::Forbidden(
                    "SuperAdmin can only disable Admin or Technician accounts".to_string(),
                ));
            }
        }
        1 => {
            // Admin: must be in the same province and can only target Technician
            if target_user.role_id != 4 {
                return Err(AppError::Forbidden(
                    "Admin can only disable Technician accounts".to_string(),
                ));
            }
            let Some(ref admin_province) = current_user.province else {
                return Err(AppError::Forbidden(
                    "Admin profile missing province assignment".to_string(),
                ));
            };
            let Some(ref target_province) = target_user.province else {
                return Err(AppError::Forbidden(
                    "You can only update users in your province".to_string(),
                ));
            };
            if admin_province != target_province {
                return Err(AppError::Forbidden(
                    "You can only update users in your province".to_string(),
                ));
            }
        }
        _ => {
            return Err(AppError::Forbidden(
                "Only administrators can update user status".to_string(),
            ));
        }
    }

    let mut user_active_model: users::ActiveModel = target_user.into();
    user_active_model.account_status = sea_orm::Set(req.account_status_id);
    user_active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    Ok(UpdateStatusEffect { user_active_model })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;
    use sea_orm::Set;

    const ROLE_ADMIN: i32 = 1;
    const ROLE_SUPER_ADMIN: i32 = 2;
    const ROLE_CUSTOMER: i32 = 3;
    const ROLE_TECHNICIAN: i32 = 4;

    #[fixture]
    fn mock_user(#[default(ROLE_ADMIN)] role_id: i32) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: 1,
            role_id,
            province: Some("Ontario".to_string()),
            avatar_url: None,
            recovery_email: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(ROLE_SUPER_ADMIN, ROLE_ADMIN, 4, "ok")]       // SA can disable Admin
    #[case(ROLE_SUPER_ADMIN, ROLE_TECHNICIAN, 4, "ok")]   // SA can disable Technician
    #[case(ROLE_ADMIN, ROLE_TECHNICIAN, 4, "ok")]         // Admin can disable Technician (same province)
    #[case(ROLE_ADMIN, ROLE_ADMIN, 4, "forbidden")]       // Admin cannot disable Admin
    #[case(ROLE_ADMIN, ROLE_CUSTOMER, 4, "forbidden")]    // Admin cannot disable Customer
    #[case(ROLE_SUPER_ADMIN, ROLE_CUSTOMER, 4, "forbidden")] // SA cannot disable Customer
    #[case(ROLE_TECHNICIAN, ROLE_TECHNICIAN, 4, "forbidden")] // Technician cannot update anyone
    fn test_decide_can_update_status_rbac(#[case] role_id: i32, #[case] target_role_id: i32, #[case] target_status: i32, #[case] expected: &str) {
        let current_user = mock_user(role_id);
        let mut target_user = mock_user(target_role_id);
        target_user.province = Some("Ontario".to_string()); // same province
        let req = UserStatusUpdateRequest { account_status_id: target_status };
        let res = decide_can_update_status(current_user, target_user, req);

        match expected {
            "ok" => {
                let effect = res.expect("Should be OK");
                assert_eq!(effect.user_active_model.account_status, Set(target_status));
            },
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }

    #[rstest]
    fn test_admin_different_province_forbidden() {
        let current_user = mock_user(ROLE_ADMIN);
        let mut target_user = mock_user(ROLE_TECHNICIAN);
        target_user.province = Some("Quebec".to_string()); // different province
        let req = UserStatusUpdateRequest { account_status_id: 4 };
        let res = decide_can_update_status(current_user, target_user, req);
        assert!(matches!(res, Err(AppError::Forbidden(_))));
    }
}
