use anyhow::Result;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use uuid::Uuid;
use zent_be::services::v1::inventory::ports::ZeusInventoryClient;

/// Generates and inserts random product records into the database.
///
/// `customer_ids` must contain at least one UUID (from previously seeded users).
/// Returns the UUIDs of all inserted products for downstream seeders.
pub async fn seed_random_products(
    _db: &DatabaseConnection,
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

    let base_url = std::env::var("ZEUS_BASE_URL")
        .map_err(|_| anyhow::anyhow!("ZEUS_BASE_URL is required for product seeding"))?;
    let api_key = std::env::var("ZEUS_API_KEY")
        .map_err(|_| anyhow::anyhow!("ZEUS_API_KEY is required for product seeding"))?;
    let zeus_client = zent_be::infrastructure::clients::zeus::ZeusClient::new(base_url, api_key);

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

        use fake::Fake;
        use fake::faker::company::en::BsNoun;
        let noun: String = BsNoun().fake();
        let serial_number = format!("SN-{}-{:05}", noun.to_uppercase().replace(' ', ""), i);
        let created = zeus_client
            .create_product(
                model_code,
                customer_id,
                &format!("Lenovo {}", noun),
                &serial_number,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create product in SCM: {}", e))?;
        inserted_ids.push(created.id);
    }
    println!("  Successfully seeded {} products in SCM.", count);

    Ok(inserted_ids)
}
