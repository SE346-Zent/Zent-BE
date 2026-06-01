use axum::{
    extract::{FromRef, FromRequestParts, Request, State},
    middleware::Next,
    response::Response,
};
use sea_orm::DatabaseConnection;
use jsonwebtoken::DecodingKey;
use std::sync::Arc;

use crate::{
    core::lookup_tables::LookupTables,
    entities::roles::Role,
    extractor::auth_user::AuthUser,
    core::errors::AppError,
    infrastructure::cache::ValkeyClient,
};

/// Middleware factory to require one of several roles.
/// Pass a slice of allowed roles; the middleware passes if the user holds any of them (or is SuperAdmin).
pub fn require_role<S>(roles: &'static [Role]) -> impl Fn(State<S>, Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone
where
    S: Send + Sync + 'static,
    DecodingKey: FromRef<S>,
    Arc<DatabaseConnection>: FromRef<S>,
    Arc<LookupTables>: FromRef<S>,
    Option<Arc<ValkeyClient>>: FromRef<S>,
{
    move |State(state), mut req, next| {
        let roles = roles;
        Box::pin(async move {
            // 1. Run the AuthUser extractor manually
            let (mut parts, body) = req.into_parts();
            let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;
            
            // Reconstruct the request
            req = Request::from_parts(parts, body);
            
            // 2. Get LookupTables
            let lookup_tables = Arc::<LookupTables>::from_ref(&state);

            let super_admin_role_id = lookup_tables
                .roles_by_name
                .get("SuperAdmin")
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("SuperAdmin role ID not found")))?;

            // 3. Check if the user has SuperAdmin or any of the allowed roles
            if auth_user.user.role_id == *super_admin_role_id {
                req.extensions_mut().insert(auth_user);
                return Ok(next.run(req).await);
            }

            for role in roles {
                if let Some(role_id) = lookup_tables.roles_by_name.get(role.as_str()) {
                    if auth_user.user.role_id == *role_id {
                        req.extensions_mut().insert(auth_user);
                        return Ok(next.run(req).await);
                    }
                }
            }

            // Resolve the user's actual role name for a clearer error message
            let user_role_name = lookup_tables
                .roles_by_name
                .iter()
                .find(|(_, &id)| id == auth_user.user.role_id)
                .map(|(name, _)| name.as_str())
                .unwrap_or("Unknown");

            Err(AppError::Forbidden(
                "You do not have permission to access this resource".to_string(),
            ))
        })
    }
}
