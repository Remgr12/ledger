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
            Migration {
                version: 3,
                name: "create_workstations_and_routing_tables",
                sql: r#"
                CREATE TABLE IF NOT EXISTS workstations (
                    id TEXT PRIMARY KEY,
                    code TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    description TEXT,
                    hourly_rate TEXT NOT NULL DEFAULT '0.0',
                    daily_capacity_hours REAL NOT NULL DEFAULT 8.0,
                    status TEXT NOT NULL DEFAULT 'Active'
                );
                CREATE INDEX IF NOT EXISTS idx_ws_code ON workstations(code);

                CREATE TABLE IF NOT EXISTS routing_operations (
                    id TEXT PRIMARY KEY,
                    bom_id TEXT NOT NULL REFERENCES boms(id),
                    workstation_id TEXT NOT NULL REFERENCES workstations(id),
                    operation_name TEXT NOT NULL,
                    sequence_order INTEGER NOT NULL,
                    setup_time_mins INTEGER NOT NULL DEFAULT 0,
                    run_time_mins_per_unit INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_ro_bom ON routing_operations(bom_id);

                CREATE TABLE IF NOT EXISTS work_order_allocations (
                    id TEXT PRIMARY KEY,
                    work_order_id TEXT NOT NULL REFERENCES work_orders(id),
                    routing_operation_id TEXT NOT NULL REFERENCES routing_operations(id),
                    workstation_id TEXT NOT NULL REFERENCES workstations(id),
                    scheduled_start TEXT NOT NULL,
                    scheduled_end TEXT NOT NULL,
                    duration_hours REAL NOT NULL,
                    status TEXT NOT NULL DEFAULT 'Scheduled'
                );
                CREATE INDEX IF NOT EXISTS idx_woa_workstation ON work_order_allocations(workstation_id);
                CREATE INDEX IF NOT EXISTS idx_woa_wo ON work_order_allocations(work_order_id);
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
