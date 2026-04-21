use anyhow::Result;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ColumnTrait, QueryFilter, ActiveModelTrait};
use std::collections::HashMap;
use serde::Deserialize;
use uuid::Uuid;
use chrono::Utc;
use fake::{Fake, Faker};

use zent_be::entities::{
    part_types, part_catalog, parts_by_model, parts,
    product_models, products, images, 
    product_image_links, product_model_image_links, part_image_links, part_catalog_image_links
};

#[derive(Debug, Deserialize)]
pub struct PartTypeData {
    pub commodity_type: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct PartCatalogData {
    pub part_number: String,
    pub commodity_type: String,
    pub mfg_number: String,
}

#[derive(Debug, Deserialize)]
pub struct PartInstallationData {
    pub part_number: String,
    pub quantity: i32,
    pub mfg_number: String,
}

#[derive(Debug, Deserialize)]
pub struct PartsFile {
    pub part_types: Vec<PartTypeData>,
    pub part_catalogs: Vec<PartCatalogData>,
    pub installations: HashMap<String, Vec<PartInstallationData>>,
}

fn load_parts_data() -> Result<PartsFile> {
    let content = std::fs::read_to_string("resources/parts.json")?;
    let data: PartsFile = serde_json::from_str(&content)?;
    Ok(data)
}

pub async fn seed_parts_and_catalogs(db: &DatabaseConnection, part_statuses: &HashMap<String, i32>) -> Result<()> {
    let data = load_parts_data()?;
    let now = Utc::now();
    let default_status = *part_statuses.get("Production").unwrap_or(&1);

    println!("  Loaded {} part types from parts.json.", data.part_types.len());

    // 1. Seed PartTypes
    let mut type_id_map: HashMap<String, i32> = HashMap::new();
    for pt in data.part_types {
        let existing = part_types::Entity::find()
            .filter(part_types::Column::PartTypeName.eq(&pt.commodity_type))
            .one(db)
            .await?;

        let id = if let Some(e) = existing {
            e.id
        } else {
            let inserted = part_types::ActiveModel {
                part_type_name: Set(pt.commodity_type.clone()),
                description: Set(Some(pt.description)),
                ..Default::default()
            }
            .insert(db)
            .await?;
            inserted.id
        };
        type_id_map.insert(pt.commodity_type, id);
    }
    println!("  Successfully seeded {} part types.", type_id_map.len());

    // 2. Seed PartCatalog
    println!("  Seeding part catalogs...");
    let mut catalog_map: HashMap<(i32, String), Uuid> = HashMap::new(); // (PartTypesId, MFGNumber) -> PartCatalog.Id

    for pc in data.part_catalogs {
        if let Some(&type_id) = type_id_map.get(&pc.commodity_type) {
            let existing = part_catalog::Entity::find()
                .filter(part_catalog::Column::PartTypesId.eq(type_id))
                .filter(part_catalog::Column::MfgNumber.eq(&pc.mfg_number))
                .one(db)
                .await?;

            let id = if let Some(e) = existing {
                e.id
            } else {
                let new_id = Uuid::new_v4();
                part_catalog::ActiveModel {
                    id: Set(new_id),
                    part_number: Set(pc.part_number),
                    part_types_id: Set(type_id),
                    mfg_number: Set(pc.mfg_number.clone()),
                    part_mfg_status: Set(default_status),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                }
                .insert(db)
                .await?;
                
                // Add Image for Catalog
                let img_id = seed_image(db).await?;
                part_catalog_image_links::ActiveModel {
                    image_id: Set(img_id),
                    part_catalog_id: Set(new_id),
                }.insert(db).await?;

                new_id
            };
            catalog_map.insert((type_id, pc.mfg_number.clone()), id);
        }
    }
    println!("  Successfully seeded {} part catalogs.", catalog_map.len());

    // 3. Seed PartsByModel
    println!("  Seeding parts by model (installations)...");
    for (model_code, installs) in data.installations {
        let existing_model = product_models::Entity::find()
            .filter(product_models::Column::ModelCode.eq(&model_code))
            .one(db)
            .await?;
        
        if existing_model.is_some() {
            for inst in installs {
                // Find PartCatalogId by resolving commodity recursively or searching DB.
                // Wait, installations only have part_number and mfg_number. 
                // We need to query the part_catalog table to find the UUID.
                let cat_item = part_catalog::Entity::find()
                    .filter(part_catalog::Column::PartNumber.eq(&inst.part_number))
                    .filter(part_catalog::Column::MfgNumber.eq(&inst.mfg_number))
                    .one(db)
                    .await?;

                if let Some(cat) = cat_item {
                    let existing_link = parts_by_model::Entity::find()
                        .filter(parts_by_model::Column::PartCatalogId.eq(cat.id))
                        .filter(parts_by_model::Column::ProductModelCode.eq(&model_code))
                        .one(db)
                        .await?;
                    
                    if existing_link.is_none() {
                        parts_by_model::ActiveModel {
                            part_catalog_id: Set(cat.id),
                            product_model_code: Set(model_code.clone()),
                            quantity: Set(inst.quantity),
                        }.insert(db).await?;
                    }
                }
            }
        }
    }
    println!("  Successfully seeded part installations.");

    // 4. Seed Random Parts instances for each product
    println!("  Seeding individual part instances attached to products...");
    let products = products::Entity::find().all(db).await?;
    let mut parts_created = 0;
    for product in products {
        // Query the parts designed for this product's model
        let model_parts = parts_by_model::Entity::find()
            .filter(parts_by_model::Column::ProductModelCode.eq(&product.product_model_code))
            .all(db)
            .await?;
        
        for mp in model_parts {
            for _ in 0..mp.quantity {
                let p_id = Uuid::new_v4();
                parts::ActiveModel {
                    id: Set(p_id),
                    part_catalog_id: Set(mp.part_catalog_id),
                    product_id: Set(Some(product.id)),
                    serial_number: Set(uuid::Uuid::new_v4().to_string().chars().take(12).collect()), // Short random SN
                    part_condition_id: Set(1), // Assume pristine condition initially
                    manufactured_date: Set(now),
                    installation_date: Set(Some(now)),
                    removal_date: Set(None),
                    scrapped_date: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                }.insert(db).await?;
                
                // Add Image for Part
                let img_id = seed_image(db).await?;
                part_image_links::ActiveModel {
                    image_id: Set(img_id),
                    part_id: Set(p_id),
                }.insert(db).await?;

                parts_created += 1;
            }
        }
    }
    println!("  Successfully seeded {} part instances.", parts_created);

    Ok(())
}

async fn seed_image(db: &DatabaseConnection) -> Result<Uuid> {
    let id = Uuid::new_v4();
    // Some random laptop/hardware images
    let r: u8 = rand::random();
    let url = match r % 4 {
        0 => "https://images.unsplash.com/photo-1518770660439-4636190af475?auto=format&fit=crop&w=500&q=60".to_string(), // Circuit board
        1 => "https://images.unsplash.com/photo-1597872200969-2b65d56bd16b?auto=format&fit=crop&w=500&q=60".to_string(), // PC components
        2 => "https://images.unsplash.com/photo-1587202372775-e229f172b9d7?auto=format&fit=crop&w=500&q=60".to_string(), // Motherboard
        _ => "https://images.unsplash.com/photo-1603513492128-ba7bc9b3e143?auto=format&fit=crop&w=500&q=60".to_string(), // Laptop logic
    };

    images::ActiveModel {
        id: Set(id),
        image_url: Set(url),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
    }.insert(db).await?;

    Ok(id)
}
