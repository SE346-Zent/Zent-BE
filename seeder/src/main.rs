use anyhow::Result;
use clap::Parser;
use dialoguer::Input;
use sea_orm::Database;
use seeder::{
    seed_account_statuses, seed_roles, seed_users,
    UserSeedConfig,
};
use serde_json::to_string_pretty;
use std::path::PathBuf;

/// Zent database seeder CLI
#[derive(Parser, Debug)]
#[command(version, about = "Seed the zent_be database with fake user data", long_about = None)]
struct Args {
    /// Database connection URL
    #[arg(short, long)]
    db_url: Option<String>,

    /// Number of users to generate
    #[arg(short, long)]
    num_users: Option<usize>,

    /// Role name to assign to every generated user (default: Customer)
    #[arg(short = 'r', long, default_value = "Customer")]
    role: String,

    /// Account status name to assign to every generated user (default: Active)
    #[arg(short = 's', long, default_value = "Active")]
    account_status: String,

    /// Random seed for reproducibility
    #[arg(long, default_value = "0")]
    rng_seed: u64,

    /// Force interactive mode — prompt for all parameters including optional ones
    #[arg(short, long)]
    interactive: bool,

    /// Write plaintext passwords to a JSON file instead of printing to STDOUT
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let (db_url, num_users, role_name, status_name, rng_seed) = if args.interactive {
        prompt_all(&args)?
    } else if args.db_url.is_none() || args.num_users.is_none() {
        prompt_missing(&args)?
    } else {
        (
            args.db_url.clone().unwrap(),
            args.num_users.unwrap(),
            args.role.clone(),
            args.account_status.clone(),
            args.rng_seed,
        )
    };

    println!("\n--- Connecting to database ---");
    let db = Database::connect(&db_url).await?;

    // -----------------------------------------------------------------------
    // Step 1: seed lookup tables
    // -----------------------------------------------------------------------
    println!("\n--- Seeding Roles ---");
    let roles = seed_roles(&db).await?;

    println!("\n--- Seeding Account Statuses ---");
    let statuses = seed_account_statuses(&db).await?;

    // Resolve names to IDs, failing fast with a helpful error on typos
    let role_id = *roles.get(&role_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown role '{}'. Valid roles: {}",
            role_name,
            seeder::role::ROLES.join(", ")
        )
    })?;

    let account_status_id = *statuses.get(&status_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown account status '{}'. Valid statuses: {}",
            status_name,
            seeder::account_status::ACCOUNT_STATUSES.join(", ")
        )
    })?;

    // -----------------------------------------------------------------------
    // Step 2: seed users (hashes passwords inline via Argon2)
    // -----------------------------------------------------------------------
    println!("\n--- Seeding Users ({}) ---", num_users);
    let records = seed_users(
        &db,
        UserSeedConfig {
            num_users,
            seed: rng_seed,
            default_account_status: account_status_id,
            default_role_id: role_id,
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Output plaintext credentials
    // -----------------------------------------------------------------------
    if !records.is_empty() {
        let json = to_string_pretty(&records)?;
        match &args.output {
            Some(path) => {
                std::fs::write(path, &json)?;
                println!("\n  Plaintext credentials written to: {}", path.display());
            }
            None => {
                println!("\n--- User Credentials (plaintext — dev only) ---");
                println!("{}", json);
            }
        }
    }

    println!("\nDone.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive prompts
// ---------------------------------------------------------------------------

fn prompt_all(args: &Args) -> Result<(String, usize, String, String, u64)> {
    let db_url: String = prompt_required("Database URL", args.db_url.clone())?;
    let num_users: usize = prompt_required("Number of users", args.num_users)?;

    let role: String = Input::new()
        .with_prompt(format!(
            "Role to assign [{}]",
            seeder::role::ROLES.join(", ")
        ))
        .default(args.role.clone())
        .interact_text()?;

    let status: String = Input::new()
        .with_prompt(format!(
            "Account status to assign [{}]",
            seeder::account_status::ACCOUNT_STATUSES.join(", ")
        ))
        .default(args.account_status.clone())
        .interact_text()?;

    let rng_seed: u64 = Input::new()
        .with_prompt("Random seed")
        .default(args.rng_seed)
        .interact_text()?;

    Ok((db_url, num_users, role, status, rng_seed))
}

fn prompt_missing(args: &Args) -> Result<(String, usize, String, String, u64)> {
    let db_url = match &args.db_url {
        Some(url) => url.clone(),
        None => Input::new().with_prompt("Database URL").interact_text()?,
    };

    let num_users = match args.num_users {
        Some(n) => n,
        None => Input::new()
            .with_prompt("Number of users")
            .interact_text()?,
    };

    Ok((
        db_url,
        num_users,
        args.role.clone(),
        args.account_status.clone(),
        args.rng_seed,
    ))
}

fn prompt_required<T>(prompt: &str, cli_value: Option<T>) -> Result<T>
where
    T: std::fmt::Display + std::str::FromStr + Clone,
    <T as std::str::FromStr>::Err: std::fmt::Debug + std::fmt::Display,
{
    let input = match cli_value {
        Some(val) => Input::new()
            .with_prompt(prompt)
            .default(val)
            .interact_text()?,
        None => Input::new().with_prompt(prompt).interact_text()?,
    };
    Ok(input)
}