use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use uuid::Uuid;
use rand;

use zent_be::entities::product_models;

/// Realistic Lenovo product models to seed.
pub const PRODUCT_MODELS: &[(&str, &str)] = &[
    ("IdeaPad 5 Pro 16ARH7 - Type 82SN", "82SN003JVN"),
    ("Legion 5 15IRX10 - Type 83LY", "83LY00HQVN"),
];

pub async fn seed_product_models(db: &DatabaseConnection, _seed: u64) -> Result<HashMap<String, String>> {
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
                    "  ProductModel '{}' already exists (code={})",
                    model_name, m.model_code
                );
                m.model_code
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
                    "  Created product_model '{}' (code={})",
                    model_name, inserted.model_code
                );
                
                // Add an image
                use zent_be::entities::{images, product_model_image_links};
                let r: u8 = rand::random();
                let url = if r % 2 == 0 {
                    "https://images.unsplash.com/photo-1593642632823-8f785ba67e45?auto=format&fit=crop&w=500&q=60"
                } else {
                    "https://images.unsplash.com/photo-1541807084-5c52b6b3adef?auto=format&fit=crop&w=500&q=60"
                };
                let img_id = Uuid::new_v4();
                images::ActiveModel {
                    id: Set(img_id),
                    image_url: Set(url.to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                }.insert(db).await?;
                product_model_image_links::ActiveModel {
                    image_id: Set(img_id),
                    product_model_code: Set(inserted.model_code.clone()),
                }.insert(db).await?;

                inserted.model_code
            }
        };

        map.insert(model_name.to_string(), id.clone());
    }

    Ok(map)
}
