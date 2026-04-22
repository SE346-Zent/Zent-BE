use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::part_conditions;

/// All part statuses that must exist in the database.
pub const PART_CONDITIONS: &[&str] = &["OPERATIONAL", "DEGRADED", "DAMAGED", "SCRAPPED", "LOST_STOLEN"];

/// Seed part statuses and return a map of name -> id.
/// Statuses that already exist are skipped (idempotent).
pub async fn seed_part_conditions(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map = HashMap::new();
    let now = Utc::now();

    for &name in PART_CONDITIONS {
        let existing = part_conditions::Entity::find()
            .filter(part_conditions::Column::Name.eq(name))
            .one(db)
            .await?;

        let id = match existing {
            Some(s) => {
                println!("  PartCondition '{}' already exists (id={})", name, s.id);
                s.id
            }
            None => {
                let inserted = part_conditions::ActiveModel {
                    name: Set(name.to_string()),

                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created part_condition '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}
