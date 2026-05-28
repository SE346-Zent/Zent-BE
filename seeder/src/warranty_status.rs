use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::warranty_statuses;

pub const WARRANTY_STATUSES: &[&str] = &["Active", "Expired", "Voided"];

pub async fn seed_warranty_statuses(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map = HashMap::new();

    for &name in WARRANTY_STATUSES {
        let existing = warranty_statuses::Entity::find()
            .filter(warranty_statuses::Column::Name.eq(name))
            .one(db)
            .await?;

        let id = match existing {
            Some(status) => {
                println!("  WarrantyStatus '{}' already exists (id={})", name, status.id);
                status.id
            }
            None => {
                let inserted = warranty_statuses::ActiveModel {
                    name: Set(name.to_string()),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created warranty_status '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}