use anyhow::Result;
use chrono::{Duration, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait};
use uuid::Uuid;
use zent_be::entities::warranties;
use zent_be::services::v1::inventory::ports::ZeusInventoryClient;

/// Generates and inserts random warranty records.
///
/// Each warranty references a random product and customer from the provided lists.
/// The warranty must belong to a customer **and** product as stated in the TODO.
pub async fn seed_random_warranties(
    db: &DatabaseConnection,
    _count: usize,
    _seed: u64,
    customer_ids: &[Uuid],
    _product_ids: &[Uuid],
    warranty_statuses_map: &std::collections::HashMap<String, i32>,
) -> Result<()> {
    if customer_ids.is_empty() {
        anyhow::bail!("Cannot seed warranties: no customer user IDs provided.");
    }
    let base_url = match std::env::var("ZEUS_BASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("  [Warning] ZEUS_BASE_URL is not set. Skipping warranty seeding.");
            return Ok(());
        }
    };
    let api_key = match std::env::var("ZEUS_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("  [Warning] ZEUS_API_KEY is not set. Skipping warranty seeding.");
            return Ok(());
        }
    };
    let zeus_client = zent_be::infrastructure::clients::zeus::ZeusClient::new(base_url, api_key);

    let mut scm_products = match zeus_client.list_products().await {
        Ok(products) => products,
        Err(e) => {
            println!("  [Warning] Failed to list SCM products: {}. Skipping warranty seeding.", e);
            return Ok(());
        }
    };
    if scm_products.is_empty() {
        println!("  [Warning] No products found in SCM. Skipping warranty seeding.");
        return Ok(());
    }

    let now = Utc::now();

    println!("  Generating warranty map 1-to-1 for {} SCM products...", scm_products.len());

    use rand::seq::IndexedRandom;
    let mut rng = rand::rng();

    let warranty_status_names: Vec<&str> = ["Active", "Expired", "Voided"]
        .into_iter()
        .filter(|name| warranty_statuses_map.contains_key(*name))
        .collect();
    if warranty_status_names.is_empty() {
        anyhow::bail!("Cannot seed warranties: no warranty status records found.");
    }

    let mut records = Vec::with_capacity(scm_products.len());
    for prod in &mut scm_products {
        // Ensure each SCM product points to a valid Zent customer.
        if !customer_ids.contains(&prod.customer_id) {
            let &new_customer_id = customer_ids.choose(&mut rng).unwrap();
            let updated = zeus_client
                .update_product(
                    prod.id,
                    &prod.product_model_code,
                    new_customer_id,
                    &prod.product_name,
                    &prod.serial_number,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to update SCM product {}: {}", prod.id, e))?;
            prod.customer_id = updated.customer_id;
        }

        // Skip if a warranty for this product already exists.
        let exists = warranties::Entity::find()
            .filter(warranties::Column::ProductId.eq(prod.id))
            .one(db)
            .await?;
        if exists.is_some() {
            continue;
        }

        let status = *warranty_status_names.choose(&mut rng).unwrap();
        let status_id = *warranty_statuses_map
            .get(status)
            .ok_or_else(|| anyhow::anyhow!("Missing warranty status id for {}", status))?;

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
            customer_id: Set(prod.customer_id),
            product_id: Set(prod.id),
            start_date: Set(start_date),
            end_date: Set(end_date.unwrap_or(start_date + Duration::days(365))),
            warranty_status: Set(status.to_string()),
            warranty_status_id: Set(Some(status_id)),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        });
    }

    if !records.is_empty() {
        println!("  Inserting into database...");
        warranties::Entity::insert_many(records).exec(db).await?;
    }
    println!("  Warranty sync completed for SCM products.");

    Ok(())
}
