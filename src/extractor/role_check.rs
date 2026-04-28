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
    extractor::jwt_claims::AuthError,
};

/// Middleware factory to require a specific role.
pub fn require_role<S>(role: Role) -> impl Fn(State<S>, Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AuthError>> + Send>> + Clone
where
    S: Send + Sync + 'static,
    DecodingKey: FromRef<S>,
    DatabaseConnection: FromRef<S>,
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
                .ok_or(AuthError::InternalServerError)?;

            // 3. Check if the user has the required role
            if auth_user.user.role_id != *required_role_id {
                return Err(AuthError::Forbidden);
            }

            // 4. Inject AuthUser into extensions so handlers don't have to re-extract it
            req.extensions_mut().insert(auth_user);

            Ok(next.run(req).await)
        })
    }
}
