use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::account_status;

/// All account statuses that must exist in the database.
pub const ACCOUNT_STATUSES: &[&str] = &["Active", "Inactive", "Locked", "Terminated", "Pending"];

/// Seed account statuses and return a map of name -> id.
/// Statuses that already exist are skipped (idempotent).
pub async fn seed_account_statuses(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map = HashMap::new();

    for &name in ACCOUNT_STATUSES {
        let existing = account_status::Entity::find()
            .filter(account_status::Column::Name.eq(name))
            .one(db)
            .await?;

        let id = match existing {
            Some(s) => {
                println!("  AccountStatus '{}' already exists (id={})", name, s.id);
                s.id
            }
            None => {
                let inserted = account_status::ActiveModel {
                    name: Set(name.to_string()),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created account_status '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}