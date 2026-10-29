use clap::Parser;
use erp_ffi::init_erp;
use erp_server::{create_router, seed_demo_data};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ledger-server")]
#[command(about = "Ledger ERP - Headless Server & Web Console for Sole Proprietors and SMBs")]
struct Args {
    /// Bind address and port for the HTTP/REST server and Web Console
    #[arg(short, long, env = "LEDGER_BIND_ADDR", default_value = "0.0.0.0:8080")]
    bind: String,

    /// Path to local SQLite database file
    #[arg(short, long, env = "LEDGER_DB_PATH", default_value = "ledger.db")]
    db: PathBuf,

    /// Seed realistic demo data (warehouse locations, items, POs, BOMs, projects) if empty
    #[arg(long, default_value_t = false)]
    seed: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let db_path_str = args.db.to_string_lossy().to_string();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("⚡ LEDGER ERP — LOCAL-FIRST HEADLESS SERVER");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Database Engine : SQLite WAL at {}", db_path_str);
    println!("Binding Address : http://{}", args.bind);

    let engine = init_erp(db_path_str)?;

    // Ensure all 7 modules are activated
    for module_id in &[
        "mod-inventory",
        "mod-contacts",
        "mod-invoicing",
        "mod-accounting",
        "mod-purchasing",
        "mod-manufacturing",
        "mod-projects",
    ] {
        let _ = engine.set_module_enabled(module_id.to_string(), true);
    }

    // Seed demo records if requested or if database has no storage locations
    if args.seed || engine.get_storage_locations().map(|l| l.is_empty()).unwrap_or(false) {
        println!("Seeding initial warehouse locations, BOMs, and demo inventory...");
        if let Err(e) = seed_demo_data(&engine) {
            eprintln!("Notice: Demo data seed skipped or partially applied: {}", e);
        }
    }

    let router = create_router(engine);

    let listener = tokio::net::TcpListener::bind(&args.bind).await?;
    println!("\n✅ Server is live and ready:");
    println!("  • Web Console SPA    : http://{}/", args.bind);
    println!("  • REST API Health    : http://{}/api/v1/health", args.bind);
    println!("  • HLC Sync Relay     : http://{}/api/v1/sync/pull", args.bind);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    axum::serve(listener, router).await?;

    Ok(())
}
