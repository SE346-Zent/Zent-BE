use anyhow::{Context, Result};
use chrono::Utc;
use rand::{rngs::StdRng, SeedableRng, seq::IndexedRandom};
use fake::{
    Fake,
    faker::{
        address::en::{BuildingNumber, CityName, CountryName, StateName, StreetName},
        internet::en::FreeEmail,
        name::en::{FirstName, LastName},
        phone_number::en::PhoneNumber,
    },
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use zent_be::entities::{work_order_statuses, work_orders};

/// Generates and inserts random work orders into the database.
///
/// Statuses are picked at random from whatever rows exist in `work_order_status`.
/// Pass the same `seed` you used for users to keep the full dataset reproducible.
pub async fn seed_random_work_orders(
    db: &DatabaseConnection,
    count: usize,
    seed: u64,
    customer_ids: &[Uuid],
    technician_ids: &[Uuid],
    admin_ids: &[Uuid],
    product_ids: &[Uuid],
    closing_form_ids: &[Uuid],
    work_order_symptoms: &std::collections::HashMap<String, i32>,
) -> Result<Vec<Uuid>> {
    if customer_ids.is_empty() || product_ids.is_empty() {
        // Can't seed without customers and products
        return Ok(vec![]);
    }

    let symptom_ids = work_order_symptoms.values().cloned().collect::<Vec<i32>>();

    let valid_statuses = work_order_statuses::Entity::find().all(db).await?;
    if valid_statuses.is_empty() {
        anyhow::bail!("Cannot seed work orders: no rows found in work_order_status.");
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let now = Utc::now();

    println!("  Generating {} fake work orders...", count);

    let mut inserted_ids = Vec::with_capacity(count);

    let records: Vec<work_orders::ActiveModel> = (0..count)
        .map(|_| {
            let status = valid_statuses
                .choose(&mut rng)
                .context("Failed to pick a random status")
                .unwrap();
            
            let wo_id = Uuid::new_v4();
            inserted_ids.push(wo_id);

            // Default to any user if specific role list is empty
            let admin_id = admin_ids.choose(&mut rng).unwrap_or_else(|| customer_ids.choose(&mut rng).unwrap());
            let technician_id = technician_ids.choose(&mut rng);
            let &customer_id = customer_ids.choose(&mut rng).unwrap();

            work_orders::ActiveModel {
                id: Set(wo_id),
                work_order_number: Set(wo_id.to_string()[..4].to_uppercase()),
                first_name: Set(FirstName().fake_with_rng(&mut rng)),
                last_name: Set(LastName().fake_with_rng(&mut rng)),
                email: Set(Some(FreeEmail().fake_with_rng(&mut rng))),
                phone_number: Set(Some(PhoneNumber().fake_with_rng(&mut rng))),
                work_order_status_id: Set(status.id),
                country: Set(CountryName().fake_with_rng(&mut rng)),
                state: Set(StateName().fake_with_rng(&mut rng)),
                city: Set(CityName().fake_with_rng(&mut rng)),
                address: Set(format!(
                    "{} {}",
                    BuildingNumber().fake_with_rng::<String, _>(&mut rng),
                    StreetName().fake_with_rng::<String, _>(&mut rng),
                )),
                building: Set(Some(BuildingNumber().fake_with_rng(&mut rng))),
                appointment: Set(now),
                reference_ticket_id: Set(None),
                description: Set("".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                admin_id: Set(Some(*admin_id)),
                customer_id: Set(customer_id),
                technician_id: Set(technician_id.copied()),
                complete_form_id: if closing_form_ids.is_empty() { Set(Some(Uuid::new_v4())) } else { Set(Some(*closing_form_ids.choose(&mut rng).unwrap())) },
                reject_form_id: Set(None),
                work_order_symptom_id: Set(*symptom_ids.choose(&mut rng).unwrap_or(&1)),
                product_id: Set(*product_ids.choose(&mut rng).unwrap()),
            }
        })
        .collect();

    println!("  Inserting into database...");
    work_orders::Entity::insert_many(records).exec(db).await?;

    println!("  Successfully seeded {} work orders.", count);
    Ok(inserted_ids)
}
