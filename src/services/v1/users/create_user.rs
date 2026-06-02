use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserCreateRequest,
};

/// Represents the side-effects for creating a new user.
#[derive(Debug)]
pub struct CreateUserEffect {
    pub user_active_model: users::ActiveModel,
    pub plain_password: Option<String>,
}

/// Validate and prepare user creation.
///
/// RBAC rules (role IDs: 1=Admin, 2=SuperAdmin, 3=Customer, 4=Technician):
/// - SuperAdmin (2): can create Admin (1), and Technician (4).
/// - Admin (1): can create Technician (4).
/// - Others: forbidden.
///
/// Province handling:
/// - SuperAdmin: uses the province from the request (must be provided for Admin, optional for Technician).
/// - Admin: province is forced to the admin's own province.
pub fn decide_can_create_user(current_user: users::Model, req: UserCreateRequest) -> Result<CreateUserEffect, AppError> {
    let current_user_id = current_user.id;
    let current_role = current_user.role_id;
    let target_role = req.role_id;

    // Determine allowed target roles based on current role
    let allowed = match current_role {
        2 => target_role == 1 || target_role == 2 || target_role == 4, // SA → SA, Admin, Tech
        1 => target_role == 4 || target_role == 3,                     // Admin → Tech, Customer
        _ => false,
    };

    if !allowed {
        tracing::warn!(
            current_user_id = %current_user_id,
            current_role = %current_role,
            target_role = %target_role,
            error.message = "CreateUserRoleForbidden",
            error.details = "",
            message = "You are not authorized to create a user with this role"
        );
        return Err(AppError::Forbidden(
            "You are not authorized to create a user with this role".to_string(),
        ));
    }

    // Determine province
    let province = match current_role {
        2 => {
            // SuperAdmin uses the province from the request
            req.province.clone()
        }
        1 => {
            // Admin's province is forced; use their own province
            current_user.province.clone()
        }
        _ => None,
    };

    let now = chrono::Utc::now();

    tracing::info!(
        current_user_id = %current_user_id,
        target_role = %target_role,
        email = %req.email,
        reason = "CreateUserDecided",
        message = "User creation successfully decided"
    );

    let user_active_model = users::ActiveModel {
        id: sea_orm::Set(uuid::Uuid::new_v4()),
        full_name: sea_orm::Set(req.full_name),
        email: sea_orm::Set(req.email),
        phone_number: sea_orm::Set(req.phone.unwrap_or_default()),
        role_id: sea_orm::Set(target_role),
        province: sea_orm::Set(province),
        account_status: sea_orm::Set(1), // Active by default when created by admin
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        ..Default::default()
    };

    // Password: always auto-generated — first 6 chars of a fresh UUID
    let raw_uuid = uuid::Uuid::new_v4().to_string();
    let plain_password: String = raw_uuid.chars().take(6).collect();

    Ok(CreateUserEffect {
        user_active_model,
        plain_password: Some(plain_password),
    })
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
    fn mock_user(#[default(3)] role_id: i32) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: 1,
            role_id,
            province: None,
            avatar_url: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    // SA can create everyone
    #[case(ROLE_SUPER_ADMIN, ROLE_SUPER_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_TECHNICIAN, "ok")]
    // Admin can create Tech and Customer
    #[case(ROLE_ADMIN, ROLE_TECHNICIAN, "ok")]
    #[case(ROLE_ADMIN, ROLE_CUSTOMER, "ok")]
    // Admin CANNOT create Admin or SA
    #[case(ROLE_ADMIN, ROLE_ADMIN, "forbidden")]
    #[case(ROLE_ADMIN, ROLE_SUPER_ADMIN, "forbidden")]
    // Others can't create anyone
    #[case(ROLE_TECHNICIAN, ROLE_CUSTOMER, "forbidden")]
    fn test_decide_can_create_user_rbac(#[case] current_role: i32, #[case] target_role: i32, #[case] expected: &str) {
        let current_user = mock_user(current_role);
        let req = UserCreateRequest {
            role_id: target_role,
            full_name: "New".to_string(),
            email: "new@zent.com".to_string(),
            phone: None,
            password: None,
            generate_password: Some(true),
            province: None,
        };
        let res = decide_can_create_user(current_user, req);
        
        match expected {
            "ok" => {
                let effect = res.expect("Should be OK");
                assert_eq!(effect.user_active_model.role_id, Set(target_role));
            },
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }
}
