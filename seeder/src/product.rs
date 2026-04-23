use anyhow::Result;
use chrono::Utc;
use sea_orm::{DatabaseConnection, Set, ActiveModelTrait};
use std::collections::HashMap;
use uuid::Uuid;
use zent_be::entities::products;

/// Generates and inserts random product records into the database.
///
/// `customer_ids` must contain at least one UUID (from previously seeded users).
/// Returns the UUIDs of all inserted products for downstream seeders.
pub async fn seed_random_products(
    db: &DatabaseConnection,
    count: usize,
    _seed: u64,
    customer_ids: &[Uuid],
    product_models: &HashMap<String, String>,
) -> Result<Vec<Uuid>> {
    if customer_ids.is_empty() {
        anyhow::bail!("Cannot seed products: no customer user IDs provided.");
    }
    if product_models.is_empty() {
        anyhow::bail!("Cannot seed products: no product models found.");
    }

    let now = Utc::now();

    // Sort for deterministic picking (even if using thread_rng for other things)
    let mut model_entries: Vec<(&String, &String)> = product_models.iter().collect();
    model_entries.sort_by_key(|(name, _)| (*name).clone());

    println!("  Generating {} fake products...", count);

    let mut inserted_ids = Vec::with_capacity(count);

    use rand::seq::IndexedRandom;
    let mut rng = rand::rng();

    for i in 0..count {
        let (_, model_code) = model_entries.choose(&mut rng).unwrap();
        let &customer_id = customer_ids.choose(&mut rng).unwrap();

        let id = Uuid::new_v4();
        inserted_ids.push(id);

        use fake::Fake;
        use fake::faker::company::en::BsNoun;
        let noun: String = BsNoun().fake();
        let serial_number = format!("SN-{}-{:05}", noun.to_uppercase().replace(' ', ""), i);

        products::ActiveModel {
            id: Set(id),
            product_model_code: Set((*model_code).to_string()),
            customer_id: Set(customer_id),
            product_name: Set(format!("Lenovo {}", noun)),
            serial_number: Set(serial_number),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        }.insert(db).await?;

        // Add Product Image
        use zent_be::entities::{images, product_image_links};
        let r: u8 = rand::random();
        let url = if r % 2 == 0 {
            "https://images.unsplash.com/photo-1517336714731-489689fd1ca8?auto=format&fit=crop&w=500&q=60"
        } else {
            "https://images.unsplash.com/photo-1544117519-31a4b719223d?auto=format&fit=crop&w=500&q=60"
        };
        let img_id = Uuid::new_v4();
        images::ActiveModel {
            id: Set(img_id),
            image_url: Set(url.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        }.insert(db).await?;
        product_image_links::ActiveModel {
            image_id: Set(img_id),
            product_id: Set(id),
        }.insert(db).await?;
    }
    println!("  Successfully seeded {} products.", count);

    Ok(inserted_ids)
}
