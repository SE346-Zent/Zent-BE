use anyhow::{Context, Result};
use chrono::Utc;
use fake::{
    Fake,
    faker::{
        address::en::{BuildingNumber, StreetName},
        lorem::en::{Paragraph, Sentence},
    },
    rand::{RngCore, SeedableRng, rngs::StdRng, seq::{IndexedRandom, SliceRandom}},
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use zent_be::entities::{work_order, work_order_status};

/// Generates and inserts random work orders into the database.
///
/// Statuses are picked at random from whatever rows exist in `work_order_status`.
/// Pass the same `seed` you used for users to keep the full dataset reproducible.
pub async fn seed_random_work_orders(
    db: &DatabaseConnection,
    count: usize,
    seed: u64,
) -> Result<()> {
    let valid_statuses = work_order_status::Entity::find().all(db).await?;
    if valid_statuses.is_empty() {
        anyhow::bail!("Cannot seed work orders: no rows found in work_order_status.");
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let now = Utc::now();

    println!("  Generating {} fake work orders...", count);

    let records: Vec<work_order::ActiveModel> = (0..count)
        .map(|_| {
            let status = valid_statuses
                .choose(&mut rng)
                .context("Failed to pick a random status")
                .unwrap();

            let address = format!(
                "{} {}",
                BuildingNumber().fake_with_rng::<String, _>(&mut rng),
                StreetName().fake_with_rng::<String, _>(&mut rng),
            );

            let priority: i32 = (rng.next_u32() % 5) as i32; // 0-4

            work_order::ActiveModel {
                id: Set(Uuid::new_v4()),
                title: Set(Sentence(3..6).fake_with_rng(&mut rng)),
                address_string: Set(address),
                status_id: Set(status.id),
                description: Set(Paragraph(1..2).fake_with_rng(&mut rng)),
                reject_reason: Set(String::new()),
                priority: Set(priority),
                created_at: Set(now),
                updated_at: Set(now),
                closed_at: Set(None),
                version: Set(1),
            }
        })
        .collect();

    println!("  Inserting into database...");
    work_order::Entity::insert_many(records).exec(db).await?;

    println!("  Successfully seeded {} work orders.", count);
    Ok(())
}