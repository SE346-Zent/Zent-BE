use tokio_cron_scheduler::Job;
use sea_orm::*;
use chrono::Utc;
use crate::entities::{work_orders, users, policy};
use crate::core::lookup_tables::LookupTables;
use crate::core::config::AppConfig;
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, error};

pub fn build_auto_assign_job(
    db: DatabaseConnection,
    luts: std::sync::Arc<LookupTables>,
) -> Result<Job, anyhow::Error> {
    // Run every 1 hour at the top of the hour: "0 0 * * * *"
    let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let luts_clone = luts.clone();
        Box::pin(async move {
            info!("Running auto-assign job...");
            if let Err(e) = process_auto_assign(&db_clone, &luts_clone).await {
                error!("Error in auto-assign job: {:?}", e);
            }
        })
    })?;
    Ok(job)
}

async fn process_auto_assign(
    db: &DatabaseConnection,
    luts: &LookupTables,
) -> Result<(), anyhow::Error> {
    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;

    let policies_vec = policy::Entity::find().all(db).await?;
    let policies: HashMap<String, String> = policies_vec.into_iter()
        .map(|p| (p.policy_name, p.policy_value))
        .collect();

    let threshold_hours: i64 = policies.get("auto_assign_threshold_hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let assigned_status_id = luts.work_order_statuses_by_name.get("Assigned")
        .copied()
        .unwrap_or_else(|| *luts.work_order_statuses_by_name.get("Pending").unwrap());

    let done_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap();

    let pending_status_id = luts.work_order_statuses_by_name.get("Pending assignment")
        .copied()
        .unwrap_or_else(|| *luts.work_order_statuses_by_name.get("Pending").unwrap());

    let now = Utc::now();
    let threshold_time = now + chrono::Duration::hours(threshold_hours);

    let target_wos = work_orders::Entity::find()
        .filter(work_orders::Column::WorkOrderStatusId.eq(pending_status_id))
        .filter(work_orders::Column::TechnicianId.is_null())
        .filter(work_orders::Column::Appointment.lte(threshold_time))
        .all(db).await?;

    if target_wos.is_empty() {
        return Ok(());
    }

    let tech_role_id = luts.roles_by_name.get("Technician")
        .ok_or_else(|| anyhow::anyhow!("Technician role not found"))?;

    for wo in target_wos {
        let province = wo.province.clone();

        let technicians = users::Entity::find()
            .filter(users::Column::RoleId.eq(*tech_role_id))
            .filter(users::Column::Province.eq(province))
            .all(db).await?;

        if technicians.is_empty() {
            continue;
        }

        let tech_ids: Vec<Uuid> = technicians.iter().map(|t| t.id).collect();
        let agendas = work_orders::Entity::find()
            .filter(work_orders::Column::TechnicianId.is_in(tech_ids.clone()))
            .filter(work_orders::Column::WorkOrderStatusId.ne(done_status_id))
            .all(db).await?;

        let mut technician_agendas: HashMap<Uuid, Vec<work_orders::Model>> = HashMap::new();
        for agenda_wo in agendas {
            if let Some(tid) = agenda_wo.technician_id {
                technician_agendas.entry(tid).or_default().push(agenda_wo);
            }
        }

        let effect = crate::services::v1::work_orders::auto_assign::decide_auto_assign(
            wo.clone(),
            technicians,
            technician_agendas,
            &policies,
            assigned_status_id,
            done_status_id,
            system_user_id,
        );

        match effect {
            Ok(Some(eff)) => {
                db.transaction::<_, (), anyhow::Error>(|txn| {
                    Box::pin(async move {
                        eff.work_order.update(txn).await.map_err(|e| anyhow::anyhow!(e))?;
                        eff.state_history.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?;
                        Ok(())
                    })
                }).await.map_err(|e| anyhow::anyhow!(e))?;
                info!("Auto-assigned WO {} successfully", wo.work_order_number);
            }
            Ok(None) => {
                info!("No suitable technician found for auto-assigning WO {}", wo.work_order_number);
            }
            Err(e) => {
                error!("Failed to decide auto assign for WO {}: {}", wo.work_order_number, e);
            }
        }
    }

    Ok(())
}
