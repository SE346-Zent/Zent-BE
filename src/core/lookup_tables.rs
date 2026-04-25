use sea_orm::{DatabaseConnection, EntityTrait};
use std::collections::HashMap;

use crate::entities::{account_status, part_conditions, part_types, roles, work_order_statuses};

/// In-memory lookup tables (LUT) loaded once at server startup.
///
/// These are small, rarely-changing reference tables that are used frequently
/// throughout the application. Caching them in memory avoids repeated DB lookups.
#[derive(Clone, Debug)]
pub struct LookupTables {
    /// `roles.id` → `roles.name`
    pub roles: HashMap<i32, String>,
    /// `roles.name` → `roles.id`
    pub roles_by_name: HashMap<String, i32>,

    /// `account_status.id` → `account_status.name`
    pub account_statuses: HashMap<i32, String>,
    /// `account_status.name` → `account_status.id`
    pub account_statuses_by_name: HashMap<String, i32>,

    /// `part_types.id` → `part_types.part_type_name`
    pub part_types: HashMap<i32, String>,
    /// `part_types.part_type_name` → `part_types.id`
    pub part_types_by_name: HashMap<String, i32>,

    /// `part_conditions.id` → `part_conditions.name`
    pub part_conditions: HashMap<i32, String>,
    /// `part_conditions.name` → `part_conditions.id`
    pub part_conditions_by_name: HashMap<String, i32>,

    /// `work_order_statuses.id` → `work_order_statuses.name`
    pub work_order_statuses: HashMap<i32, String>,
    /// `work_order_statuses.name` → `work_order_statuses.id`
    pub work_order_statuses_by_name: HashMap<String, i32>,
}

impl LookupTables {
    /// Load all lookup tables from the database.
    pub async fn load(db: &DatabaseConnection) -> Result<Self, sea_orm::DbErr> {
        let roles_list = roles::Entity::find().all(db).await?;
        let account_statuses_list = account_status::Entity::find().all(db).await?;
        let part_types_list = part_types::Entity::find().all(db).await?;
        let part_conditions_list = part_conditions::Entity::find().all(db).await?;
        let work_order_statuses_list = work_order_statuses::Entity::find().all(db).await?;

        let roles: HashMap<i32, String> = roles_list.iter().map(|r| (r.id, r.name.clone())).collect();
        let roles_by_name: HashMap<String, i32> = roles_list.iter().map(|r| (r.name.clone(), r.id)).collect();

        let account_statuses: HashMap<i32, String> = account_statuses_list.iter().map(|a| (a.id, a.name.clone())).collect();
        let account_statuses_by_name: HashMap<String, i32> = account_statuses_list.iter().map(|a| (a.name.clone(), a.id)).collect();

        let part_types: HashMap<i32, String> = part_types_list.iter().map(|p| (p.id, p.part_type_name.clone())).collect();
        let part_types_by_name: HashMap<String, i32> = part_types_list.iter().map(|p| (p.part_type_name.clone(), p.id)).collect();

        let part_conditions: HashMap<i32, String> = part_conditions_list.iter().map(|p| (p.id, p.name.clone())).collect();
        let part_conditions_by_name: HashMap<String, i32> = part_conditions_list.iter().map(|p| (p.name.clone(), p.id)).collect();

        let work_order_statuses: HashMap<i32, String> = work_order_statuses_list.iter().map(|w| (w.id, w.name.clone())).collect();
        let work_order_statuses_by_name: HashMap<String, i32> = work_order_statuses_list.iter().map(|w| (w.name.clone(), w.id)).collect();

        tracing::info!(
            roles = roles.len(),
            account_statuses = account_statuses.len(),
            part_types = part_types.len(),
            part_conditions = part_conditions.len(),
            work_order_statuses = work_order_statuses.len(),
            "Lookup tables loaded into memory"
        );

        Ok(Self {
            roles,
            roles_by_name,
            account_statuses,
            account_statuses_by_name,
            part_types,
            part_types_by_name,
            part_conditions,
            part_conditions_by_name,
            work_order_statuses,
            work_order_statuses_by_name,
        })
    }

    /// Create empty lookup tables (useful for tests).
    pub fn empty() -> Self {
        Self {
            roles: HashMap::new(),
            roles_by_name: HashMap::new(),
            account_statuses: HashMap::new(),
            account_statuses_by_name: HashMap::new(),
            part_types: HashMap::new(),
            part_types_by_name: HashMap::new(),
            part_conditions: HashMap::new(),
            part_conditions_by_name: HashMap::new(),
            work_order_statuses: HashMap::new(),
            work_order_statuses_by_name: HashMap::new(),
        }
    }
}
