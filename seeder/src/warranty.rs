use anyhow::Result;
use chrono::{Duration, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use zent_be::entities::warranties;

const WARRANTY_STATUSES: &[&str] = &["Active", "Expired", "Voided"];

/// Generates and inserts random warranty records.
///
/// Each warranty references a random product and customer from the provided lists.
/// The warranty must belong to a customer **and** product as stated in the TODO.
pub async fn seed_random_warranties(
    db: &DatabaseConnection,
    count: usize,
    _seed: u64,
    customer_ids: &[Uuid],
    product_ids: &[Uuid],
) -> Result<()> {
    if customer_ids.is_empty() {
        anyhow::bail!("Cannot seed warranties: no customer user IDs provided.");
    }
    if product_ids.is_empty() {
        anyhow::bail!("Cannot seed warranties: no product IDs provided.");
    }

    let now = Utc::now();

    println!("  Generating {} fake warranties...", count);

    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let &customer_id = customer_ids.choose(&mut rng).unwrap();
        let &product_id = product_ids.choose(&mut rng).unwrap();
        let &status = WARRANTY_STATUSES.choose(&mut rng).unwrap();

        // Start date: somewhere between 2 years ago and now
        let days_ago: i64 = (rand::random::<u32>() % 730) as i64;
        let start_date = now - Duration::days(days_ago);

        // End date: 1-3 years after start, or None for "Active" warranties still running
        let end_date = if status == "Active" {
            None
        } else {
            let warranty_years: i64 = ((rand::random::<u32>() % 3) + 1) as i64;
            Some(start_date + Duration::days(warranty_years * 365))
        };
        
        records.push(warranties::ActiveModel {
            id: Set(Uuid::new_v4()),
            customer_id: Set(customer_id),
            product_id: Set(product_id),
            start_date: Set(start_date),
            end_date: Set(end_date.unwrap_or(start_date + Duration::days(365))),
            warranty_status: Set(status.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        });
    }

    println!("  Inserting into database...");
    warranties::Entity::insert_many(records).exec(db).await?;
    println!("  Successfully seeded {} warranties.", count);

    Ok(())
}
