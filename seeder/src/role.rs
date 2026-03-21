use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::role;

/// All roles that must exist in the database.
pub const ROLES: &[&str] = &["Admin", "SuperAdmin", "Customer", "Technician"];

/// Seed roles and return a map of name -> id.
/// Roles that already exist are skipped (idempotent).
pub async fn seed_roles(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map = HashMap::new();

    for &name in ROLES {
        let existing = role::Entity::find()
            .filter(role::Column::Name.eq(name))
            .one(db)
            .await?;

        let id = match existing {
            Some(r) => {
                println!("  Role '{}' already exists (id={})", name, r.id);
                r.id
            }
            None => {
                let inserted = role::ActiveModel {
                    name: Set(name.to_string()),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created role '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}