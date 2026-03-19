use anyhow::Result;
use chrono::Utc;
use fake::{
    Fake,
    faker::{internet::en::FreeEmail, name::en::Name, phone_number::en::PhoneNumber},
    rand::SeedableRng,
    rand::rngs::StdRng,
};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use uuid::Uuid;
use zent_be::entities::user;

/// Configuration for user seeding.
pub struct UserSeedConfig {
    /// How many users to generate.
    pub num_users: usize,
    /// RNG seed for reproducibility.
    pub seed: u64,
    /// Default `account_status` id to assign (must already exist in the DB).
    pub default_account_status: i32,
    /// Default `role_id` to assign (must already exist in the DB).
    pub default_role_id: i32,
}

/// Seed the `user` table with fake data.
///
/// Passwords are left as a placeholder (`"PENDING_HASH"`) at this stage;
/// the real Argon2 hash is filled in by `seed_credentials` once the
/// credential rows are created.
///
/// Returns the list of inserted user IDs.
pub async fn seed_users(
    db: &DatabaseConnection,
    config: UserSeedConfig,
) -> Result<Vec<Uuid>> {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let now = Utc::now();

    let models: Vec<user::ActiveModel> = (0..config.num_users)
        .map(|i| {
            let full_name: String = Name().fake_with_rng(&mut rng);
            let base_email: String = FreeEmail().fake_with_rng(&mut rng);
            // Append index to guarantee uniqueness across large batches.
            let email = {
                let (local, domain) = base_email.split_once('@').unwrap_or((&base_email, "example.com"));
                format!("{}+{}@{}", local, i, domain)
            };
            let phone_number: String = PhoneNumber().fake_with_rng(&mut rng);

            user::ActiveModel {
                id: Set(Uuid::new_v4()),
                full_name: Set(full_name),
                email: Set(email),
                // Real hash inserted later by seed_credentials.
                password_hash: Set("PENDING_HASH".to_string()),
                phone_number: Set(phone_number),
                account_status: Set(config.default_account_status),
                role_id: Set(config.default_role_id),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
            }
        })
        .collect();

    let ids: Vec<Uuid> = models
        .iter()
        .map(|m| *m.id.as_ref())
        .collect();

    if !models.is_empty() {
        user::Entity::insert_many(models).exec(db).await?;
    }

    let total = user::Entity::find().count(db).await?;
    println!(
        "Inserted {} users. Total users in DB: {}",
        ids.len(),
        total
    );

    Ok(ids)
}