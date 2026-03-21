use anyhow::Result;
use clap::Parser;
use dialoguer::Input;
use sea_orm::Database;
use seeder::{
    UserSeedConfig, seed_account_statuses, seed_random_work_orders, seed_roles, seed_users, seed_work_order_statuses
};
use serde_json::to_string_pretty;
use std::path::PathBuf;

/// Zent database seeder CLI
#[derive(Parser, Debug)]
#[command(version, about = "Seed the zent_be database with fake data", long_about = None)]
struct Args {
    /// Database connection URL
    #[arg(short, long)]
    db_url: Option<String>,

    /// Number of users to generate
    #[arg(short, long)]
    num_users: Option<usize>,

    /// Number of work orders to generate
    #[arg(short, long)]
    work_orders: Option<usize>,

    /// Random seed for reproducibility
    #[arg(long, default_value = "0")]
    rng_seed: u64,

    /// Force interactive mode — prompt for all parameters
    #[arg(short, long)]
    interactive: bool,

    /// Write plaintext user credentials to a JSON file instead of STDOUT
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let (db_url, num_users, num_work_orders, rng_seed) = if args.interactive {
        prompt_all(&args)?
    } else if args.db_url.is_none() || args.num_users.is_none() {
        prompt_missing(&args)?
    } else {
        (
            args.db_url.clone().unwrap(),
            args.num_users.unwrap(),
            args.work_orders.unwrap_or(0),
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

    println!("\n --- Seeding Work Order Statuses ---");
    let _ = seed_work_order_statuses(&db).await?;

    // -----------------------------------------------------------------------
    // Step 2: seed work orders
    // -----------------------------------------------------------------------

    if num_work_orders > 0 {
        println!("\n--- Seeding Work Orders ({}) ---", num_work_orders);
        seed_random_work_orders(&db, num_work_orders, rng_seed).await?;
    }

    // -----------------------------------------------------------------------
    // Step 3: seed users — role and account_status assigned randomly per user
    // -----------------------------------------------------------------------
    println!("\n--- Seeding Users ({}) ---", num_users);
    let records = seed_users(
        &db,
        UserSeedConfig {
            num_users,
            seed: rng_seed,
            roles,
            account_statuses: statuses,
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

fn prompt_all(args: &Args) -> Result<(String, usize, usize, u64)> {
    let db_url: String = prompt_required("Database URL", args.db_url.clone())?;
    let num_users: usize = prompt_required("Number of users", args.num_users)?;
    let num_work_orders: usize = Input::new()
        .with_prompt("Number of work orders")
        .default(args.work_orders.unwrap_or(0))
        .interact_text()?;
    let rng_seed: u64 = Input::new()
        .with_prompt("Random seed")
        .default(args.rng_seed)
        .interact_text()?;

    Ok((db_url, num_users, num_work_orders, rng_seed))
}

fn prompt_missing(args: &Args) -> Result<(String, usize, usize, u64)> {
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
        args.work_orders.unwrap_or(0),
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