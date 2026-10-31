use erp_kernel::{ErpModule, Migration, ModuleRoute};

pub struct ManufacturingModule;

impl Default for ManufacturingModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ManufacturingModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for ManufacturingModule {
    fn id(&self) -> &'static str {
        "mod-manufacturing"
    }

    fn name(&self) -> &'static str {
        "Manufacturing & Assembly"
    }

    fn description(&self) -> &'static str {
        "Bill of Materials (BOM), production planning, and atomic component-to-finished goods work orders"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["mod-inventory"]
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "create_manufacturing_tables",
                sql: r#"
                CREATE TABLE IF NOT EXISTS boms (
                    id TEXT PRIMARY KEY,
                    bom_number TEXT NOT NULL UNIQUE,
                    item_id TEXT NOT NULL,
                    name TEXT NOT NULL,
                    quantity TEXT NOT NULL DEFAULT '1',
                    routing_notes TEXT,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_boms_number ON boms(bom_number);
                CREATE INDEX IF NOT EXISTS idx_boms_item ON boms(item_id);

                CREATE TABLE IF NOT EXISTS bom_components (
                    id TEXT PRIMARY KEY,
                    bom_id TEXT NOT NULL,
                    item_id TEXT NOT NULL,
                    quantity TEXT NOT NULL,
                    scrap_percentage TEXT NOT NULL DEFAULT '0',
                    notes TEXT,
                    FOREIGN KEY(bom_id) REFERENCES boms(id)
                );
                CREATE INDEX IF NOT EXISTS idx_bomc_bom ON bom_components(bom_id);

                CREATE TABLE IF NOT EXISTS work_orders (
                    id TEXT PRIMARY KEY,
                    work_order_number TEXT NOT NULL UNIQUE,
                    bom_id TEXT NOT NULL,
                    item_id TEXT NOT NULL,
                    target_quantity TEXT NOT NULL,
                    source_location_id TEXT NOT NULL,
                    destination_location_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    completed_at TEXT,
                    FOREIGN KEY(bom_id) REFERENCES boms(id)
                );
                CREATE INDEX IF NOT EXISTS idx_wo_number ON work_orders(work_order_number);
                CREATE INDEX IF NOT EXISTS idx_wo_status ON work_orders(status);
                "#,
            },
            Migration {
                version: 2,
                name: "create_work_order_genealogy_table",
                sql: r#"
                CREATE TABLE IF NOT EXISTS work_order_genealogy (
                    id TEXT PRIMARY KEY,
                    work_order_id TEXT NOT NULL REFERENCES work_orders(id),
                    finished_item_id TEXT NOT NULL,
                    finished_lot_number TEXT NOT NULL,
                    component_item_id TEXT NOT NULL,
                    component_lot_number TEXT,
                    component_quantity TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_genealogy_wo ON work_order_genealogy(work_order_id);
                CREATE INDEX IF NOT EXISTS idx_genealogy_finished_lot ON work_order_genealogy(finished_lot_number);
                "#,
            },
        ]
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "manufacturing/boms".to_string(),
                title: "Bill of Materials".to_string(),
                icon: "build".to_string(),
                display_order: 60,
            },
            ModuleRoute {
                route_id: "manufacturing/work_orders".to_string(),
                title: "Work Orders".to_string(),
                icon: "precision_manufacturing".to_string(),
                display_order: 61,
            },
        ]
    }
}
