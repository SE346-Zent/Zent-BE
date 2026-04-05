use anyhow::Result;
use chrono::Utc;
use fake::{
    Fake,
    faker::company::en::BsNoun,
    rand::{SeedableRng, rngs::StdRng, seq::IndexedRandom},
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
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
    seed: u64,
    customer_ids: &[Uuid],
    product_models: &HashMap<String, i32>,
) -> Result<Vec<Uuid>> {
    if customer_ids.is_empty() {
        anyhow::bail!("Cannot seed products: no customer user IDs provided.");
    }
    if product_models.is_empty() {
        anyhow::bail!("Cannot seed products: no product models found.");
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let now = Utc::now();

    // Sort for deterministic picking
    let mut model_entries: Vec<(&String, &i32)> = product_models.iter().collect();
    model_entries.sort_by_key(|(name, _)| name.clone());

    println!("  Generating {} fake products...", count);

    let mut inserted_ids = Vec::with_capacity(count);

    let records: Vec<products::ActiveModel> = (0..count)
        .map(|i| {
            let (_, model_id) = model_entries.choose(&mut rng).unwrap();
            let &customer_id = customer_ids.choose(&mut rng).unwrap();

            let id = Uuid::new_v4();
            inserted_ids.push(id);

            let noun: String = BsNoun().fake_with_rng(&mut rng);
            let serial_number = format!("SN-{}-{:05}", noun.to_uppercase().replace(' ', ""), i);

            products::ActiveModel {
                id: Set(id),
                model_id: Set(**model_id),
                customer_id: Set(customer_id),
                serial_number: Set(serial_number),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
            }
        })
        .collect();

    println!("  Inserting into database...");
    products::Entity::insert_many(records).exec(db).await?;
    println!("  Successfully seeded {} products.", count);

    Ok(inserted_ids)
}
