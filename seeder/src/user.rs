use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use fake::{
    Fake,
    faker::{
        internet::en::{FreeEmail, Password},
        name::en::Name,
        phone_number::en::PhoneNumber,
    },
    rand::{SeedableRng, rngs::StdRng, seq::IndexedRandom},
};
use rayon::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;
use zent_be::entities::users;

/// Configuration for user seeding.
pub struct UserSeedConfig {
    /// How many users to generate.
    pub num_users: usize,
    /// RNG seed for reproducibility.
    pub seed: u64,
    /// All available role name -> id pairs. Each user gets one at random.
    pub roles: HashMap<String, i32>,
    /// All available account_status name -> id pairs. Each user gets one at random.
    pub account_statuses: HashMap<String, i32>,
}

/// Plaintext record exported after seeding — dev/test use only.
#[derive(Debug, Clone, Serialize)]
pub struct UserRecord {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub password: String,
    pub role: String,
    pub account_status: String,
}

// Internal helper before hashing
struct UserInput {
    id: Uuid,
    full_name: String,
    email: String,
    phone_number: String,
    password: String,
    role_id: i32,
    role_name: String,
    account_status_id: i32,
    account_status_name: String,
}

// Internal helper after hashing
struct HashedUser {
    id: Uuid,
    full_name: String,
    email: String,
    phone_number: String,
    password: String,
    password_hash: String,
    role_id: i32,
    role_name: String,
    account_status_id: i32,
    account_status_name: String,
}

/// Seed the `user` table. Each user is assigned a random role and account
/// status drawn from the maps supplied in `config`.
/// Returns plaintext `UserRecord`s for every inserted user.
pub async fn seed_users(
    db: &DatabaseConnection,
    config: UserSeedConfig,
) -> Result<Vec<UserRecord>> {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let now = Utc::now();

    // Flatten maps into sorted vecs so SliceRandom can pick from them
    // deterministically (HashMap iteration order is not stable).
    let mut role_entries: Vec<(String, i32)> = config.roles.into_iter().collect();
    role_entries.sort_by_key(|(name, _)| name.clone());

    let mut status_entries: Vec<(String, i32)> = config.account_statuses.into_iter().collect();
    status_entries.sort_by_key(|(name, _)| name.clone());

    // Phase 1: generate plaintext data (deterministic, single-threaded)
    let inputs: Vec<UserInput> = (0..config.num_users)
        .map(|i| {
            let full_name: String = Name().fake_with_rng(&mut rng);
            let base_email: String = FreeEmail().fake_with_rng(&mut rng);
            // Suffix with index to satisfy the unique constraint.
            let email = {
                let (local, domain) = base_email
                    .split_once('@')
                    .unwrap_or((&base_email, "example.com"));
                format!("{}+{}@{}", local, i, domain)
            };
            let phone_number: String = PhoneNumber().fake_with_rng(&mut rng);
            let password: String = Password(12..16).fake_with_rng(&mut rng);

            let (role_name, role_id) = role_entries.choose(&mut rng).unwrap();
            let (status_name, status_id) = status_entries.choose(&mut rng).unwrap();

            UserInput {
                id: Uuid::new_v4(),
                full_name,
                email,
                phone_number,
                password,
                role_id: *role_id,
                role_name: role_name.clone(),
                account_status_id: *status_id,
                account_status_name: status_name.clone(),
            }
        })
        .collect();

    // Phase 2: parallel Argon2 hashing (CPU-bound)
    let hashed: Result<Vec<HashedUser>> = inputs
        .into_par_iter()
        .map(|input| {
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = Argon2::default()
                .hash_password(input.password.as_bytes(), &salt)
                .map_err(|e| anyhow::anyhow!("Argon2 error: {}", e))?
                .to_string();

            Ok(HashedUser {
                id: input.id,
                full_name: input.full_name,
                email: input.email,
                phone_number: input.phone_number,
                password: input.password,
                password_hash,
                role_id: input.role_id,
                role_name: input.role_name,
                account_status_id: input.account_status_id,
                account_status_name: input.account_status_name,
            })
        })
        .collect();

    let hashed = hashed?;

    // Phase 3: build ActiveModels and plaintext export records
    let (models, records): (Vec<_>, Vec<_>) = hashed
        .into_iter()
        .map(|h| {
            let model = users::ActiveModel {
                id: Set(h.id),
                full_name: Set(h.full_name.clone()),
                email: Set(h.email.clone()),
                password_hash: Set(h.password_hash),
                phone_number: Set(h.phone_number),
                account_status: Set(h.account_status_id),
                role_id: Set(h.role_id),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
            };

            let record = UserRecord {
                id: h.id,
                full_name: h.full_name,
                email: h.email,
                password: h.password,
                role: h.role_name,
                account_status: h.account_status_name,
            };

            (model, record)
        })
        .unzip();

    // Phase 4: bulk insert
    if !models.is_empty() {
        users::Entity::insert_many(models).exec(db).await?;
    }

    let total = users::Entity::find().count(db).await?;
    println!("  Inserted {} users. Total in DB: {}", records.len(), total);

    Ok(records)
}
