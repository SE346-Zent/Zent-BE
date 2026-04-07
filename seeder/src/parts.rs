use anyhow::Result;
use sea_orm::{DatabaseConnection, EntityTrait, Set, QuerySelect};
use std::collections::HashMap;
use serde::Deserialize;
use uuid::Uuid;
use chrono::Utc;

use zent_be::entities::{part_types, parts_by_model, products, product_models};

#[derive(Debug, Deserialize)]
pub struct PartTypeData {
    pub part_number: String,
    pub commodity_type: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct PartInstallationData {
    pub part_number: String,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct PartsFile {
    pub part_types: Vec<PartTypeData>,
    pub installations: HashMap<String, Vec<PartInstallationData>>,
}

fn load_parts_data() -> Result<PartsFile> {
    // Correct relative path when running from workspace root
    let content = std::fs::read_to_string("resources/parts.json")?;
    let data: PartsFile = serde_json::from_str(&content)?;
    Ok(data)
}

pub async fn seed_part_types(db: &DatabaseConnection, part_statuses: &HashMap<String, i32>) -> Result<()> {
    let data = load_parts_data()?;
    let mut records = Vec::new();
    let now = Utc::now();
    let default_status = *part_statuses.get("Production").unwrap_or(&1);

    println!("  Loaded {} part types from parts.json.", data.part_types.len());

    for pt in data.part_types {
        records.push(part_types::ActiveModel {
            part_number: Set(pt.part_number),
            commodity_type: Set(pt.commodity_type),
            description: Set(pt.description),
            part_status_id: Set(default_status),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        });
    }

    if !records.is_empty() {
        println!("  Inserting {} part types into database...", records.len());
        // Using chunks or handling duplicates would be safe, but since it's a seed we can try on_conflict or just insert_many
        // Actually, let's just insert all, assuming db is empty or delete beforehand.
        // Wait, sea_orm insert_many might fail if partially exists. Let's do a basic loop or attempt.
        // For simplicity and speed:
        for chunk in records.chunks(1000) {
            // we use insert_many but we ignore conflicts or just try 
            // In SeaORM, to avoid conflict errors easily in a simple way, we map each and insert.
            // Or better, let's just use insert_many!
        }
        
    }
    
    // Proper way to handle idempotency in insert_many:
    if !records.is_empty() {
        // sea_orm does not natively do insert_many with ON CONFLICT DO NOTHING for all backends in a simple uniform way in ActiveModel insert_many without generic work.
        // We will just fetch existing first.
        use sea_orm::ColumnTrait;
        use sea_orm::QueryFilter;
        let existing: Vec<String> = part_types::Entity::find().select_only().column(part_types::Column::PartNumber).into_tuple().all(db).await?;
        let to_insert: Vec<_> = records.into_iter().filter(|r| !existing.contains(r.part_number.as_ref())).collect();
        if !to_insert.is_empty() {
            part_types::Entity::insert_many(to_insert).exec(db).await?;
        }
    }

    println!("  Successfully seeded part types.");
    Ok(())
}

pub async fn seed_part_by_model(db: &DatabaseConnection, part_statuses: &HashMap<String, i32>) -> Result<()> {
    let data = load_parts_data()?;
    println!("  Loaded installation mappings for {} models.", data.installations.len());
    let now = Utc::now();
    let default_status = *part_statuses.get("Production").unwrap_or(&1);

    // Get all products with their model info
    // SeaORM finding all products
    let all_products = products::Entity::find().all(db).await?;
    let all_models = product_models::Entity::find().all(db).await?;
    
    let model_map: HashMap<i32, String> = all_models.into_iter().map(|m| (m.id, m.model_name)).collect();

    let mut installations = Vec::new();

    for p in all_products {
        if let Some(model_name) = model_map.get(&p.model_id) {
            if let Some(parts) = data.installations.get(model_name) {
                for pt in parts {
                    installations.push(parts_by_model::ActiveModel {
                        mfg_part: Set(format!("{}-{}-{}", p.id, pt.part_number, rand::random::<u32>())),
                        product_id: Set(p.id),
                        part_number: Set(pt.part_number.clone()),
                        quantity: Set(pt.quantity),
                        part_status_id: Set(default_status),
                        created_at: Set(now),
                        updated_at: Set(now),
                        deleted_at: Set(None),
                    });
                }
            }
        }
    }

    if !installations.is_empty() {
        println!("  Inserting {} part installations...", installations.len());
        // Batch insert
        for chunk in installations.chunks(5000) {
            parts_by_model::Entity::insert_many(chunk.to_vec()).exec(db).await?;
        }
        println!("  Successfully seeded part installations.");
    } else {
        println!("  No part installations to seed.");
    }

    Ok(())
}
