use anyhow::Result;
use chrono::Utc;
use fake::{
    Fake,
    faker::lorem::en::Sentence,
    rand::{SeedableRng, rngs::StdRng},
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use zent_be::entities::work_order_closing_forms;

/// Generates and inserts random work order closing form records.
/// Returns the UUIDs of all inserted forms.
pub async fn seed_work_order_closing_forms(
    db: &DatabaseConnection,
    count: usize,
    seed: u64,
) -> Result<Vec<Uuid>> {
    let mut rng = StdRng::seed_from_u64(seed);
    let now = Utc::now();

    println!("  Generating {} fake work order closing forms...", count);

    let mut inserted_ids = Vec::with_capacity(count);

    let records: Vec<work_order_closing_forms::ActiveModel> = (0..count)
        .map(|i| {
            let id = Uuid::new_v4();
            inserted_ids.push(id);

            work_order_closing_forms::ActiveModel {
                id: Set(id),
                work_order_counting: Set(format!("WO-{:05}", i + 1)),
                mtm: Set(format!("MTM-{:04}-{:04}", (i % 9999) + 1, (i * 7 % 9999) + 1)),
                serial_number: Set(format!("SN-CF-{:06}", i + 1)),
                diagnosis: Set(Sentence(5..12).fake_with_rng(&mut rng)),
                created_at: Set(now),
                updated_at: Set(now),
            }
        })
        .collect();

    println!("  Inserting into database...");
    work_order_closing_forms::Entity::insert_many(records)
        .exec(db)
        .await?;
    println!(
        "  Successfully seeded {} work order closing forms.",
        count
    );

    Ok(inserted_ids)
}
