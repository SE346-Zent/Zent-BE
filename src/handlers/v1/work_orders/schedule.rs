use std::collections::HashMap;
use sea_orm::DatabaseConnection;
use crate::core::lookup_tables::LookupTables;

/// Cron-triggered auto-assign: finds all pending unassigned work orders within the
/// appointment threshold and assigns them to the least-loaded technician in the same province.
pub async fn schedule(
    db: &DatabaseConnection,
    luts: &LookupTables,
    rabbitmq_opt: &Option<std::sync::Arc<lapin::Connection>>,
    templates: &std::sync::Arc<std::collections::HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    use chrono::Utc;
    use tracing::{info, error};
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, TransactionTrait, ActiveModelTrait};
    use crate::entities::{work_orders, users, policy};
    use crate::core::config::AppConfig;

    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;

    let policies_vec = policy::Entity::find().all(db).await?;
    let policies: HashMap<String, String> = policies_vec.into_iter().map(|p| (p.policy_name, p.policy_value)).collect();
    let threshold_hours: i64 = policies.get("auto_assign_threshold_hours").and_then(|v| v.parse().ok()).unwrap_or(3);

    let assigned_status_id = luts.work_order_statuses_by_name.get("Assigned").copied().unwrap_or_else(|| *luts.work_order_statuses_by_name.get("Pending").unwrap());
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap();
    let pending_status_id = luts.work_order_statuses_by_name.get("Pending assignment").copied().unwrap_or_else(|| *luts.work_order_statuses_by_name.get("Pending").unwrap());

    let now = Utc::now();
    let threshold_time = now + chrono::Duration::hours(threshold_hours);

    let target_wos = work_orders::Entity::find()
        .filter(work_orders::Column::WorkOrderStatusId.eq(pending_status_id))
        .filter(work_orders::Column::TechnicianId.is_null())
        .filter(work_orders::Column::Appointment.lte(threshold_time))
        .all(db).await?;

    if target_wos.is_empty() { return Ok(()); }

    let tech_role_id = luts.roles_by_name.get("Technician").ok_or_else(|| anyhow::anyhow!("Technician role not found"))?;

    for wo in target_wos {
        let province = wo.province.clone();
        let technicians = users::Entity::find().filter(users::Column::RoleId.eq(*tech_role_id)).filter(users::Column::Province.eq(province)).all(db).await?;
        if technicians.is_empty() { continue; }

        let tech_ids: Vec<uuid::Uuid> = technicians.iter().map(|t| t.id).collect();
        let agendas = work_orders::Entity::find().filter(work_orders::Column::TechnicianId.is_in(tech_ids.clone())).filter(work_orders::Column::WorkOrderStatusId.ne(done_status_id)).all(db).await?;

        let mut technician_agendas: HashMap<uuid::Uuid, Vec<work_orders::Model>> = HashMap::new();
        for a in agendas { if let Some(tid) = a.technician_id { technician_agendas.entry(tid).or_default().push(a); } }

        let effect = crate::services::v1::work_orders::schedule::decide_auto_assign(wo.clone(), technicians, technician_agendas, &policies, assigned_status_id, done_status_id, system_user_id);
        match effect {
            Ok(Some(eff)) => {
                let tid = eff.work_order.technician_id.clone().unwrap().unwrap();
                db.transaction::<_, (), anyhow::Error>(|txn| Box::pin(async move { eff.work_order.update(txn).await.map_err(|e| anyhow::anyhow!(e))?; eff.state_history.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?; Ok(()) })).await.map_err(|e| anyhow::anyhow!(e))?;
                info!("Auto-assigned WO {} successfully", wo.work_order_number);
                if let Some(rmq) = rabbitmq_opt.as_ref() {
                    let cust = users::Entity::find_by_id(wo.customer_id).one(db).await.unwrap_or_default();
                    let tech = users::Entity::find_by_id(tid).one(db).await.unwrap_or_default();
                    if let (Some(c), Some(t)) = (cust, tech) {
                        let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(rmq, templates, &c.email, &c.full_name, &wo.work_order_number, &t.full_name, &wo.appointment.to_string()).await;
                    }
                }
            }
            Ok(None) => info!("No suitable technician for WO {}", wo.work_order_number),
            Err(e) => error!("Auto-assign failed for WO {}: {}", wo.work_order_number, e),
        }
    }
    Ok(())
}
