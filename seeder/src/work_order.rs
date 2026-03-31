use anyhow::{Context, Result};
use chrono::Utc;
use fake::{
    Fake,
    faker::{
        address::en::{BuildingNumber, CityName, CountryName, StateName, StreetName},
        internet::en::FreeEmail,
        name::en::{FirstName, LastName},
        phone_number::en::PhoneNumber,
    },
    rand::{SeedableRng, rngs::StdRng, seq::IndexedRandom},
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use zent_be::entities::{work_order_status, work_orders};

const ROLES: &[&str] = &["Technician", "Admin", "Customer", "SuperAdmin"];

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

    let records: Vec<work_orders::ActiveModel> = (0..count)
        .map(|i| {
            let status = valid_statuses
                .choose(&mut rng)
                .context("Failed to pick a random status")
                .unwrap();

            work_orders::ActiveModel {
                id: Set(Uuid::new_v4()),
                first_name: Set(FirstName().fake_with_rng(&mut rng)),
                last_name: Set(LastName().fake_with_rng(&mut rng)),
                email: Set(FreeEmail().fake_with_rng(&mut rng)),
                phone_number: Set(PhoneNumber().fake_with_rng(&mut rng)),
                work_order_status_id: Set(status.id),
                country: Set(CountryName().fake_with_rng(&mut rng)),
                state: Set(StateName().fake_with_rng(&mut rng)),
                city: Set(CityName().fake_with_rng(&mut rng)),
                address: Set(format!(
                    "{} {}",
                    BuildingNumber().fake_with_rng::<String, _>(&mut rng),
                    StreetName().fake_with_rng::<String, _>(&mut rng),
                )),
                building: Set(BuildingNumber().fake_with_rng(&mut rng)),
                appointment: Set(now),
                reference_ticket: Set(format!("REF-{:05}", i + 1)),
                created_at: Set(now),
                updated_at: Set(now),
                closed_at: Set(now),
                admin_id: Set(Uuid::new_v4()),
                customer_id: Set(Uuid::new_v4()),
                technician_id: Set(Uuid::new_v4()),
                complete_form_id: Set(Uuid::new_v4()),
                reject_reason: Set("".to_string()),
            }
        })
        .collect();

    println!("  Inserting into database...");
    work_orders::Entity::insert_many(records).exec(db).await?;

    println!("  Successfully seeded {} work orders.", count);
    Ok(())
}
