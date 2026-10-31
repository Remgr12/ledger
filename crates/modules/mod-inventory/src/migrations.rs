use erp_kernel::Migration;

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "create_inventory_tables",
            sql: r#"
            CREATE TABLE IF NOT EXISTS storage_locations (
                id TEXT PRIMARY KEY,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                parent_id TEXT REFERENCES storage_locations(id) ON DELETE RESTRICT,
                location_type TEXT NOT NULL,
                path_breadcrumb TEXT NOT NULL,
                qr_code_uri TEXT NOT NULL UNIQUE,
                map_x REAL,
                map_y REAL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS location_closure (
                ancestor_id TEXT NOT NULL REFERENCES storage_locations(id) ON DELETE CASCADE,
                descendant_id TEXT NOT NULL REFERENCES storage_locations(id) ON DELETE CASCADE,
                depth INTEGER NOT NULL,
                PRIMARY KEY (ancestor_id, descendant_id)
            );

            CREATE TABLE IF NOT EXISTS inventory_items (
                id TEXT PRIMARY KEY,
                sku TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                description TEXT,
                unit TEXT NOT NULL,
                track_serial INTEGER NOT NULL DEFAULT 0,
                track_lot INTEGER NOT NULL DEFAULT 0,
                min_stock_level TEXT NOT NULL DEFAULT '0',
                qr_code_uri TEXT NOT NULL UNIQUE,
                barcode TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stock_movements (
                id TEXT PRIMARY KEY,
                movement_type TEXT NOT NULL,
                item_id TEXT NOT NULL REFERENCES inventory_items(id),
                from_location_id TEXT REFERENCES storage_locations(id),
                to_location_id TEXT REFERENCES storage_locations(id),
                quantity TEXT NOT NULL,
                lot_number TEXT,
                serial_number TEXT,
                reference_id TEXT,
                notes TEXT,
                hlc_timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stock_quants (
                id TEXT PRIMARY KEY,
                item_id TEXT NOT NULL REFERENCES inventory_items(id),
                location_id TEXT NOT NULL REFERENCES storage_locations(id),
                quantity TEXT NOT NULL,
                lot_number TEXT,
                serial_number TEXT,
                updated_at TEXT NOT NULL,
                UNIQUE(item_id, location_id, lot_number, serial_number)
            );

            CREATE INDEX IF NOT EXISTS idx_locations_parent ON storage_locations(parent_id);
            CREATE INDEX IF NOT EXISTS idx_locations_type ON storage_locations(location_type);
            CREATE INDEX IF NOT EXISTS idx_closure_descendant ON location_closure(descendant_id);
            CREATE INDEX IF NOT EXISTS idx_items_sku ON inventory_items(sku);
            CREATE INDEX IF NOT EXISTS idx_items_barcode ON inventory_items(barcode);
            CREATE INDEX IF NOT EXISTS idx_movements_item ON stock_movements(item_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_movements_from ON stock_movements(from_location_id);
            CREATE INDEX IF NOT EXISTS idx_movements_to ON stock_movements(to_location_id);
            CREATE INDEX IF NOT EXISTS idx_quants_item_loc ON stock_quants(item_id, location_id);
            CREATE INDEX IF NOT EXISTS idx_quants_loc ON stock_quants(location_id);
            "#,
        },
        Migration {
            version: 2,
            name: "create_inventory_lots_table",
            sql: r#"
            CREATE TABLE IF NOT EXISTS inventory_lots (
                lot_number TEXT PRIMARY KEY,
                item_id TEXT NOT NULL REFERENCES inventory_items(id),
                expiration_date TEXT,
                quarantine_status TEXT NOT NULL DEFAULT 'released',
                manufacturer_batch TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_lots_item ON inventory_lots(item_id);
            "#,
        },
        Migration {
            version: 3,
            name: "create_v004_inventory_tables",
            sql: r#"
            CREATE TABLE IF NOT EXISTS serialized_assets (
                id TEXT PRIMARY KEY,
                item_id TEXT NOT NULL REFERENCES inventory_items(id),
                serial_number TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'Active',
                warranty_start_date TEXT,
                warranty_end_date TEXT,
                customer_id TEXT,
                current_location_id TEXT,
                next_calibration_date TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_serial_sn ON serialized_assets(serial_number);
            CREATE INDEX IF NOT EXISTS idx_serial_item ON serialized_assets(item_id);

            CREATE TABLE IF NOT EXISTS asset_maintenance_logs (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL REFERENCES serialized_assets(id),
                log_type TEXT NOT NULL,
                notes TEXT NOT NULL,
                performed_by TEXT NOT NULL,
                service_date TEXT NOT NULL,
                next_due_date TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_maint_asset ON asset_maintenance_logs(asset_id);

            CREATE TABLE IF NOT EXISTS rma_claims (
                id TEXT PRIMARY KEY,
                rma_number TEXT NOT NULL UNIQUE,
                asset_id TEXT NOT NULL REFERENCES serialized_assets(id),
                customer_id TEXT,
                issue_description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Open',
                resolution TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_rma_num ON rma_claims(rma_number);

            CREATE TABLE IF NOT EXISTS stock_transfer_orders (
                id TEXT PRIMARY KEY,
                transfer_number TEXT NOT NULL UNIQUE,
                origin_location_id TEXT NOT NULL REFERENCES storage_locations(id),
                destination_location_id TEXT NOT NULL REFERENCES storage_locations(id),
                in_transit_location_id TEXT NOT NULL REFERENCES storage_locations(id),
                status TEXT NOT NULL DEFAULT 'Draft',
                carrier_name TEXT,
                tracking_number TEXT,
                driver_signature TEXT,
                created_at TEXT NOT NULL,
                dispatched_at TEXT,
                received_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_transfer_num ON stock_transfer_orders(transfer_number);
            CREATE INDEX IF NOT EXISTS idx_transfer_status ON stock_transfer_orders(status);

            CREATE TABLE IF NOT EXISTS stock_transfer_lines (
                id TEXT PRIMARY KEY,
                transfer_id TEXT NOT NULL REFERENCES stock_transfer_orders(id),
                item_id TEXT NOT NULL REFERENCES inventory_items(id),
                requested_qty TEXT NOT NULL,
                dispatched_qty TEXT NOT NULL DEFAULT '0',
                received_qty TEXT NOT NULL DEFAULT '0',
                variance_qty TEXT NOT NULL DEFAULT '0',
                variance_reason TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_stl_transfer ON stock_transfer_lines(transfer_id);
            "#,
        },
    ]
}
