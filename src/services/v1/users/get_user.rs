use crate::{
    core::errors::AppError,
    entities::users,
    model::responses::users::UserResponseData,
};

/// Represents the result for a single user detail request.
#[derive(Debug)]
pub struct GetUserEffect {
    pub response_data: UserResponseData,
}

/// Validate and prepare user detail retrieval.
///
/// RBAC rules:
/// - SuperAdmin: can view any user.
/// - Admin: can only view users in their own province.
/// - Others: forbidden.
///
/// Deleted users are treated as Not Found.
pub fn decide_get_user(
    current_user: users::Model,
    target_user: users::Model,
) -> Result<GetUserEffect, AppError> {
    // Reject deleted users
    if target_user.deleted_at.is_some() {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    let current_role = current_user.role_id;

    match current_role {
        2 => {
            // SuperAdmin: can view anyone
        }
        1 => {
            // Admin: must be in the same province (fail-closed if admin has no province)
            let Some(ref admin_province) = current_user.province else {
                return Err(AppError::Forbidden(
                    "Admin profile missing province assignment".to_string(),
                ));
            };
            let Some(ref target_province) = target_user.province else {
                return Err(AppError::Forbidden(
                    "You can only view users in your province".to_string(),
                ));
            };
            if admin_province != target_province {
                return Err(AppError::Forbidden(
                    "You can only view users in your province".to_string(),
                ));
            }
        }
        _ => {
            return Err(AppError::Forbidden(
                "Only administrators can view user details".to_string(),
            ));
        }
    }

    let response_data = UserResponseData {
        id: target_user.id,
        role_id: target_user.role_id,
        full_name: target_user.full_name,
        email: target_user.email,
        phone: Some(target_user.phone_number),
        province: target_user.province,
        account_status_id: target_user.account_status,
        employee_id: crate::utils::user::get_employee_id(target_user.role_id, target_user.id),
        rating_counts: None,
        created_at: target_user.created_at.to_rfc3339(),
        updated_at: target_user.updated_at.to_rfc3339(),
    };

    Ok(GetUserEffect { response_data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    const ROLE_ADMIN: i32 = 1;
    const ROLE_SUPER_ADMIN: i32 = 2;
    const ROLE_TECHNICIAN: i32 = 4;

    #[fixture]
    fn mock_user(#[default(4)] role_id: i32) -> users::Model {
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
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(ROLE_SUPER_ADMIN, true)]
    #[case(ROLE_ADMIN, true)]
    #[case(ROLE_TECHNICIAN, false)]
    fn test_decide_get_user_rbac(#[case] role_id: i32, #[case] expected_ok: bool) {
        let current_user = mock_user(role_id);
        let target_user = mock_user(ROLE_TECHNICIAN);
        let res = decide_get_user(current_user, target_user.clone());
        
        if expected_ok {
            let effect = res.expect("Should be OK");
            assert_eq!(effect.response_data.id, target_user.id);
            assert_eq!(effect.response_data.full_name, target_user.full_name);
        } else {
            assert!(matches!(res, Err(AppError::Forbidden(_))));
        }
    }

    #[rstest]
    fn test_decide_get_user_target_deleted() {
        let admin = mock_user(ROLE_ADMIN);
        let mut target = mock_user(ROLE_TECHNICIAN);
        target.deleted_at = Some(Utc::now());
        let res = decide_get_user(admin, target);
        assert!(matches!(res, Err(AppError::NotFound(_))));
    }
}
