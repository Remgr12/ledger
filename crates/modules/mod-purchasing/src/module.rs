use erp_kernel::{ErpModule, Migration, ModuleRoute};

pub struct PurchasingModule;

impl Default for PurchasingModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PurchasingModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for PurchasingModule {
    fn id(&self) -> &'static str {
        "mod-purchasing"
    }

    fn name(&self) -> &'static str {
        "Purchasing & Supplier Orders"
    }

    fn description(&self) -> &'static str {
        "Purchase orders, supplier shipments, goods receipts with instant warehouse bin putaway"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["mod-contacts", "mod-inventory"]
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "create_purchasing_tables",
                sql: r#"
                CREATE TABLE IF NOT EXISTS purchase_orders (
                    id TEXT PRIMARY KEY,
                    po_number TEXT NOT NULL UNIQUE,
                    supplier_id TEXT NOT NULL,
                    order_date TEXT NOT NULL,
                    expected_delivery TEXT,
                    status TEXT NOT NULL,
                    subtotal TEXT NOT NULL,
                    tax_total TEXT NOT NULL,
                    grand_total TEXT NOT NULL,
                    notes TEXT,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_po_number ON purchase_orders(po_number);
                CREATE INDEX IF NOT EXISTS idx_po_supplier ON purchase_orders(supplier_id);

                CREATE TABLE IF NOT EXISTS purchase_order_lines (
                    id TEXT PRIMARY KEY,
                    purchase_order_id TEXT NOT NULL,
                    item_id TEXT NOT NULL,
                    description TEXT NOT NULL,
                    quantity TEXT NOT NULL,
                    received_quantity TEXT NOT NULL DEFAULT '0',
                    unit_price TEXT NOT NULL,
                    tax_rate TEXT NOT NULL DEFAULT '0',
                    line_total TEXT NOT NULL,
                    FOREIGN KEY(purchase_order_id) REFERENCES purchase_orders(id)
                );
                CREATE INDEX IF NOT EXISTS idx_pol_po ON purchase_order_lines(purchase_order_id);

                CREATE TABLE IF NOT EXISTS purchase_receipts (
                    id TEXT PRIMARY KEY,
                    receipt_number TEXT NOT NULL UNIQUE,
                    purchase_order_id TEXT,
                    supplier_id TEXT NOT NULL,
                    destination_location_id TEXT NOT NULL,
                    received_at TEXT NOT NULL,
                    notes TEXT,
                    FOREIGN KEY(purchase_order_id) REFERENCES purchase_orders(id)
                );
                CREATE INDEX IF NOT EXISTS idx_prec_number ON purchase_receipts(receipt_number);
                CREATE INDEX IF NOT EXISTS idx_prec_supplier ON purchase_receipts(supplier_id);

                CREATE TABLE IF NOT EXISTS purchase_receipt_lines (
                    id TEXT PRIMARY KEY,
                    receipt_id TEXT NOT NULL,
                    item_id TEXT NOT NULL,
                    quantity TEXT NOT NULL,
                    lot_number TEXT,
                    serial_number TEXT,
                    FOREIGN KEY(receipt_id) REFERENCES purchase_receipts(id)
                );
                CREATE INDEX IF NOT EXISTS idx_precl_receipt ON purchase_receipt_lines(receipt_id);
                "#,
            },
        ]
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "purchasing/orders".to_string(),
                title: "Purchase Orders".to_string(),
                icon: "shopping_cart".to_string(),
                display_order: 50,
            },
            ModuleRoute {
                route_id: "purchasing/receipts".to_string(),
                title: "Goods Receipts".to_string(),
                icon: "move_to_inbox".to_string(),
                display_order: 51,
            },
        ]
    }
}
