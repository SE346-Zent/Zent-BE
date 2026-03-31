use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::work_order_symptoms;


pub const WORK_ORDER_SYMPTOMS: &[&str] = &[
    ""
];

pub async fn seed_work_order_symptoms(db: &DatabaseConnection) -> Result<HashMap<String, i32>> 
{
    Err(anyhow::anyhow!("Not implemented"))
}