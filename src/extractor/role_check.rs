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
};

/// Middleware factory to require a specific role.
pub fn require_role<S>(role: Role) -> impl Fn(State<S>, Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone
where
    S: Send + Sync + 'static,
    DecodingKey: FromRef<S>,
    Arc<DatabaseConnection>: FromRef<S>,
    Arc<LookupTables>: FromRef<S>,
{
    move |State(state), mut req, next| {
        let role = role;
        Box::pin(async move {
            // 1. Run the AuthUser extractor manually
            let (mut parts, body) = req.into_parts();
            let auth_user = AuthUser::from_request_parts(&mut parts, &state).await?;
            
            // Reconstruct the request
            req = Request::from_parts(parts, body);
            
            // 2. Get LookupTables to find the ID for the required role
            let lookup_tables = Arc::<LookupTables>::from_ref(&state);
            let required_role_id = lookup_tables
                .roles_by_name
                .get(role.as_str())
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Role ID not found for role: {}", role.as_str())))?;

            let super_admin_role_id = lookup_tables
                .roles_by_name
                .get("SuperAdmin")
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("SuperAdmin role ID not found")))?;

            // 3. Check if the user has the required role (or is SuperAdmin)
            if auth_user.user.role_id != *required_role_id && auth_user.user.role_id != *super_admin_role_id {
                return Err(AppError::Forbidden("You do not have the required role to access this resource".to_string()));
            }

            // 4. Inject AuthUser into extensions so handlers don't have to re-extract it
            req.extensions_mut().insert(auth_user);

            Ok(next.run(req).await)
        })
    }
}
