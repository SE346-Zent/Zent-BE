use tokio_cron_scheduler::Job;
use sea_orm::*;
use sea_orm::prelude::DateTimeUtc;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::core::config::AppConfig;
use crate::core::lookup_tables::LookupTables;
use crate::entities::{work_orders, work_order_state_history, work_order_escalations, users};
use crate::infrastructure::cache::ValkeyClient;

/// Build a cron job that monitors InProg work orders and escalates when
/// elapsed time exceeds 110% / 125% / 150% of the configurable baseline.
///
/// Runs every 5 minutes. Notifies SuperAdmins (always) and province admins
/// at each threshold, skipping the system user. Inserts an audit row into
/// `work_order_escalations` for each escalation event.
pub fn build_escalation_job(
    db: DatabaseConnection,
    lookup_tables: Arc<LookupTables>,
    mongodb: Arc<mongodb::Database>,
    valkey: Option<Arc<ValkeyClient>>,
) -> Result<Job, anyhow::Error> {
    let job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
        let db = db.clone();
        let luts = lookup_tables.clone();
        let mongodb = mongodb.clone();
        let valkey = valkey.clone();
        Box::pin(async move {
            info!("Running escalation check job...");
            if let Err(e) = run_escalation_check(&db, &luts, &mongodb, valkey).await {
                error!("Escalation check failed: {:?}", e);
            }
        })
    })?;
    Ok(job)
}

