use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::refuse_request::{RefuseWorkOrderRequest, RefuseWorkOrderMultipart};
use crate::model::responses::base::ApiResponse;

/// Submit a work order refusal request with reason, explanation, and evidence images.

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/refuse",
    request_body(content = RefuseWorkOrderMultipart, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Work order refusal submitted successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse), (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse), (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn refuse(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut reason = String::new();
    let mut explanation = String::new();
    let mut photos_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "photos" {
            let ct = field.content_type().unwrap_or("image/jpeg").to_string();
            let fn_ = field.file_name().unwrap_or("photo.jpg").to_string();
            if let Ok(data) = field.bytes().await { photos_data.push((data, ct, fn_)); }
        } else if let Ok(text) = field.text().await {
            match name.as_str() { "reason" => reason = text, "explanation" => explanation = text, _ => {} }
        }
    }

    if reason.is_empty() { return Err(AppError::BadRequest("reason is required".to_string())); }
    if photos_data.len() > 5 { return Err(AppError::BadRequest("A maximum of 5 photos are allowed".to_string())); }

    // Write-through: use the cache for individual work order instead of querying DB
    let wo = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;
    if wo.technician_id != Some(auth.user.id) { return Err(AppError::Forbidden("You are not assigned to this work order".to_string())); }

    let mut urls = Vec::new();
    for (data, ct, file_name) in photos_data {
        let ext = file_name.split('.').next_back().unwrap_or("jpg");
        let name = format!("{}/refusal/{}.{}", id, chrono::Utc::now().timestamp(), ext);
        crate::utils::oci::upload_object(&name, data.to_vec(), &ct).await?;
        urls.push(name);
    }

    let payload = RefuseWorkOrderRequest { reason, explanation, evidence_image_urls: urls };
    let status_id = *luts.work_order_statuses_by_name.get("Reject_InReview").ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Reject_InReview' status missing")))?;
    let effect = crate::services::v1::work_orders::refuse::decide_refuse_work_order(payload, wo, status_id, auth.user.id)?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.reject_form_model.insert(txn).await?;
        for img in effect.image_models { img.insert(txn).await?; }
        for link in effect.image_link_models { link.insert(txn).await?; }
        effect.work_order_model.update(txn).await?;
        effect.state_history_model.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    Ok(Json(ApiResponse::success(200, "Work order refusal submitted successfully", ())))
}
