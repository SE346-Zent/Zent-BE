use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::policy;

/// Seed policies and return a map of name -> value.
pub async fn seed_policies(db: &DatabaseConnection) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();

    let policies = vec![
        ("pending_customer_cleanup_hours", "1"),
        ("pending_staff_cleanup_days", "3"),
        ("buffer", "5"),
        ("baseline", "2"),
        ("workday_start", "7"),
        ("workday_end", "17"),
        ("geofencing_radius", "2000"),
        ("auto_assign_threshold_hours", "3"),
        ("unassigned_cleanup_threshold_hours", "3"),
        ("internet_time_drift_minutes", "30")
    ];

    for (name, value) in policies {
        let existing = policy::Entity::find()
            .filter(policy::Column::PolicyName.eq(name))
            .one(db)
            .await?;

        let val = match existing {
            Some(p) => {
                println!("  Policy '{}' already exists (value={})", name, p.policy_value);
                p.policy_value
            }
            None => {
                let inserted = policy::ActiveModel {
                    policy_name: Set(name.to_string()),
                    policy_value: Set(value.to_string()),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created policy '{}' (value={})", name, inserted.policy_value);
                inserted.policy_value
            }
        };

        map.insert(name.to_string(), val);
    }

    Ok(map)
}
