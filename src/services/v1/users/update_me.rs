use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::users::ProfileUpdateRequest,
        responses::users::MeResponseData,
    },
};

/// Represents the calculated results and side-effects for updating the current user's profile.
#[derive(Debug)]
pub struct UpdateMeEffect {
    /// The updated database model ready for persistence.
    pub user_active_model: users::ActiveModel,
    /// The updated profile data to be returned in the API response.
    pub response_data: MeResponseData,
}

/// Validate and prepare the profile update.
///
/// Only the owner of the account can update their profile (enforced by AuthUser extraction).
/// Deleted accounts are rejected. Only provided fields are updated; omitted fields keep
/// their current value.
pub fn decide_update_me(user: users::Model, req: ProfileUpdateRequest) -> Result<UpdateMeEffect, AppError> {
    let user_id = user.id;
    // Reject deleted accounts
    if user.deleted_at.is_some() {
        tracing::warn!(
            user_id = %user_id,
            error.message = "AccountDeactivated",
            error.details = "",
            message = "Account is deactivated/soft-deleted"
        );
        return Err(AppError::Unauthorized("Account is deactivated".to_string()));
    }

    let new_name = req.full_name.unwrap_or_else(|| user.full_name.clone());
    let new_email = req.email.unwrap_or_else(|| user.email.clone());
    let new_phone = req.phone.unwrap_or_else(|| user.phone_number.clone());

    let now = chrono::Utc::now();

    let user_active_model = users::ActiveModel {
        id: sea_orm::Set(user.id),
        full_name: sea_orm::Set(new_name.clone()),
        email: sea_orm::Set(new_email.clone()),
        phone_number: sea_orm::Set(new_phone.clone()),
        updated_at: sea_orm::Set(now),
        ..Default::default()
    };

    let response_data = MeResponseData {
        id: user.id,
        role_id: user.role_id,
        full_name: new_name,
        email: new_email,
        phone: Some(new_phone),
        province: user.province,
        account_status_id: user.account_status,
        employee_id: crate::utils::user::get_employee_id(user.role_id, user.id),
        created_at: user.created_at.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };

    tracing::info!(
        user_id = %user_id,
        reason = "UpdateMeDecided",
        message = "Profile update successfully decided"
    );

    Ok(UpdateMeEffect { user_active_model, response_data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;
    use sea_orm::Set;

    #[fixture]
    fn mock_user() -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: 1,
            role_id: 3,
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
    #[case(Some("Jane Doe"), Some("jane@zent.com"), Some("+111"))]
    #[case(None, Some("jane@zent.com"), None)]
    #[case(Some("Jane Doe"), None, None)]
    fn test_decide_update_me_success(
        #[case] new_name: Option<&str>,
        #[case] new_email: Option<&str>,
        #[case] new_phone: Option<&str>,
        mock_user: users::Model,
    ) {
        let req = ProfileUpdateRequest {
            full_name: new_name.map(|s| s.to_string()),
            email: new_email.map(|s| s.to_string()),
            phone: new_phone.map(|s| s.to_string()),
        };
        
        let effect = decide_update_me(mock_user.clone(), req).unwrap();
        
        if let Some(name) = new_name {
            assert_eq!(effect.user_active_model.full_name, Set(name.to_string()));
            assert_eq!(effect.response_data.full_name, name);
        } else {
            assert_eq!(effect.user_active_model.full_name, Set(mock_user.full_name));
        }

        if let Some(email) = new_email {
            assert_eq!(effect.user_active_model.email, Set(email.to_string()));
            assert_eq!(effect.response_data.email, email);
        }

        if let Some(phone) = new_phone {
            assert_eq!(effect.user_active_model.phone_number, Set(phone.to_string()));
            assert_eq!(effect.response_data.phone, Some(phone.to_string()));
        }
    }

    #[rstest]
    fn test_decide_update_me_unauthorized_if_deleted() {
        let mut user = mock_user();
        user.deleted_at = Some(Utc::now());
        let req = ProfileUpdateRequest { full_name: None, email: None, phone: None };
        let res = decide_update_me(user, req);
        assert!(matches!(res, Err(AppError::Unauthorized(_))));
    }
}