async fn run_escalation_check(
    db: &DatabaseConnection,
    luts: &LookupTables,
    mongodb: &mongodb::Database,
    valkey: Option<Arc<ValkeyClient>>,
) -> Result<(), anyhow::Error> {
    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;

    // ── Resolve status & role IDs ──────────────────────────────────
    let in_prog_id = *luts.work_order_statuses_by_name
        .get("InProg")
        .ok_or_else(|| anyhow::anyhow!("'InProg' status not found"))?;

    let paused_id = luts.work_order_statuses_by_name
        .get("Paused")
        .copied();

    let super_admin_role_id = *luts.roles_by_name
        .get("SuperAdmin")
        .ok_or_else(|| anyhow::anyhow!("'SuperAdmin' role not found"))?;

    let admin_role_id = *luts.roles_by_name
        .get("Admin")
        .ok_or_else(|| anyhow::anyhow!("'Admin' role not found"))?;

    // ── Baseline from policies ─────────────────────────────────────
    let baseline_minutes: i64 = luts.policies
        .get("work_order_escalation_baseline_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    // ── Batch 1: all InProg and Paused work orders ─────────────────
    use sea_orm::Condition;
    let mut status_cond = Condition::any()
        .add(work_orders::Column::WorkOrderStatusId.eq(in_prog_id));
    if let Some(pid) = paused_id {
        status_cond = status_cond.add(work_orders::Column::WorkOrderStatusId.eq(pid));
    }

    let active_wos = work_orders::Entity::find()
        .filter(status_cond)
        .all(db)
        .await?;

    if active_wos.is_empty() {
        return Ok(());
    }

    let wo_ids: Vec<Uuid> = active_wos.iter().map(|wo| wo.id).collect();

    info!("Escalation check: {} active (InProg/Paused) work orders found", active_wos.len());

    // ── Batch 2: highest escalation level per WO (one query) ───────
    let existing_escalations = work_order_escalations::Entity::find()
        .filter(work_order_escalations::Column::WorkOrderId.is_in(wo_ids.clone()))
        .all(db)
        .await?;

    // Group by work_order_id, keeping the max escalation_level
    let mut highest_levels: HashMap<Uuid, i32> = HashMap::new();
    for esc in &existing_escalations {
        let entry = highest_levels.entry(esc.work_order_id).or_insert(0);
        if esc.escalation_level > *entry {
            *entry = esc.escalation_level;
        }
    }

    // ── Batch 3: state_history "InProg" or "Paused" transitions ───
    let mut trans_cond = Condition::any()
        .add(work_order_state_history::Column::ToStatusId.eq(in_prog_id));
    if let Some(pid) = paused_id {
        trans_cond = trans_cond.add(work_order_state_history::Column::ToStatusId.eq(pid));
    }

    let all_start_entries = work_order_state_history::Entity::find()
        .filter(work_order_state_history::Column::WorkOrderId.is_in(wo_ids.clone()))
        .filter(trans_cond)
        .order_by_desc(work_order_state_history::Column::ChangedAt)
        .all(db)
        .await?;

    // Group by work_order_id, keep the most recent (first per WO after sort desc)
    let mut start_times: HashMap<Uuid, DateTimeUtc> = HashMap::new();
    for entry in &all_start_entries {
        start_times.entry(entry.work_order_id).or_insert(entry.changed_at);
    }

    // ── Batch 4: SuperAdmins (once) ─────────────────────────────────
    let super_admins = users::Entity::find()
        .filter(users::Column::RoleId.eq(super_admin_role_id))
        .filter(users::Column::DeletedAt.is_null())
        .all(db)
        .await?;

    // ── Batch 5: all Admins grouped by province (one query) ────────
    let all_admins = users::Entity::find()
        .filter(users::Column::RoleId.eq(admin_role_id))
        .filter(users::Column::DeletedAt.is_null())
        .all(db)
        .await?;

    let mut admins_by_province: HashMap<String, Vec<users::Model>> = HashMap::new();
    for admin in all_admins {
        if let Some(ref province) = admin.province {
            admins_by_province.entry(province.clone()).or_default().push(admin);
        }
    }

    // ── Process each InProg WO ─────────────────────────────────────
    let now: DateTimeUtc = Utc::now().into();

    for wo in &active_wos {
        // Get start time (skip if never transitioned to InProg)
        let start_time = match start_times.get(&wo.id) {
            Some(t) => *t,
            None => {
                warn!("WO {} in InProg but no state_history entry found — skipping", wo.work_order_number);
                continue;
            }
        };

        let elapsed_minutes = now.signed_duration_since(start_time).num_minutes();
        let highest_level = highest_levels.get(&wo.id).copied().unwrap_or(0);

        // ── Call pure service logic ─────────────────────────────────
        let effect = match crate::services::v1::work_orders::escalation::decide_escalation(
            baseline_minutes,
            elapsed_minutes,
            highest_level,
        ) {
            Some(e) => e,
            None => continue,
        };

        // ── Province admins (from pre-built map) ────────────────────
        let province_admins = admins_by_province.get(&wo.province);

        let notification_data = serde_json::json!({
            "workOrderId": wo.id,
            "workOrderNumber": wo.work_order_number,
            "elapsedMinutes": elapsed_minutes,
            "baselineMinutes": baseline_minutes,
            "escalationLevel": effect.level_label,
            "province": wo.province,
        });

        let title = format!("Escalation: Work Order {} ({})", wo.work_order_number, effect.level_label);
        let body = format!(
            "WO {} has been in progress for {} min (baseline: {} min, {}). Province: {}",
            wo.work_order_number, elapsed_minutes, baseline_minutes, effect.level_label, wo.province
        );

        let mut sa_count: i32 = 0;
        let mut admin_count: i32 = 0;

        // ── Notify SuperAdmins ──────────────────────────────────────
        for sa in &super_admins {
            if sa.id == system_user_id {
                continue;
            }
            match crate::handlers::v1::notifications::send_notification::send_notification(
                mongodb,
                valkey.clone(),
                db,
                sa.id,
                "work_order_escalation",
                &title,
                &body,
                notification_data.clone(),
            ).await {
                Ok(()) => sa_count += 1,
                Err(e) => warn!("Failed to notify SA {} for WO {}: {:?}", sa.id, wo.work_order_number, e),
            }
        }

        // ── Notify Province Admins ──────────────────────────────────
        if let Some(admins) = province_admins {
            for admin in admins {
                if admin.id == system_user_id {
                    continue;
                }
                match crate::handlers::v1::notifications::send_notification::send_notification(
                    mongodb,
                    valkey.clone(),
                    db,
                    admin.id,
                    "work_order_escalation",
                    &title,
                    &body,
                    notification_data.clone(),
                ).await {
                    Ok(()) => admin_count += 1,
                    Err(e) => warn!("Failed to notify admin {} for WO {}: {:?}", admin.id, wo.work_order_number, e),
                }
            }
        }

        // ── Insert audit row only if at least one notification succeeded ──
        if sa_count > 0 || admin_count > 0 {
            let audit = work_order_escalations::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_order_id: Set(wo.id),
                escalation_level: Set(effect.target_level),
                elapsed_minutes: Set(elapsed_minutes),
                baseline_minutes: Set(baseline_minutes),
                notified_sa_count: Set(sa_count),
                notified_admin_count: Set(admin_count),
                created_at: Set(now),
            };
            audit.insert(db).await?;

            info!(
                "Escalation {} triggered for WO {} — notified {} SA(s) and {} admin(s)",
                effect.level_label, wo.work_order_number, sa_count, admin_count,
            );
        } else {
            warn!(
                "Escalation {} for WO {} had zero successful deliveries — audit skipped, will retry",
                effect.level_label, wo.work_order_number,
            );
        }
    }

    Ok(())
}
