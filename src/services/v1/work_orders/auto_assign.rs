use std::collections::HashMap;
use chrono::{Duration, FixedOffset, TimeZone, Utc};
use sea_orm::Set;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{work_orders, users, work_order_state_history},
};

pub struct AutoAssignWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_auto_assign(
    work_order: work_orders::Model,
    technicians: Vec<users::Model>,
    mut technician_agendas: HashMap<Uuid, Vec<work_orders::Model>>,
    policies: &HashMap<String, String>,
    assigned_status_id: i32,
    done_status_id: i32,
    system_user_id: Uuid,
) -> Result<Option<AutoAssignWorkOrderEffect>, AppError> {
    let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
    let j_new_s = work_order.appointment.with_timezone(&tz_offset);

    let workday_start: u32 = policies.get("workday_start")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing workday_start policy")))?
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid workday_start policy")))?;

    let buffer_hours: i64 = policies.get("buffer")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    
    let buffer = Duration::hours(buffer_hours);
    let j_new_e = j_new_s + buffer;

    let mut best_technician_id = None;
    let mut max_delta = Duration::milliseconds(-1);

    for tech in technicians {
        let mut agenda = technician_agendas.remove(&tech.id).unwrap_or_default();
        // sort by appointment
        agenda.sort_by_key(|wo| wo.appointment);

        let mut j_minus_1 = None;
        let mut j_plus_1 = None;

        for job in &agenda {
            if job.id == work_order.id || job.work_order_status_id == done_status_id {
                continue;
            }
            let job_s = job.appointment.with_timezone(&tz_offset);
            
            // Only care about jobs on the same day
            if job_s.date_naive() != j_new_s.date_naive() {
                continue;
            }

            if job_s <= j_new_s {
                j_minus_1 = Some(job_s);
            } else if j_plus_1.is_none() {
                j_plus_1 = Some(job_s);
            }
        }

        // Calculate estimated completion of previous job
        let j_minus_1_e = match j_minus_1 {
            Some(s) => s + buffer,
            None => {
                let start = j_new_s.date_naive().and_hms_opt(workday_start, 0, 0).unwrap();
                tz_offset.from_local_datetime(&start).unwrap()
            }
        };

        // Check validity conditions
        if j_minus_1_e > j_new_s {
            continue; // Overlaps with previous job
        }

        if let Some(j_plus_1_s) = j_plus_1 {
            if j_new_e > j_plus_1_s {
                continue; // Overlaps with next job
            }
        }

        // Compute delta (gap difference)
        let delta = if let Some(j_plus_1_s) = j_plus_1 {
            let d1 = j_new_s - j_minus_1_e;
            let d2 = j_plus_1_s - j_new_e;
            if d1 < d2 { d1 } else { d2 }
        } else {
            j_new_s - j_minus_1_e
        };

        if delta > max_delta {
            max_delta = delta;
            best_technician_id = Some(tech.id);
        }
    }

    if let Some(tech_id) = best_technician_id {
        let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
        active_wo.technician_id = Set(Some(tech_id));
        active_wo.work_order_status_id = Set(assigned_status_id);
        active_wo.updated_at = Set(Utc::now());

        let state_history = work_order_state_history::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_order_id: Set(work_order.id),
            from_status_id: Set(Some(work_order.work_order_status_id)),
            to_status_id: Set(assigned_status_id),
            changed_by_id: Set(system_user_id),
            changed_at: Set(Utc::now()),
        };

        Ok(Some(AutoAssignWorkOrderEffect { work_order: active_wo, state_history }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(appointment: chrono::DateTime<Utc>) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 1, // Pending
            customer_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "".to_string(),
            first_name: "".to_string(),
            last_name: "".to_string(),
            email: None,
            phone_number: None,
            country: "".to_string(),
            province: "".to_string(),
            city: "".to_string(),
            address: "".to_string(),
            building: None,
            appointment,
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            customer_complaint: None,
            customer_complaint_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    fn dummy_technician(id: Uuid) -> users::Model {
        users::Model {
            id,
            role_id: 3,
            account_status: 1,
            email: "".to_string(),
            full_name: "".to_string(),
            password_hash: "".to_string(),
            phone_number: "".to_string(),
            province: Some("".to_string()),
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_decide_auto_assign_success_largest_gap() {
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("buffer".to_string(), "2".to_string());

        let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        // Today at 12:00 PM local time
        let appointment = tz_offset.with_ymd_and_hms(2023, 10, 10, 12, 0, 0).unwrap().with_timezone(&Utc);
        let wo = dummy_work_order(appointment);

        let tech1_id = Uuid::new_v4();
        let tech2_id = Uuid::new_v4();
        let technicians = vec![dummy_technician(tech1_id), dummy_technician(tech2_id)];

        // Tech 1 has a job at 9:00 AM (finishes 11:00 AM, gap 1hr before 12:00 PM)
        let t1_job_time = tz_offset.with_ymd_and_hms(2023, 10, 10, 9, 0, 0).unwrap().with_timezone(&Utc);
        let t1_job = dummy_work_order(t1_job_time);

        // Tech 2 has a job at 8:00 AM (finishes 10:00 AM, gap 2hrs before 12:00 PM) -> Larger gap!
        let t2_job_time = tz_offset.with_ymd_and_hms(2023, 10, 10, 8, 0, 0).unwrap().with_timezone(&Utc);
        let t2_job = dummy_work_order(t2_job_time);

        let mut agendas = HashMap::new();
        agendas.insert(tech1_id, vec![t1_job]);
        agendas.insert(tech2_id, vec![t2_job]);

        let result = decide_auto_assign(wo, technicians, agendas, &policies, 2, 4, Uuid::new_v4());
        assert!(result.is_ok());
        let effect = result.unwrap().unwrap();
        assert_eq!(effect.work_order.technician_id, Set(Some(tech2_id)));
    }

    #[test]
    fn test_decide_auto_assign_no_technicians() {
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());

        let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        let appointment = tz_offset.with_ymd_and_hms(2023, 10, 10, 12, 0, 0).unwrap().with_timezone(&Utc);
        let wo = dummy_work_order(appointment);

        let result = decide_auto_assign(wo, vec![], HashMap::new(), &policies, 2, 4, Uuid::new_v4());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_decide_auto_assign_missing_policies() {
        let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        let appointment = tz_offset.with_ymd_and_hms(2023, 10, 10, 12, 0, 0).unwrap().with_timezone(&Utc);
        let wo = dummy_work_order(appointment);

        let result = decide_auto_assign(wo, vec![], HashMap::new(), &HashMap::new(), 2, 4, Uuid::new_v4());
        assert!(result.is_err()); // Missing workday_start
    }

    #[test]
    fn test_decide_auto_assign_overlapping() {
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("buffer".to_string(), "2".to_string());

        let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        let appointment = tz_offset.with_ymd_and_hms(2023, 10, 10, 12, 0, 0).unwrap().with_timezone(&Utc);
        let wo = dummy_work_order(appointment);

        let tech1_id = Uuid::new_v4();
        let technicians = vec![dummy_technician(tech1_id)];

        // Tech 1 has a job at 11:00 AM (finishes 1:00 PM, overlaps with 12:00 PM)
        let t1_job_time = tz_offset.with_ymd_and_hms(2023, 10, 10, 11, 0, 0).unwrap().with_timezone(&Utc);
        let t1_job = dummy_work_order(t1_job_time);

        let mut agendas = HashMap::new();
        agendas.insert(tech1_id, vec![t1_job]);

        let result = decide_auto_assign(wo, technicians, agendas, &policies, 2, 4, Uuid::new_v4());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Overlap prevents assignment
    }
}
