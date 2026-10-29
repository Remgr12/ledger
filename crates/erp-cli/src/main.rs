use clap::{Parser, Subcommand};
use erp_ffi::{init_erp, LocationNodeDto};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ledger-cli")]
#[command(about = "Ledger ERP - Local-First Offline Core Engine CLI", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ledger.db")]
    db: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize local database and seed with demo spatial warehouse data
    Init,

    /// Instant "Where Is It?" search for SKU, barcode, or lot
    Locate {
        query: String,
    },

    /// Test-scan a QR code or GS1 barcode payload
    Scan {
        payload: String,
        #[arg(short, long)]
        context_location: Option<String>,
    },

    /// Display hierarchical storage tree
    Tree,

    /// Generate an offline SVG thermal label
    Label {
        sku: String,
        #[arg(short, long, default_value = "ThermalRoll62mm")]
        format: String,
    },

    /// Run the headless ERP server and embedded web console
    Server {
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        bind: String,

        #[arg(long, default_value_t = false)]
        seed: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db_path = cli.db.to_string_lossy().to_string();

    let engine = init_erp(db_path.clone())?;

    // Ensure core modules are active
    let _ = engine.set_module_enabled("mod-inventory".into(), true);
    let _ = engine.set_module_enabled("mod-contacts".into(), true);
    let _ = engine.set_module_enabled("mod-invoicing".into(), true);

    match cli.command {
        Commands::Init => {
            println!("Seeding realistic warehouse spatial hierarchy...");

            let wh = engine.create_location(
                "WH-MAIN".into(),
                "Main Facility".into(),
                None,
                "Warehouse".into(),
                Some(100.0),
                Some(80.0),
            )?;

            let room = engine.create_location(
                "WS-01".into(),
                "Workshop".into(),
                Some(wh.id.clone()),
                "Room".into(),
                Some(140.0),
                Some(110.0),
            )?;

            let rack = engine.create_location(
                "RCK-02".into(),
                "Rack 2 (North Wall)".into(),
                Some(room.id.clone()),
                "Rack".into(),
                Some(160.0),
                Some(120.0),
            )?;

            let bin = engine.create_location(
                "BIN-04".into(),
                "Blue Bin #4".into(),
                Some(rack.id.clone()),
                "Bin".into(),
                Some(165.0),
                Some(125.0),
            )?;

            println!("Created: {}", bin.path_breadcrumb);

            let item1 = engine.create_item(
                "M8-HEX-BOLT-316".into(),
                "M8x40mm Hex Bolt Stainless 316".into(),
                Some("Grade 316 A4 Marine".into()),
                "pcs".into(),
                false,
                true,
                "50".into(),
                Some("07350012345678".into()),
            )?;

            let item2 = engine.create_item(
                "BEARING-608-2RS".into(),
                "Deep Groove Ball Bearing 608-2RS".into(),
                Some("Rubber sealed".into()),
                "pcs".into(),
                false,
                false,
                "20".into(),
                None,
            )?;

            engine.move_stock(
                item1.id.clone(),
                None,
                Some(bin.id.clone()),
                "142".into(),
                "Receive".into(),
                Some("LOT-2026-Q1".into()),
                Some("Vendor batch receive".into()),
            )?;

            engine.move_stock(
                item2.id.clone(),
                None,
                Some(bin.id.clone()),
                "38".into(),
                "Receive".into(),
                None,
                Some("Initial stock".into()),
            )?;

            println!("Database initialized at '{}' with 2 parts and 180 total units.", db_path);
        }

        Commands::Locate { query } => {
            let results = engine.search_item_locations(query)?;
            if results.is_empty() {
                println!("No items matched query.");
            } else {
                println!("{:<20} {:<30} {:<10} {:<40}", "SKU", "ITEM NAME", "QTY", "STORAGE BREADCRUMB");
                println!("{}", "-".repeat(105));
                for r in results {
                    println!(
                        "{:<20} {:<30} {:<10} {:<40}",
                        r.sku,
                        r.item_name,
                        format!("{} {}", r.quantity, r.unit),
                        r.breadcrumb_path
                    );
                }
            }
        }

        Commands::Scan { payload, context_location } => {
            let res = engine.resolve_qr_scan(payload, context_location)?;
            println!("Scan Resolution: {:?}", res.resolution_type);
            println!("Message: {}", res.prompt_message);
            if let Some(loc) = res.location {
                println!("Location: {} [{}] ({})", loc.name, loc.code, loc.path_breadcrumb);
            }
            if let Some(item) = res.item {
                println!("Item: {} (SKU: {})", item.name, item.sku);
            }
            if let Some(qty) = res.current_quantity_at_location {
                println!("Current Qty at Location: {}", qty);
            }
            if let Some(act) = res.suggested_action {
                println!("Suggested Rapid Action: {}", act);
            }
        }

        Commands::Tree => {
            let tree = engine.get_location_tree()?;
            println!("Storage Location Hierarchy:");
            for node in tree {
                print_tree_node(&node, 0);
            }
        }

        Commands::Label { sku, format } => {
            let matches = engine.search_item_locations(sku.clone())?;
            let (title, subtitle) = if let Some(m) = matches.first() {
                (m.sku.clone(), m.item_name.clone())
            } else {
                (sku.clone(), "Catalog Item".to_string())
            };

            let uri = format!("erp://item/{}", sku);
            let svg = engine.generate_formatted_label_svg(
                format,
                uri,
                title,
                subtitle,
                Some("Offline Vector Thermal Label".into()),
            )?;

            println!("{}", svg);
        }

        Commands::Server { bind, seed } => {
            println!("Starting Ledger ERP Headless Server on http://{}...", bind);
            for mod_id in &[
                "mod-inventory",
                "mod-contacts",
                "mod-invoicing",
                "mod-accounting",
                "mod-purchasing",
                "mod-manufacturing",
                "mod-projects",
            ] {
                let _ = engine.set_module_enabled(mod_id.to_string(), true);
            }
            if seed {
                let _ = erp_server::seed_demo_data(&engine);
            }
            let router = erp_server::create_router(engine);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(&bind).await?;
                println!("Ledger Headless Server is listening on http://{}", bind);
                axum::serve(listener, router).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            })?;
        }
    }

    Ok(())
}

fn print_tree_node(node: &LocationNodeDto, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{} [{}] ({})", indent, node.location.name, node.location.code, node.location.location_type);
    for child in &node.children {
        print_tree_node(child, depth + 1);
    }
}
