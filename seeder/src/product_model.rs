use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use uuid::Uuid;
use zent_be::entities::product_models;

/// Realistic Lenovo product models to seed.
pub const PRODUCT_MODELS: &[(&str, &str)] = &[
    ("IdeaPad Pro 5 16AKP10", "83JN002SVN"),
    ("Legion 5 15IRX10 - Type 83LY", "83LY00NGVN"),
    ("LOQ 15AHP9", "83DX007QVN"),
    ("ThinkBook 16 G9 IPL - Type 21UR", "21UR0009US"),
    ("X9-15p Gen 1 - Type 21VV, 21VW", "21VV000UVN"),
    ("Yoga 9 2-in-1 14IPH11", "83SE000HVN")
];

/// Seed product models and return a map of model_name -> id.
/// Models that already exist are skipped (idempotent).
pub async fn seed_product_models(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map = HashMap::new();
    let now = Utc::now();

    for &(model_name, model_code) in PRODUCT_MODELS {
        let existing = product_models::Entity::find()
            .filter(product_models::Column::ModelName.eq(model_name))
            .one(db)
            .await?;

        let id = match existing {
            Some(m) => {
                println!(
                    "  ProductModel '{}' already exists (id={})",
                    model_name, m.id
                );
                m.id
            }
            None => {
                let inserted = product_models::ActiveModel {
                    model_name: Set(model_name.to_string()),
                    model_code: Set(model_code.to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!(
                    "  Created product_model '{}' (id={})",
                    model_name, inserted.id
                );
                inserted.id
            }
        };

        map.insert(model_name.to_string(), id);
    }

    Ok(map)
}
