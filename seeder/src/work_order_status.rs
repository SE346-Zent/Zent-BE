use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::work_order_statuses;
use chrono::Utc;

pub const WO_STATUSES: &[&str] = &["Pending", "Assigned", "InProg", "Closed", "Reject_InReview", "Rejected"];

pub async fn seed_work_order_statuses(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map: HashMap<String, i32> = HashMap::new();
    let now = Utc::now();

    for &name in WO_STATUSES {
        let existing = work_order_statuses::Entity::find()
            .filter(work_order_statuses::Column::Name.eq(name))
            .one(db)
            .await?;

        let id: i32 = match existing {
            Some(s) => {
                println!("  Work order status '{}' already exists (id={})", name, s.id);
                s.id
            }
            None => {
                let inserted = work_order_statuses::ActiveModel {
                    name: Set(name.to_string()),

                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created work order status '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}