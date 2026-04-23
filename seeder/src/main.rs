use anyhow::Result;
use clap::Parser;
use dialoguer::Input;
use sea_orm::Database;
use seeder::{
    UserSeedConfig, seed_account_statuses, seed_product_models, 
    seed_random_products, seed_random_warranties, seed_random_work_orders, seed_roles,
    seed_users, seed_work_order_closing_forms, seed_work_order_statuses,
    seed_parts_and_catalogs, seed_part_statuses, seed_work_order_symptoms, seed_part_conditions
};
use serde_json::to_string_pretty;
use std::path::PathBuf;

/// Zent database seeder CLI
#[derive(Parser, Debug)]
#[command(version, about = "Seed the zent_be database with fake data", long_about = None)]
struct Args {
    /// Database connection URL
    #[arg(short, long, env="database_url")]
    db_url: Option<String>,

    /// Number of users to generate
    #[arg(short, long)]
    num_users: Option<usize>,

    /// Number of work orders to generate
    #[arg(short, long)]
    work_orders: Option<usize>,

    /// Number of products to generate
    #[arg(short, long)]
    products: Option<usize>,

    /// Number of warranties to generate
    #[arg(long)]
    warranties: Option<usize>,

    /// Number of closing forms to generate
    #[arg(short, long)]
    closing_forms: Option<usize>,

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

    let (db_url, num_users, num_work_orders, num_products, num_warranties, num_closing_forms, rng_seed) =
        if args.interactive {
            prompt_all(&args)?
        } else if args.db_url.is_none() || args.num_users.is_none() {
            prompt_missing(&args)?
        } else {
            let mut forms = args.closing_forms.unwrap_or(0);
            let wos = args.work_orders.unwrap_or(0);
            if wos > 0 && forms == 0 {
                forms = wos; // Auto-generate forms for the work orders to satisfy `complete_form_id` FK
            }
            (
                args.db_url.clone().unwrap(),
                args.num_users.unwrap(),
                wos,
                args.products.unwrap_or(0),
                args.warranties.unwrap_or(0),
                forms,
                args.rng_seed,
            )
        };

    println!("\n--- Connecting to database ---");
    let db = Database::connect(&db_url).await?;

    // -----------------------------------------------------------------------
    // Step 1: seed lookup tables (no FK dependencies)
    // -----------------------------------------------------------------------
    println!("\n--- Seeding Roles ---");
    let roles = seed_roles(&db).await?;

    println!("\n--- Seeding Account Statuses ---");
    let statuses = seed_account_statuses(&db).await?;

    println!("\n--- Seeding Work Order Statuses ---");
    let _ = seed_work_order_statuses(&db).await?;

    println!("\n--- Seeding Work Order Symptoms ---");
    let wo_symptoms = seed_work_order_symptoms(&db).await?;

    println!("\n--- Seeding Product Models ---");
    let prod_models = seed_product_models(&db, rng_seed).await?;
    
    println!("\n--- Seeding Part Statuses ---");
    let part_statuses = seed_part_statuses(&db).await?;

    println!("\n--- Seeding Part Conditions ---");
    let part_conditions = seed_part_conditions(&db).await?;

    // -----------------------------------------------------------------------
    // Step 2: seed users FIRST (products & warranties need customer_id)
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

    // Collect user IDs for downstream seeders
    let customer_ids: Vec<uuid::Uuid> = records
        .iter()
        .filter(|r| r.role == "Customer")
        .map(|r| r.id)
        .collect();
    let technician_ids: Vec<uuid::Uuid> = records
        .iter()
        .filter(|r| r.role == "Technician")
        .map(|r| r.id)
        .collect();
    let admin_ids: Vec<uuid::Uuid> = records
        .iter()
        .filter(|r| r.role == "Admin" || r.role == "SuperAdmin")
        .map(|r| r.id)
        .collect();

    // -----------------------------------------------------------------------
    // Step 3: seed products (needs users, product_status, product_models)
    // -----------------------------------------------------------------------
    let mut product_ids = Vec::new();
    if num_products > 0 {
        println!("\n--- Seeding Products ({}) ---", num_products);
        product_ids = seed_random_products(
            &db,
            num_products,
            rng_seed,
            &customer_ids,
            &prod_models,
        )
        .await?;
        
        println!("\n--- Seeding Parts and Catalogs ---");
        seed_parts_and_catalogs(&db, &part_statuses, rng_seed).await?;
    }

    // -----------------------------------------------------------------------
    // Step 4: seed work orders
    // -----------------------------------------------------------------------
    let mut work_order_ids = Vec::new();
    if num_work_orders > 0 {
        println!("\n--- Seeding Work Orders ({}) ---", num_work_orders);
        work_order_ids = seed_random_work_orders(
            &db, 
            num_work_orders, 
            rng_seed, 
            &customer_ids,
            &technician_ids,
            &admin_ids,
            &product_ids, 
            &[], // closing forms are generated after, so pass empty
            &wo_symptoms
        ).await?;
    }

    // -----------------------------------------------------------------------
    // Step 5: seed work order closing forms
    // -----------------------------------------------------------------------
    let mut _closing_form_ids = Vec::new();
    if num_closing_forms > 0 {
        println!(
            "\n--- Seeding Work Order Closing Forms ({}) ---",
            num_closing_forms
        );
        _closing_form_ids = seed_work_order_closing_forms(&db, num_closing_forms, rng_seed, &work_order_ids, &product_ids).await?;
    }

    // -----------------------------------------------------------------------
    // Step 6: seed warranties (needs users + products)
    // -----------------------------------------------------------------------
    if num_warranties > 0 {
        println!("\n--- Seeding Warranties ({}) ---", num_warranties);
        seed_random_warranties(&db, num_warranties, rng_seed, &customer_ids, &product_ids).await?;
    }

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

fn prompt_all(args: &Args) -> Result<(String, usize, usize, usize, usize, usize, u64)> {
    let db_url: String = prompt_required("Database URL", args.db_url.clone())?;
    let num_users: usize = prompt_required("Number of users", args.num_users)?;
    let num_work_orders: usize = Input::new()
        .with_prompt("Number of work orders")
        .default(args.work_orders.unwrap_or(0))
        .interact_text()?;
    let num_products: usize = Input::new()
        .with_prompt("Number of products")
        .default(args.products.unwrap_or(0))
        .interact_text()?;
    let num_warranties: usize = Input::new()
        .with_prompt("Number of warranties")
        .default(args.warranties.unwrap_or(0))
        .interact_text()?;
    let num_closing_forms: usize = Input::new()
        .with_prompt("Number of closing forms")
        .default(args.closing_forms.unwrap_or(0))
        .interact_text()?;
    let rng_seed: u64 = Input::new()
        .with_prompt("Random seed")
        .default(args.rng_seed)
        .interact_text()?;

    Ok((
        db_url,
        num_users,
        num_work_orders,
        num_products,
        num_warranties,
        num_closing_forms,
        rng_seed,
    ))
}

fn prompt_missing(args: &Args) -> Result<(String, usize, usize, usize, usize, usize, u64)> {
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
        args.products.unwrap_or(0),
        args.warranties.unwrap_or(0),
        args.closing_forms.unwrap_or(0),
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