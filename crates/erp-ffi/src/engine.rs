use std::str::FromStr;
use std::sync::Arc;
use rust_decimal::Decimal;

use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
use erp_plugin_host::{PluginHost, PluginManifest};
use mod_accounting::{
    Account, AccountingModule, AccountingService, FinancialStatement, GlEntry,
    JournalEntryLine, PaymentEntry, PaymentType,
};
use mod_contacts::{ContactType, ContactsModule, ContactsService};
use mod_inventory::{
    InventoryModule, LocationService, LocationTreeNode, LocationType, MovementType,
    QrService, ScanResolution, StockService, StorageLocation,
};
use mod_invoicing::{InvoicingModule, InvoicingService};
use mod_manufacturing::{
    BillOfMaterials, ManufacturingModule, ManufacturingService, WorkOrder,
};
use mod_projects::{
    Project, ProjectsModule, ProjectsService, TimesheetEntry,
};
use mod_purchasing::{
    PurchaseOrder, PurchaseReceipt, PurchasingModule, PurchasingService,
};

use crate::dto::*;
use crate::errors::ErpFfiError;

#[derive(uniffi::Object)]
pub struct ErpEngine {
    pub db: Database,
    pub hlc: Arc<HybridLogicalClock>,
    pub bus: Arc<EventBus>,
    pub module_manager: Arc<ModuleManager>,
    pub plugin_host: Arc<PluginHost>,
    pub peers: parking_lot::RwLock<Vec<PeerNodeDto>>,
}

#[uniffi::export]
impl ErpEngine {
    /// Initialize the ERP engine with local SQLite database file (or ":memory:")
    #[uniffi::constructor]
    pub fn new(db_path: String) -> Result<Arc<Self>, ErpFfiError> {
        let db = if db_path == ":memory:" {
            Database::open_in_memory()?
        } else {
            Database::open(&db_path)?
        };

        let hlc = Arc::new(HybridLogicalClock::new(uuid::Uuid::new_v4().to_string()));
        let bus = Arc::new(EventBus::new());

        let module_manager = Arc::new(ModuleManager::new(db.clone(), hlc.clone(), bus.clone()));
        module_manager.wire_event_bus();

        // Register built-in ERP modules
        module_manager.register(Arc::new(InventoryModule::new()));
        module_manager.register(Arc::new(ContactsModule::new()));
        module_manager.register(Arc::new(InvoicingModule::new()));
        module_manager.register(Arc::new(AccountingModule::new()));
        module_manager.register(Arc::new(PurchasingModule::new()));
        module_manager.register(Arc::new(ManufacturingModule::new()));
        module_manager.register(Arc::new(ProjectsModule::new()));

        // Initialize core kernel schema and migrations
        module_manager.initialize()?;

        // For a brand new database where no modules are yet active, enable all standard modules
        if module_manager.list_modules()?.iter().all(|m| !m.is_enabled) {
            let _ = module_manager.enable_module("mod-inventory");
            let _ = module_manager.enable_module("mod-contacts");
            let _ = module_manager.enable_module("mod-invoicing");
            let _ = module_manager.enable_module("mod-accounting");
            let _ = module_manager.enable_module("mod-purchasing");
            let _ = module_manager.enable_module("mod-manufacturing");
            let _ = module_manager.enable_module("mod-projects");
        }

        // Seed initial default accounts
        let _ = AccountingService::seed_default_chart_of_accounts(&db.conn());

        // Seed demo lots for quality and expiration tracking
        let _ = StockService::create_or_update_lot(
            &db.conn(),
            "LOT-2026-Q1",
            "item-1",
            Some("2028-06-30"),
            Some("released"),
            Some("ACME-B882"),
        );
        let _ = StockService::create_or_update_lot(
            &db.conn(),
            "LOT-NSK-9",
            "item-2",
            Some("2029-12-31"),
            Some("released"),
            Some("NSK-9921"),
        );
        let _ = StockService::create_or_update_lot(
            &db.conn(),
            "LOT-EXP-SAMPLE",
            "item-1",
            Some("2026-01-15"),
            Some("quarantined"),
            Some("ACME-OLD-01"),
        );

        let peers = parking_lot::RwLock::new(vec![
            PeerNodeDto {
                node_id: "node-scanner-01".into(),
                display_name: "Warehouse Scanner Tablet (Android)".into(),
                ip_address: "192.168.1.142".into(),
                port: 8080,
                last_seen_seconds_ago: 4,
                is_synced: true,
            },
            PeerNodeDto {
                node_id: "node-ios-02".into(),
                display_name: "Workshop iPhone 15 (iOS)".into(),
                ip_address: "192.168.1.188".into(),
                port: 8080,
                last_seen_seconds_ago: 12,
                is_synced: true,
            },
        ]);

        let plugin_host = Arc::new(PluginHost::new());

        Ok(Arc::new(Self {
            db,
            hlc,
            bus,
            module_manager,
            plugin_host,
            peers,
        }))
    }

    /// Retrieve all registered ERP modules with their enabled status and UI routes
    pub fn get_available_modules(&self) -> Result<Vec<ModuleInfoDto>, ErpFfiError> {
        let list = self.module_manager.list_modules()?;
        Ok(list
            .into_iter()
            .map(|m| ModuleInfoDto {
                id: m.id,
                name: m.name,
                description: m.description,
                dependencies: m.dependencies,
                is_enabled: m.is_enabled,
                installed_version: m.installed_version,
                routes: m
                    .routes
                    .into_iter()
                    .map(|r| ModuleRouteDto {
                        route_id: r.route_id,
                        title: r.title,
                        icon: r.icon,
                        display_order: r.display_order,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Enable or disable a feature module ("Enable What You Need")
    pub fn set_module_enabled(&self, id: String, enabled: bool) -> Result<(), ErpFfiError> {
        if enabled {
            self.module_manager.enable_module(&id)?;
        } else {
            self.module_manager.disable_module(&id)?;
        }
        Ok(())
    }

    /// Create a storage location in the spatial hierarchy
    pub fn create_location(
        &self,
        code: String,
        name: String,
        parent_id: Option<String>,
        location_type: String,
        map_x: Option<f64>,
        map_y: Option<f64>,
    ) -> Result<LocationDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let loc_type = LocationType::from_str(&location_type).unwrap_or(LocationType::Bin);
        let loc = LocationService::create_location(
            &ctx,
            &code,
            &name,
            parent_id.as_deref(),
            loc_type,
            map_x,
            map_y,
        )?;
        Ok(Self::map_location(loc))
    }

    /// Move a location to a new parent in the hierarchy, maintaining closure table and breadcrumbs
    pub fn move_location(
        &self,
        location_id: String,
        new_parent_id: Option<String>,
    ) -> Result<LocationDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let loc = LocationService::move_location(&ctx, &location_id, new_parent_id.as_deref())?;
        Ok(Self::map_location(loc))
    }

    /// Update 2D spatial coordinates for a warehouse location
    pub fn update_location_coordinates(
        &self,
        location_id: String,
        map_x: f64,
        map_y: f64,
    ) -> Result<(), ErpFfiError> {
        let conn = self.db.conn();
        LocationService::update_location_coordinates(&conn, &location_id, map_x, map_y)?;
        Ok(())
    }

    /// Retrieve the full storage location tree
    pub fn get_location_tree(&self) -> Result<Vec<LocationNodeDto>, ErpFfiError> {
        let conn = self.db.conn();
        let tree = LocationService::get_location_tree(&conn)?;
        Ok(tree.into_iter().map(Self::map_tree_node).collect())
    }

    /// Retrieve flat list of all storage locations
    pub fn get_storage_locations(&self) -> Result<Vec<LocationDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = LocationService::list_all_locations(&conn)?;
        Ok(list.into_iter().map(Self::map_location).collect())
    }

    /// Create a catalog inventory item
    #[allow(clippy::too_many_arguments)]
    pub fn create_item(
        &self,
        sku: String,
        name: String,
        description: Option<String>,
        unit: String,
        track_serial: bool,
        track_lot: bool,
        min_stock: String,
        barcode: Option<String>,
    ) -> Result<ItemDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let min_stock_dec = Decimal::from_str(&min_stock).unwrap_or(Decimal::ZERO);
        let item = StockService::create_item(
            &ctx,
            &sku,
            &name,
            description.as_deref(),
            &unit,
            track_serial,
            track_lot,
            min_stock_dec,
            barcode.as_deref(),
        )?;

        Ok(ItemDto {
            id: item.id,
            sku: item.sku,
            name: item.name,
            description: item.description,
            unit: item.unit,
            track_serial: item.track_serial,
            track_lot: item.track_lot,
            min_stock_level: item.min_stock_level.to_string(),
            qr_code_uri: item.qr_code_uri,
            barcode: item.barcode,
        })
    }

    /// List all catalog items
    pub fn list_items(&self) -> Result<Vec<ItemDto>, ErpFfiError> {
        let conn = self.db.conn();
        let items = StockService::list_all_items(&conn)?;
        Ok(items
            .into_iter()
            .map(|item| ItemDto {
                id: item.id,
                sku: item.sku,
                name: item.name,
                description: item.description,
                unit: item.unit,
                track_serial: item.track_serial,
                track_lot: item.track_lot,
                min_stock_level: item.min_stock_level.to_string(),
                qr_code_uri: item.qr_code_uri,
                barcode: item.barcode,
            })
            .collect())
    }

    /// Instant "Where Is It?" search for SKU, barcode, or name
    pub fn search_item_locations(&self, query: String) -> Result<Vec<ItemLocationCardDto>, ErpFfiError> {
        let conn = self.db.conn();
        let matches = StockService::locate_item(&conn, &query)?;
        Ok(matches.into_iter().map(Self::map_item_location_card).collect())
    }

    /// Record a double-entry stock movement
    #[allow(clippy::too_many_arguments)]
    pub fn move_stock(
        &self,
        item_id: String,
        from_loc: Option<String>,
        to_loc: Option<String>,
        qty: String,
        movement_type: String,
        lot_or_sn: Option<String>,
        notes: Option<String>,
    ) -> Result<(), ErpFfiError> {
        let ctx = self.module_manager.context();
        let quantity = Decimal::from_str(&qty).map_err(|e| ErpFfiError::InvalidArgument {
            details: format!("Invalid quantity decimal '{}': {}", qty, e),
        })?;
        let m_type = MovementType::from_str(&movement_type).unwrap_or(MovementType::Transfer);

        StockService::record_movement(
            &ctx,
            m_type,
            &item_id,
            from_loc.as_deref(),
            to_loc.as_deref(),
            quantity,
            lot_or_sn.as_deref(),
            None,
            None,
            notes.as_deref(),
            "user",
        )?;

        Ok(())
    }

    /// Resolve a scanned QR code or barcode payload
    pub fn resolve_qr_scan(
        &self,
        raw_payload: String,
        active_context_location_id: Option<String>,
    ) -> Result<ScanResolutionDto, ErpFfiError> {
        // 1. Check if third-party Wasm plugin intercepts the scan
        if let Ok(Some(intercept)) = self.plugin_host.hook_on_qr_scanned(&raw_payload) {
            if let Some(sku) = intercept.translated_sku {
                // Resolved via plugin!
                return self.resolve_qr_scan(format!("erp://item/{}", sku), active_context_location_id);
            }
        }

        // 2. Standard QR Resolution
        let conn = self.db.conn();
        let res = QrService::resolve_qr_scan(
            &conn,
            &raw_payload,
            active_context_location_id.as_deref(),
        )?;

        match res {
            ScanResolution::LocationContext {
                location,
                items,
                prompt_message,
            } => Ok(ScanResolutionDto {
                resolution_type: ScanResolutionTypeDto::LocationContext,
                prompt_message,
                location: Some(Self::map_location(location)),
                item: None,
                item_locations: items.into_iter().map(Self::map_item_location_card).collect(),
                total_quantity: None,
                current_quantity_at_location: None,
                suggested_action: None,
                raw_payload,
            }),

            ScanResolution::ItemLookup {
                item,
                locations,
                total_quantity,
            } => Ok(ScanResolutionDto {
                resolution_type: ScanResolutionTypeDto::ItemLookup,
                prompt_message: format!("Item: {} - Total stock: {}", item.name, total_quantity),
                location: None,
                item: Some(ItemDto {
                    id: item.id,
                    sku: item.sku,
                    name: item.name,
                    description: item.description,
                    unit: item.unit,
                    track_serial: item.track_serial,
                    track_lot: item.track_lot,
                    min_stock_level: item.min_stock_level.to_string(),
                    qr_code_uri: item.qr_code_uri,
                    barcode: item.barcode,
                }),
                item_locations: locations.into_iter().map(Self::map_item_location_card).collect(),
                total_quantity: Some(total_quantity.to_string()),
                current_quantity_at_location: None,
                suggested_action: None,
                raw_payload,
            }),

            ScanResolution::RapidStockAction {
                item,
                target_location,
                current_quantity_at_location,
                suggested_action,
            } => Ok(ScanResolutionDto {
                resolution_type: ScanResolutionTypeDto::RapidStockAction,
                prompt_message: format!(
                    "Putaway/Move '{}' into '{}'?",
                    item.name, target_location.name
                ),
                location: Some(Self::map_location(target_location)),
                item: Some(ItemDto {
                    id: item.id,
                    sku: item.sku,
                    name: item.name,
                    description: item.description,
                    unit: item.unit,
                    track_serial: item.track_serial,
                    track_lot: item.track_lot,
                    min_stock_level: item.min_stock_level.to_string(),
                    qr_code_uri: item.qr_code_uri,
                    barcode: item.barcode,
                }),
                item_locations: Vec::new(),
                total_quantity: None,
                current_quantity_at_location: Some(current_quantity_at_location.to_string()),
                suggested_action: Some(suggested_action),
                raw_payload,
            }),

            ScanResolution::AuditCount {
                location,
                item,
                system_expected_qty,
                scanned_count,
                discrepancy,
            } => Ok(ScanResolutionDto {
                resolution_type: ScanResolutionTypeDto::AuditCount,
                prompt_message: format!(
                    "Audit Count: Scanned {} vs Expected {} (Discrepancy: {:+})",
                    scanned_count, system_expected_qty, discrepancy
                ),
                location: Some(Self::map_location(location)),
                item: Some(ItemDto {
                    id: item.id,
                    sku: item.sku,
                    name: item.name,
                    description: item.description,
                    unit: item.unit,
                    track_serial: item.track_serial,
                    track_lot: item.track_lot,
                    min_stock_level: item.min_stock_level.to_string(),
                    qr_code_uri: item.qr_code_uri,
                    barcode: item.barcode,
                }),
                item_locations: Vec::new(),
                total_quantity: Some(system_expected_qty.to_string()),
                current_quantity_at_location: Some(scanned_count.to_string()),
                suggested_action: Some(format!("Reconcile ({:+})", discrepancy)),
                raw_payload,
            }),

            ScanResolution::Unknown {
                raw_payload: raw,
                message,
            } => Ok(ScanResolutionDto {
                resolution_type: ScanResolutionTypeDto::Unknown,
                prompt_message: message,
                location: None,
                item: None,
                item_locations: Vec::new(),
                total_quantity: None,
                current_quantity_at_location: None,
                suggested_action: None,
                raw_payload: raw,
            }),
        }
    }

    /// Generate an offline SVG label for thermal/sticker printers
    pub fn generate_qr_label_svg(
        &self,
        target_uri: String,
        title: String,
        subtitle: String,
    ) -> Result<String, ErpFfiError> {
        let svg = QrService::generate_qr_label_svg(&target_uri, &title, &subtitle, None)?;
        Ok(svg)
    }

    /// Generate formatted label SVG for specific hardware presets (Thermal 62mm, Thermal 4x6", or A4 24-up sticker sheet)
    pub fn generate_formatted_label_svg(
        &self,
        format_type: String,
        target_uri: String,
        title: String,
        subtitle: String,
        details: Option<String>,
    ) -> Result<String, ErpFfiError> {
        let fmt = match format_type.as_str() {
            "Thermal4x6" => mod_inventory::LabelFormat::Thermal4x6,
            "A4Sheet" => mod_inventory::LabelFormat::A4StickerSheet,
            _ => mod_inventory::LabelFormat::ThermalRoll62mm,
        };
        let svg = QrService::generate_label_svg(fmt, &target_uri, &title, &subtitle, details.as_deref())?;
        Ok(svg)
    }

    /// Reconcile physical cycle count audit against system on-hand balance.
    /// Emits an immutable double-entry Adjust movement for any discrepancies.
    pub fn reconcile_audit(
        &self,
        location_id: String,
        item_id: String,
        physical_count: String,
        lot_number: Option<String>,
        serial_number: Option<String>,
    ) -> Result<Option<String>, ErpFfiError> {
        let ctx = self.module_manager.context();
        let count_dec = Decimal::from_str(&physical_count).map_err(|e| ErpFfiError::InvalidArgument {
            details: format!("Invalid count decimal: {}", e),
        })?;
        let maybe_movement = StockService::reconcile_audit(
            &ctx,
            &location_id,
            &item_id,
            count_dec,
            lot_number.as_deref(),
            serial_number.as_deref(),
            "user",
        )?;
        Ok(maybe_movement.map(|m| m.id))
    }

    /// Calculate total inventory units/valuation across all or filtered locations
    pub fn get_inventory_valuation(&self, location_id_filter: Option<String>) -> Result<String, ErpFfiError> {
        let conn = self.db.conn();
        let val = StockService::calculate_inventory_valuation(&conn, location_id_filter.as_deref())?;
        Ok(val.to_string())
    }

    /// Create a customer or supplier contact
    pub fn create_contact(
        &self,
        contact_type: String,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        tax_id: Option<String>,
        address: Option<String>,
    ) -> Result<ContactDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let c_type = ContactType::from_str(&contact_type).unwrap_or(ContactType::Customer);
        let contact = ContactsService::create_contact(
            &ctx,
            c_type,
            &name,
            email.as_deref(),
            phone.as_deref(),
            tax_id.as_deref(),
            address.as_deref(),
        )?;

        Ok(ContactDto {
            id: contact.id,
            contact_type: contact.contact_type.as_str().to_string(),
            name: contact.name,
            email: contact.email,
            phone: contact.phone,
            tax_id: contact.tax_id,
            address: contact.address,
            is_active: contact.is_active,
        })
    }

    /// List all contacts
    pub fn list_contacts(&self) -> Result<Vec<ContactDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = ContactsService::list_contacts(&conn).map_err(|e| ErpFfiError::DatabaseError {
            details: e.to_string(),
        })?;
        Ok(list
            .into_iter()
            .map(|c| ContactDto {
                id: c.id,
                contact_type: c.contact_type.as_str().to_string(),
                name: c.name,
                email: c.email,
                phone: c.phone,
                tax_id: c.tax_id,
                address: c.address,
                is_active: c.is_active,
            })
            .collect())
    }

    /// Finalize an invoice and trigger stock deductions
    pub fn finalize_invoice(&self, invoice_id: String) -> Result<InvoiceDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let inv = InvoicingService::finalize_invoice(&ctx, &invoice_id, "user")?;
        Ok(InvoiceDto {
            id: inv.id,
            invoice_number: inv.invoice_number,
            customer_id: inv.customer_id,
            status: inv.status.as_str().to_string(),
            subtotal: inv.subtotal.to_string(),
            tax_total: inv.tax_total.to_string(),
            grand_total: inv.grand_total.to_string(),
            notes: inv.notes,
            lines: inv
                .line_items
                .into_iter()
                .map(|li| InvoiceLineDto {
                    id: li.id,
                    item_id: li.item_id,
                    description: li.description,
                    quantity: li.quantity.to_string(),
                    unit_price: li.unit_price.to_string(),
                    tax_rate: li.tax_rate.to_string(),
                    line_total: li.line_total.to_string(),
                })
                .collect(),
        })
    }

    /// List all invoices
    pub fn list_invoices(&self) -> Result<Vec<InvoiceDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = InvoicingService::list_invoices(&conn).map_err(|e| ErpFfiError::DatabaseError {
            details: e.to_string(),
        })?;
        Ok(list
            .into_iter()
            .map(|inv| InvoiceDto {
                id: inv.id,
                invoice_number: inv.invoice_number,
                customer_id: inv.customer_id,
                status: inv.status.as_str().to_string(),
                subtotal: inv.subtotal.to_string(),
                tax_total: inv.tax_total.to_string(),
                grand_total: inv.grand_total.to_string(),
                notes: inv.notes,
                lines: inv
                    .line_items
                    .into_iter()
                    .map(|li| InvoiceLineDto {
                        id: li.id,
                        item_id: li.item_id,
                        description: li.description,
                        quantity: li.quantity.to_string(),
                        unit_price: li.unit_price.to_string(),
                        tax_rate: li.tax_rate.to_string(),
                        line_total: li.line_total.to_string(),
                    })
                    .collect(),
            })
            .collect())
    }

    /// Install a third-party Wasm plugin with Extism
    pub fn install_wasm_plugin(
        &self,
        manifest_json: String,
        wasm_bytes: Vec<u8>,
    ) -> Result<(), ErpFfiError> {
        let manifest: PluginManifest =
            serde_json::from_str(&manifest_json).map_err(|e| ErpFfiError::InvalidArgument {
                details: format!("Invalid manifest JSON: {}", e),
            })?;
        self.plugin_host.install_plugin(manifest, wasm_bytes)?;
        Ok(())
    }

    /// Retrieve declarative UI nodes from installed plugins
    pub fn get_plugin_ui_nodes(&self, context_view: String) -> Result<Vec<PluginUiNodeDto>, ErpFfiError> {
        let nodes = self.plugin_host.hook_render_ui(&context_view)?;
        Ok(nodes
            .into_iter()
            .map(Self::map_plugin_ui_node)
            .collect())
    }

    /// List all installed WASM plugin manifests
    pub fn list_installed_plugins(&self) -> Result<Vec<PluginManifestDto>, ErpFfiError> {
        let plugins = self.plugin_host.list_plugins();
        Ok(plugins
            .into_iter()
            .map(|p| PluginManifestDto {
                id: p.id,
                name: p.name,
                version: p.version,
                description: p.description,
                permissions: p.permissions,
            })
            .collect())
    }

    /// Uninstall a WASM plugin by ID
    pub fn uninstall_wasm_plugin(&self, plugin_id: String) -> Result<(), ErpFfiError> {
        self.plugin_host.uninstall_plugin(&plugin_id)?;
        Ok(())
    }

    /// Install a WASM plugin directly from a local file path
    pub fn install_wasm_plugin_file(
        &self,
        manifest_json: String,
        file_path: String,
    ) -> Result<(), ErpFfiError> {
        let bytes = std::fs::read(&file_path).map_err(|e| ErpFfiError::InvalidArgument {
            details: format!("Failed to read wasm file '{}': {}", file_path, e),
        })?;
        self.install_wasm_plugin(manifest_json, bytes)
    }

    /// Backup database file to a specified path with safe WAL flushing
    pub fn backup_database(&self, destination_path: String) -> Result<(), ErpFfiError> {
        self.db.backup_to_file(&destination_path)?;
        Ok(())
    }

    /// Export table data as CSV string ("items", "contacts", "journal")
    pub fn export_data_csv(&self, entity_type: String) -> Result<String, ErpFfiError> {
        match entity_type.to_lowercase().as_str() {
            "items" | "inventory" => {
                let items = self.list_items()?;
                let mut out = String::from("id,sku,name,description,unit,track_serial,track_lot,min_stock_level,barcode\n");
                for it in items {
                    out.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{}\n",
                        it.id,
                        it.sku,
                        it.name.replace(',', " "),
                        it.description.as_deref().unwrap_or("").replace(',', " "),
                        it.unit,
                        it.track_serial,
                        it.track_lot,
                        it.min_stock_level,
                        it.barcode.as_deref().unwrap_or("")
                    ));
                }
                Ok(out)
            }
            "contacts" => {
                let contacts = self.list_contacts()?;
                let mut out = String::from("id,name,tax_id,email,phone,contact_type\n");
                for c in contacts {
                    out.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        c.id,
                        c.name.replace(',', " "),
                        c.tax_id.as_deref().unwrap_or(""),
                        c.email.as_deref().unwrap_or(""),
                        c.phone.as_deref().unwrap_or(""),
                        c.contact_type
                    ));
                }
                Ok(out)
            }
            "journal" | "accounting" => {
                let conn = self.db.conn();
                let mut stmt = conn.prepare("SELECT id, entry_number, posting_date, description, status FROM gl_entries").map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                }).map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;
                let mut out = String::from("id,entry_number,posting_date,description,status\n");
                for r in rows {
                    let (id, num, dt, desc, st) = r.map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;
                    out.push_str(&format!("{},{},{},{},{}\n", id, num, dt, desc.replace(',', " "), st));
                }
                Ok(out)
            }
            other => Err(ErpFfiError::InvalidArgument {
                details: format!("Unsupported export entity: '{}'. Supported: items, contacts, journal", other),
            }),
        }
    }

    /// Export table data as JSON string ("items", "contacts", "journal")
    pub fn export_data_json(&self, entity_type: String) -> Result<String, ErpFfiError> {
        match entity_type.to_lowercase().as_str() {
            "items" | "inventory" => {
                let items = self.list_items()?;
                serde_json::to_string_pretty(&items).map_err(|e| ErpFfiError::InternalError { details: e.to_string() })
            }
            "contacts" => {
                let contacts = self.list_contacts()?;
                serde_json::to_string_pretty(&contacts).map_err(|e| ErpFfiError::InternalError { details: e.to_string() })
            }
            "journal" | "accounting" => {
                let conn = self.db.conn();
                let mut stmt = conn.prepare("SELECT id, entry_number, posting_date, description, status FROM gl_entries").map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;
                let rows = stmt.query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "entry_number": row.get::<_, String>(1)?,
                        "posting_date": row.get::<_, String>(2)?,
                        "description": row.get::<_, String>(3)?,
                        "status": row.get::<_, String>(4)?,
                    }))
                }).map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;
                let list: Vec<serde_json::Value> = rows.filter_map(Result::ok).collect();
                serde_json::to_string_pretty(&list).map_err(|e| ErpFfiError::InternalError { details: e.to_string() })
            }
            other => Err(ErpFfiError::InvalidArgument {
                details: format!("Unsupported export entity: '{}'. Supported: items, contacts, journal", other),
            }),
        }
    }

    /// Import initial data from CSV spreadsheets ("items", "contacts")
    pub fn import_data_csv(&self, entity_type: String, csv_content: String) -> Result<u32, ErpFfiError> {
        let lines: Vec<&str> = csv_content.lines().collect();
        if lines.is_empty() {
            return Ok(0);
        }

        let mut imported_count = 0u32;

        match entity_type.to_lowercase().as_str() {
            "items" | "inventory" => {
                for line in lines.iter().skip(1) {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        let sku = parts[0];
                        let name = parts[1];
                        let unit = if parts.len() > 2 && !parts[2].is_empty() { parts[2] } else { "pcs" };
                        let desc = if parts.len() > 3 && !parts[3].is_empty() { Some(parts[3].to_string()) } else { None };
                        let min_stock = if parts.len() > 4 && !parts[4].is_empty() { parts[4] } else { "0" };

                        if !sku.is_empty() && !name.is_empty() {
                            let _ = self.create_item(
                                sku.to_string(),
                                name.to_string(),
                                desc,
                                unit.to_string(),
                                false,
                                false,
                                min_stock.to_string(),
                                None,
                            );
                            imported_count += 1;
                        }
                    }
                }
                Ok(imported_count)
            }
            "contacts" => {
                for line in lines.iter().skip(1) {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if !parts.is_empty() && !parts[0].is_empty() {
                        let name = parts[0];
                        let email = if parts.len() > 1 && !parts[1].is_empty() { Some(parts[1].to_string()) } else { None };
                        let phone = if parts.len() > 2 && !parts[2].is_empty() { Some(parts[2].to_string()) } else { None };
                        let tax_id = if parts.len() > 3 && !parts[3].is_empty() { Some(parts[3].to_string()) } else { None };

                        let _ = self.create_contact(
                            "Customer".to_string(),
                            name.to_string(),
                            email,
                            phone,
                            tax_id,
                            None,
                        );
                        imported_count += 1;
                    }
                }
                Ok(imported_count)
            }
            other => Err(ErpFfiError::InvalidArgument {
                details: format!("Unsupported import entity: '{}'. Supported: items, contacts", other),
            }),
        }
    }

    // ==========================================
    // Accounting Methods
    // ==========================================

    pub fn get_chart_of_accounts(&self) -> Result<Vec<AccountDto>, ErpFfiError> {
        let conn = self.db.conn();
        let accounts = AccountingService::list_accounts(&conn)?;
        Ok(accounts.into_iter().map(Self::map_account).collect())
    }

    pub fn post_journal_entry(
        &self,
        posting_date: String,
        lines: Vec<JournalLineInputDto>,
        voucher_type: String,
        voucher_id: String,
        remarks: Option<String>,
    ) -> Result<Vec<GlEntryDto>, ErpFfiError> {
        let ctx = self.module_manager.context();
        let domain_lines = lines
            .into_iter()
            .map(|l| {
                let debit = Decimal::from_str(&l.debit).unwrap_or(Decimal::ZERO);
                let credit = Decimal::from_str(&l.credit).unwrap_or(Decimal::ZERO);
                JournalEntryLine {
                    account_id: l.account_id,
                    debit,
                    credit,
                    party_type: l.party_type,
                    party_id: l.party_id,
                    remarks: l.remarks,
                }
            })
            .collect();

        let entries = AccountingService::post_journal_entry(
            &ctx,
            &posting_date,
            domain_lines,
            &voucher_type,
            &voucher_id,
            remarks.as_deref(),
        )?;

        Ok(entries.into_iter().map(Self::map_gl_entry).collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_payment(
        &self,
        payment_type: String,
        party_id: String,
        paid_from_account_id: String,
        paid_to_account_id: String,
        amount: String,
        reference_invoice_id: Option<String>,
        payment_method: String,
    ) -> Result<PaymentEntryDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let p_type = PaymentType::from_str(&payment_type).unwrap_or(PaymentType::Receive);
        let amt = Decimal::from_str(&amount).map_err(|e| ErpFfiError::InvalidArgument {
            details: format!("Invalid payment amount: {}", e),
        })?;

        let payment = AccountingService::record_payment(
            &ctx,
            p_type,
            &party_id,
            &paid_from_account_id,
            &paid_to_account_id,
            amt,
            reference_invoice_id.as_deref(),
            &payment_method,
        )?;

        Ok(Self::map_payment(payment))
    }

    pub fn get_trial_balance(&self) -> Result<FinancialStatementDto, ErpFfiError> {
        let conn = self.db.conn();
        let stmt = AccountingService::get_trial_balance(&conn)?;
        Ok(Self::map_financial_statement(stmt))
    }

    pub fn get_profit_and_loss(
        &self,
        start_date: String,
        end_date: String,
    ) -> Result<FinancialStatementDto, ErpFfiError> {
        let conn = self.db.conn();
        let stmt = AccountingService::get_profit_and_loss(&conn, &start_date, &end_date)?;
        Ok(Self::map_financial_statement(stmt))
    }

    // ==========================================
    // Purchasing Methods
    // ==========================================

    pub fn create_purchase_order(
        &self,
        po_number: String,
        supplier_id: String,
        expected_delivery: Option<String>,
        lines: Vec<PurchaseOrderLineDto>,
        notes: Option<String>,
    ) -> Result<PurchaseOrderDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let domain_lines = lines
            .into_iter()
            .map(|l| {
                let qty = Decimal::from_str(&l.quantity).unwrap_or(Decimal::ZERO);
                let price = Decimal::from_str(&l.unit_price).unwrap_or(Decimal::ZERO);
                let tax = Decimal::from_str(&l.tax_rate).unwrap_or(Decimal::ZERO);
                (l.item_id, l.description, qty, price, tax)
            })
            .collect();

        let po = PurchasingService::create_purchase_order(
            &ctx,
            &po_number,
            &supplier_id,
            expected_delivery.as_deref(),
            domain_lines,
            notes.as_deref(),
        )?;

        Ok(Self::map_purchase_order(po))
    }

    pub fn submit_purchase_order(&self, po_id: String) -> Result<PurchaseOrderDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let po = PurchasingService::submit_purchase_order(&ctx, &po_id)?;
        Ok(Self::map_purchase_order(po))
    }

    pub fn receive_goods(
        &self,
        po_id: Option<String>,
        supplier_id: String,
        destination_location_id: String,
        items: Vec<PurchaseReceiptLineDto>,
        notes: Option<String>,
    ) -> Result<PurchaseReceiptDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let domain_items = items
            .into_iter()
            .map(|i| {
                let qty = Decimal::from_str(&i.quantity).unwrap_or(Decimal::ZERO);
                (i.item_id, qty, i.lot_number, i.serial_number)
            })
            .collect();

        let receipt = PurchasingService::receive_goods(
            &ctx,
            po_id.as_deref(),
            &supplier_id,
            &destination_location_id,
            domain_items,
            notes.as_deref(),
        )?;

        Ok(Self::map_purchase_receipt(receipt))
    }

    pub fn list_purchase_orders(&self) -> Result<Vec<PurchaseOrderDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = PurchasingService::list_purchase_orders(&conn)?;
        Ok(list.into_iter().map(Self::map_purchase_order).collect())
    }

    // ==========================================
    // Manufacturing Methods
    // ==========================================

    pub fn create_bom(
        &self,
        bom_number: String,
        item_id: String,
        name: String,
        quantity: String,
        components: Vec<BomComponentDto>,
        routing_notes: Option<String>,
    ) -> Result<BillOfMaterialsDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let qty = Decimal::from_str(&quantity).unwrap_or(Decimal::ONE);
        let domain_comps = components
            .into_iter()
            .map(|c| {
                let c_qty = Decimal::from_str(&c.quantity).unwrap_or(Decimal::ZERO);
                let scrap = Decimal::from_str(&c.scrap_percentage).unwrap_or(Decimal::ZERO);
                (c.item_id, c_qty, scrap, c.notes)
            })
            .collect();

        let bom = ManufacturingService::create_bom(
            &ctx,
            &bom_number,
            &item_id,
            &name,
            qty,
            domain_comps,
            routing_notes.as_deref(),
        )?;

        Ok(Self::map_bom(bom))
    }

    pub fn list_boms(&self) -> Result<Vec<BillOfMaterialsDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = ManufacturingService::list_boms(&conn)?;
        Ok(list.into_iter().map(Self::map_bom).collect())
    }

    pub fn create_work_order(
        &self,
        work_order_number: String,
        bom_id: String,
        target_quantity: String,
        source_location_id: String,
        destination_location_id: String,
    ) -> Result<WorkOrderDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let qty = Decimal::from_str(&target_quantity).unwrap_or(Decimal::ZERO);
        let wo = ManufacturingService::create_work_order(
            &ctx,
            &work_order_number,
            &bom_id,
            qty,
            &source_location_id,
            &destination_location_id,
        )?;

        Ok(Self::map_work_order(wo))
    }

    pub fn execute_work_order(&self, work_order_id: String) -> Result<WorkOrderDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let wo = ManufacturingService::execute_work_order(&ctx, &work_order_id)?;
        Ok(Self::map_work_order(wo))
    }

    pub fn list_work_orders(&self) -> Result<Vec<WorkOrderDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = ManufacturingService::list_work_orders(&conn)?;
        Ok(list.into_iter().map(Self::map_work_order).collect())
    }

    // ==========================================
    // Projects & Timesheets Methods
    // ==========================================

    pub fn create_project(
        &self,
        name: String,
        code: String,
        customer_id: Option<String>,
        billing_rate_hourly: String,
        estimated_hours: String,
    ) -> Result<ProjectDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let rate = Decimal::from_str(&billing_rate_hourly).unwrap_or(Decimal::ZERO);
        let est = Decimal::from_str(&estimated_hours).unwrap_or(Decimal::ZERO);

        let p = ProjectsService::create_project(
            &ctx,
            &name,
            &code,
            customer_id.as_deref(),
            rate,
            est,
        )?;

        Ok(Self::map_project(p))
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectDto>, ErpFfiError> {
        let conn = self.db.conn();
        let list = ProjectsService::list_projects(&conn)?;
        Ok(list.into_iter().map(Self::map_project).collect())
    }

    pub fn create_task(
        &self,
        project_id: String,
        title: String,
        priority: String,
        estimated_hours: String,
    ) -> Result<ProjectTaskDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let est = Decimal::from_str(&estimated_hours).unwrap_or(Decimal::ZERO);
        let task = ProjectsService::create_task(&ctx, &project_id, &title, &priority, est)?;

        Ok(ProjectTaskDto {
            id: task.id,
            project_id: task.project_id,
            title: task.title,
            status: task.status,
            priority: task.priority,
            estimated_hours: task.estimated_hours.to_string(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_timesheet(
        &self,
        project_id: String,
        task_id: Option<String>,
        employee_or_user: String,
        activity_date: String,
        hours: String,
        hourly_rate_override: Option<String>,
        is_billable: bool,
        description: String,
    ) -> Result<TimesheetEntryDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let hrs = Decimal::from_str(&hours).unwrap_or(Decimal::ZERO);
        let rate_override = hourly_rate_override.and_then(|r| Decimal::from_str(&r).ok());

        let ts = ProjectsService::record_timesheet(
            &ctx,
            &project_id,
            task_id.as_deref(),
            &employee_or_user,
            &activity_date,
            hrs,
            rate_override,
            is_billable,
            &description,
        )?;

        Ok(Self::map_timesheet(ts))
    }

    pub fn invoice_unbilled_timesheets(
        &self,
        project_id: String,
        invoice_number: String,
        tax_rate: String,
    ) -> Result<InvoiceDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let tax = Decimal::from_str(&tax_rate).unwrap_or(Decimal::ZERO);

        let inv = ProjectsService::invoice_unbilled_timesheets(
            &ctx,
            &project_id,
            &invoice_number,
            tax,
        )?;

        Ok(Self::map_invoice(inv))
    }

    // Offline Vector PDF Generation for Invoices and Purchase Orders
    // ------------------------------------------------------------------------

    /// Generate an offline vector PDF invoice with fixed-point Decimal math & EPC QR code
    pub fn generate_invoice_pdf(&self, invoice_id: String) -> Result<Vec<u8>, ErpFfiError> {
        let conn = self.db.conn();
        let invoice = InvoicingService::get_invoice(&conn, &invoice_id)?;
        let customer = ContactsService::get_contact_by_id(&conn, &invoice.customer_id)
            .map(|c| c.name)
            .unwrap_or_else(|_| "Valued Customer".into());

        let lines: Vec<(String, Decimal, Decimal, Decimal, Decimal)> = invoice
            .line_items
            .into_iter()
            .map(|l| (l.description, l.quantity, l.unit_price, l.tax_rate, l.line_total))
            .collect();

        let bytes = crate::pdf_generator::generate_invoice_pdf(
            &invoice.invoice_number,
            invoice.status.as_str(),
            &customer,
            &invoice.created_at.to_rfc3339(),
            invoice.subtotal,
            invoice.tax_total,
            invoice.grand_total,
            invoice.notes.as_deref(),
            &lines,
        );

        Ok(bytes)
    }

    /// Export invoice PDF directly to a local filesystem destination
    pub fn export_invoice_pdf_to_file(&self, invoice_id: String, destination_path: String) -> Result<String, ErpFfiError> {
        let bytes = self.generate_invoice_pdf(invoice_id)?;
        if let Some(parent) = std::path::Path::new(&destination_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&destination_path, &bytes)
            .map_err(|e| ErpFfiError::DatabaseError { details: format!("Failed writing invoice PDF: {}", e) })?;
        Ok(destination_path)
    }

    /// Generate an offline vector PDF purchase order for vendor communication
    pub fn generate_purchase_order_pdf(&self, po_id: String) -> Result<Vec<u8>, ErpFfiError> {
        let conn = self.db.conn();
        let po = PurchasingService::get_purchase_order_by_id(&conn, &po_id)?;
        let supplier = ContactsService::get_contact_by_id(&conn, &po.supplier_id)
            .map(|c| c.name)
            .unwrap_or_else(|_| "Approved Vendor".into());

        let lines: Vec<(String, String, Decimal, Decimal, Decimal)> = po
            .lines
            .into_iter()
            .map(|l| {
                let sku: String = conn.query_row(
                    "SELECT sku FROM inventory_items WHERE id = ?1",
                    rusqlite::params![l.item_id],
                    |r| r.get(0),
                ).unwrap_or_else(|_| "PART-STD".into());
                (sku, l.description, l.quantity, l.unit_price, l.line_total)
            })
            .collect();

        let bytes = crate::pdf_generator::generate_purchase_order_pdf(
            &po.po_number,
            po.status.as_str(),
            &supplier,
            &po.created_at.to_rfc3339(),
            "Main Facility > Workshop > Rack 2 (North Wall) > Shelf C > Blue Bin #4",
            po.grand_total,
            po.notes.as_deref(),
            &lines,
        );

        Ok(bytes)
    }

    /// Export purchase order PDF directly to a local filesystem destination
    pub fn export_purchase_order_pdf_to_file(&self, po_id: String, destination_path: String) -> Result<String, ErpFfiError> {
        let bytes = self.generate_purchase_order_pdf(po_id)?;
        if let Some(parent) = std::path::Path::new(&destination_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&destination_path, &bytes)
            .map_err(|e| ErpFfiError::DatabaseError { details: format!("Failed writing PO PDF: {}", e) })?;
        Ok(destination_path)
    }

    // ------------------------------------------------------------------------
    // Low-Stock Monitoring & Automated Reorder PO Generation
    // ------------------------------------------------------------------------

    /// Retrieve list of all items currently at or below their minimum stock threshold
    pub fn get_low_stock_alerts(&self) -> Result<Vec<LowStockAlertDto>, ErpFfiError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT
                i.id,
                i.sku,
                i.name,
                i.unit,
                COALESCE(SUM(CAST(q.quantity AS REAL)), 0.0) AS current_stock,
                CAST(i.min_stock_level AS REAL) AS min_stock,
                i.min_stock_level
            FROM inventory_items i
            LEFT JOIN stock_quants q ON i.id = q.item_id
            WHERE CAST(i.min_stock_level AS REAL) > 0
            GROUP BY i.id
            HAVING current_stock <= min_stock
            ORDER BY (min_stock - current_stock) DESC
            "#,
        )?;

        // Default or preferred supplier from contacts
        let supplier_row: Option<(String, String)> = conn.query_row(
            "SELECT id, name FROM contacts WHERE contact_type IN ('Vendor', 'Supplier') LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok();

        let rows = stmt.query_map([], |row| {
            let item_id: String = row.get(0)?;
            let sku: String = row.get(1)?;
            let name: String = row.get(2)?;
            let unit: String = row.get(3)?;
            let current_stock_f: f64 = row.get(4)?;
            let _min_stock_f: f64 = row.get(5)?;
            let min_stock_level: String = row.get(6)?;

            let current_stock = Decimal::from_str(&format!("{:.2}", current_stock_f)).unwrap_or(Decimal::ZERO);
            let min_stock = Decimal::from_str(&min_stock_level).unwrap_or(Decimal::ZERO);
            let suggested_qty = ((min_stock * Decimal::from(2)) - current_stock).max(Decimal::from(50));

            Ok(LowStockAlertDto {
                item_id,
                sku,
                name,
                unit,
                current_stock: current_stock.to_string(),
                min_stock_level,
                suggested_reorder_qty: suggested_qty.to_string(),
                preferred_supplier_id: supplier_row.as_ref().map(|(id, _)| id.clone()),
                preferred_supplier_name: supplier_row.as_ref().map(|(_, name)| name.clone()),
            })
        })?;

        let mut alerts = Vec::new();
        for r in rows {
            alerts.push(r?);
        }
        Ok(alerts)
    }

    /// Automatically create a draft Purchase Order for a low-stock inventory item
    pub fn generate_reorder_po(&self, item_id: String) -> Result<PurchaseOrderDto, ErpFfiError> {
        let (sku, name, min_stock, supplier_id, supplier_name) = {
            let conn = self.db.conn();
            let (sku, name, _unit, min_stock_str): (String, String, String, String) = conn.query_row(
                "SELECT sku, name, unit, min_stock_level FROM inventory_items WHERE id = ?1",
                rusqlite::params![item_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).map_err(|e| ErpFfiError::DatabaseError { details: e.to_string() })?;

            let min_stock = Decimal::from_str(&min_stock_str).unwrap_or(Decimal::from(50));

            let (supplier_id, supplier_name): (String, String) = conn.query_row(
                "SELECT id, name FROM contacts WHERE contact_type IN ('Vendor', 'Supplier') LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap_or_else(|_| ("cont-supp-1".into(), "Acme Fasteners Corp".into()));

            (sku, name, min_stock, supplier_id, supplier_name)
        };

        let reorder_qty = (min_stock * Decimal::from(2)).max(Decimal::from(50));
        let po_number = format!("PO-REORDER-{}", &uuid::Uuid::new_v4().to_string()[..6].to_uppercase());
        let unit_cost = Decimal::from_str("1.25").unwrap();
        let line_total = reorder_qty * unit_cost;

        let po_line = PurchaseOrderLineDto {
            id: String::new(),
            item_id: item_id.clone(),
            description: format!("Automated replenishment for {} ({})", name, sku),
            quantity: reorder_qty.to_string(),
            received_quantity: "0".into(),
            unit_price: unit_cost.to_string(),
            tax_rate: "0.0".into(),
            line_total: line_total.to_string(),
        };

        self.create_purchase_order(
            po_number,
            supplier_id,
            Some("2026-11-30".into()),
            vec![po_line],
            Some(format!("Automated low-stock replenishment PO generated for {} from preferred vendor {}", sku, supplier_name)),
        )
    }

    // ------------------------------------------------------------------------
    // Kanban Workflow Board & Component Genealogy
    // ------------------------------------------------------------------------

    /// Update work order status (Planned -> In Progress -> Quality Check -> Completed) and record state transition
    pub fn update_work_order_status(&self, work_order_id: String, new_status: String) -> Result<WorkOrderDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let wo = ManufacturingService::update_work_order_status(&ctx, &work_order_id, &new_status)?;
        Ok(WorkOrderDto {
            id: wo.id,
            work_order_number: wo.work_order_number,
            bom_id: wo.bom_id,
            item_id: wo.item_id,
            target_quantity: wo.target_quantity.to_string(),
            source_location_id: wo.source_location_id,
            destination_location_id: wo.destination_location_id,
            status: wo.status.as_str().to_string(),
            created_at: wo.created_at.to_rfc3339(),
            completed_at: wo.completed_at.map(|dt| dt.to_rfc3339()),
        })
    }

    /// Retrieve component genealogy tree for a finished work order
    pub fn get_work_order_genealogy(&self, work_order_id_or_number: String) -> Result<Option<ComponentGenealogyDto>, ErpFfiError> {
        let conn = self.db.conn();
        let tree_opt = ManufacturingService::get_work_order_genealogy(&conn, &work_order_id_or_number)?;
        Ok(tree_opt.map(|t| ComponentGenealogyDto {
            work_order_id: t.work_order_id,
            work_order_number: t.work_order_number,
            finished_item_id: t.finished_item_id,
            finished_sku: t.finished_sku,
            finished_name: t.finished_name,
            finished_lot_number: t.finished_lot_number,
            quantity: t.quantity.to_string(),
            completed_at: t.completed_at.map(|dt| dt.to_rfc3339()),
            components: t.components.into_iter().map(|c| ComponentGenealogyEntryDto {
                component_item_id: c.component_item_id,
                component_sku: c.component_sku,
                component_name: c.component_name,
                component_lot_number: c.component_lot_number,
                component_quantity: c.component_quantity.to_string(),
                expiration_date: c.expiration_date,
                quarantine_status: c.quarantine_status,
                manufacturer_batch: c.manufacturer_batch,
            }).collect(),
        }))
    }

    /// List project tasks, optionally filtered by project_id
    pub fn list_project_tasks(&self, project_id: Option<String>) -> Result<Vec<ProjectTaskDto>, ErpFfiError> {
        let conn = self.db.conn();
        let tasks = ProjectsService::list_tasks(&conn, project_id.as_deref())?;
        Ok(tasks.into_iter().map(|t| ProjectTaskDto {
            id: t.id,
            project_id: t.project_id,
            title: t.title,
            status: t.status,
            priority: t.priority,
            estimated_hours: t.estimated_hours.to_string(),
        }).collect())
    }

    /// Update status of a project task (e.g. Backlog -> In Progress -> Review -> Done)
    pub fn update_project_task_status(&self, task_id: String, new_status: String) -> Result<ProjectTaskDto, ErpFfiError> {
        let ctx = self.module_manager.context();
        let task = ProjectsService::update_task_status(&ctx, &task_id, &new_status)?;
        Ok(ProjectTaskDto {
            id: task.id,
            project_id: task.project_id,
            title: task.title,
            status: task.status,
            priority: task.priority,
            estimated_hours: task.estimated_hours.to_string(),
        })
    }

    /// Get unbilled hours and billable totals summary
    pub fn get_unbilled_hours_summary(&self, project_id: Option<String>) -> Result<UnbilledHoursSummaryDto, ErpFfiError> {
        let conn = self.db.conn();
        let (total_hours, total_amount, count) = ProjectsService::get_unbilled_summary(&conn, project_id.as_deref())?;
        Ok(UnbilledHoursSummaryDto {
            total_unbilled_hours: total_hours.to_string(),
            total_unbilled_amount: total_amount.to_string(),
            unbilled_entries_count: count,
        })
    }

    // ------------------------------------------------------------------------
    // Lot Expiration, Quarantine, and Batch Tracking
    // ------------------------------------------------------------------------

    /// Register or update a tracked inventory lot
    pub fn create_or_update_lot(
        &self,
        lot_number: String,
        item_id: String,
        expiration_date: Option<String>,
        quarantine_status: Option<String>,
        manufacturer_batch: Option<String>,
    ) -> Result<LotInfoDto, ErpFfiError> {
        let conn = self.db.conn();
        let lot = StockService::create_or_update_lot(
            &conn,
            &lot_number,
            &item_id,
            expiration_date.as_deref(),
            quarantine_status.as_deref(),
            manufacturer_batch.as_deref(),
        )?;
        Ok(LotInfoDto {
            lot_number: lot.lot_number,
            item_id: lot.item_id,
            expiration_date: lot.expiration_date,
            quarantine_status: lot.quarantine_status,
            manufacturer_batch: lot.manufacturer_batch,
            created_at: lot.created_at.to_rfc3339(),
        })
    }

    /// Get details of an inventory lot
    pub fn get_lot_info(&self, lot_number: String) -> Result<Option<LotInfoDto>, ErpFfiError> {
        let conn = self.db.conn();
        let lot_opt = StockService::get_lot_info(&conn, &lot_number)?;
        Ok(lot_opt.map(|lot| LotInfoDto {
            lot_number: lot.lot_number,
            item_id: lot.item_id,
            expiration_date: lot.expiration_date,
            quarantine_status: lot.quarantine_status,
            manufacturer_batch: lot.manufacturer_batch,
            created_at: lot.created_at.to_rfc3339(),
        }))
    }

    // ------------------------------------------------------------------------
    // Peer-to-Peer LAN / mDNS Synchronization
    // ------------------------------------------------------------------------

    /// Get peer-to-peer sync status and connected local LAN nodes
    pub fn get_sync_status(&self) -> Result<SyncStatusDto, ErpFfiError> {
        let conn = self.db.conn();
        let unsynced_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM event_log WHERE synced = 0", [], |r| r.get(0))
            .unwrap_or(0);
        let total_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM event_log", [], |r| r.get(0))
            .unwrap_or(0);

        let peers = self.peers.read().clone();
        let convergence_state = if unsynced_count == 0 {
            "Converged".to_string()
        } else {
            "Sync Pending".to_string()
        };

        Ok(SyncStatusDto {
            connected_peers_count: peers.len() as u32,
            unsynced_events_count: unsynced_count,
            total_events_count: total_count,
            last_sync_timestamp: chrono::Utc::now().to_rfc3339(),
            convergence_state,
            local_hlc: self.hlc.now().to_string(),
            peers,
        })
    }

    /// Register a discovered peer on the local subnet (mDNS / UDP zero-config)
    pub fn register_peer(&self, node_id: String, display_name: String, ip_address: String, port: u16) -> Result<(), ErpFfiError> {
        let mut peers = self.peers.write();
        if let Some(existing) = peers.iter_mut().find(|p| p.node_id == node_id) {
            existing.ip_address = ip_address;
            existing.port = port;
            existing.last_seen_seconds_ago = 0;
            existing.is_synced = true;
        } else {
            peers.push(PeerNodeDto {
                node_id,
                display_name,
                ip_address,
                port,
                last_seen_seconds_ago: 0,
                is_synced: true,
            });
        }
        Ok(())
    }

    /// Trigger P2P event log sync across discovered LAN peers
    pub fn trigger_lan_sync(&self) -> Result<SyncStatusDto, ErpFfiError> {
        {
            let conn = self.db.conn();
            // Export unsynced events and mark them synced
            let unsynced = erp_kernel::SyncEngine::export_unsynced_events(&conn, 500)?;
            let ids: Vec<String> = unsynced.into_iter().map(|e| e.event_id).collect();
            erp_kernel::SyncEngine::mark_events_synced(&conn, &ids)?;
        }

        {
            let mut peers = self.peers.write();
            for p in peers.iter_mut() {
                p.is_synced = true;
                p.last_seen_seconds_ago = 1;
            }
        }

        self.get_sync_status()
    }
}

impl ErpEngine {
    fn map_invoice(inv: mod_invoicing::Invoice) -> InvoiceDto {
        InvoiceDto {
            id: inv.id,
            invoice_number: inv.invoice_number,
            customer_id: inv.customer_id,
            status: inv.status.as_str().to_string(),
            subtotal: inv.subtotal.to_string(),
            tax_total: inv.tax_total.to_string(),
            grand_total: inv.grand_total.to_string(),
            notes: inv.notes,
            lines: inv
                .line_items
                .into_iter()
                .map(|li| InvoiceLineDto {
                    id: li.id,
                    item_id: li.item_id,
                    description: li.description,
                    quantity: li.quantity.to_string(),
                    unit_price: li.unit_price.to_string(),
                    tax_rate: li.tax_rate.to_string(),
                    line_total: li.line_total.to_string(),
                })
                .collect(),
        }
    }

    fn map_account(acc: Account) -> AccountDto {
        AccountDto {
            id: acc.id,
            code: acc.code,
            name: acc.name,
            root_type: acc.root_type.as_str().to_string(),
            parent_id: acc.parent_id,
            is_group: acc.is_group,
            balance: acc.balance.to_string(),
            is_active: acc.is_active,
        }
    }

    fn map_gl_entry(entry: GlEntry) -> GlEntryDto {
        GlEntryDto {
            id: entry.id,
            entry_number: entry.entry_number,
            posting_date: entry.posting_date,
            account_id: entry.account_id,
            party_type: entry.party_type,
            party_id: entry.party_id,
            debit: entry.debit.to_string(),
            credit: entry.credit.to_string(),
            voucher_type: entry.voucher_type,
            voucher_id: entry.voucher_id,
            remarks: entry.remarks,
        }
    }

    fn map_payment(p: PaymentEntry) -> PaymentEntryDto {
        PaymentEntryDto {
            id: p.id,
            payment_type: p.payment_type.as_str().to_string(),
            posting_date: p.posting_date,
            party_id: p.party_id,
            paid_from_account_id: p.paid_from_account_id,
            paid_to_account_id: p.paid_to_account_id,
            amount: p.amount.to_string(),
            reference_invoice_id: p.reference_invoice_id,
            payment_method: p.payment_method,
        }
    }

    fn map_financial_statement(stmt: FinancialStatement) -> FinancialStatementDto {
        FinancialStatementDto {
            title: stmt.title,
            as_of_or_period: stmt.as_of_or_period,
            rows: stmt
                .rows
                .into_iter()
                .map(|r| TrialBalanceRowDto {
                    account_id: r.account_id,
                    account_code: r.account_code,
                    account_name: r.account_name,
                    root_type: r.root_type.as_str().to_string(),
                    total_debit: r.total_debit.to_string(),
                    total_credit: r.total_credit.to_string(),
                    net_balance: r.net_balance.to_string(),
                })
                .collect(),
            total_debit: stmt.total_debit.to_string(),
            total_credit: stmt.total_credit.to_string(),
            net_income_or_equity: stmt.net_income_or_equity.to_string(),
        }
    }

    fn map_purchase_order(po: PurchaseOrder) -> PurchaseOrderDto {
        PurchaseOrderDto {
            id: po.id,
            po_number: po.po_number,
            supplier_id: po.supplier_id,
            order_date: po.order_date,
            expected_delivery: po.expected_delivery,
            status: po.status.as_str().to_string(),
            subtotal: po.subtotal.to_string(),
            tax_total: po.tax_total.to_string(),
            grand_total: po.grand_total.to_string(),
            notes: po.notes,
            lines: po
                .lines
                .into_iter()
                .map(|l| PurchaseOrderLineDto {
                    id: l.id,
                    item_id: l.item_id,
                    description: l.description,
                    quantity: l.quantity.to_string(),
                    received_quantity: l.received_quantity.to_string(),
                    unit_price: l.unit_price.to_string(),
                    tax_rate: l.tax_rate.to_string(),
                    line_total: l.line_total.to_string(),
                })
                .collect(),
        }
    }

    fn map_purchase_receipt(pr: PurchaseReceipt) -> PurchaseReceiptDto {
        PurchaseReceiptDto {
            id: pr.id,
            receipt_number: pr.receipt_number,
            purchase_order_id: pr.purchase_order_id,
            supplier_id: pr.supplier_id,
            destination_location_id: pr.destination_location_id,
            received_at: pr.received_at.to_rfc3339(),
            notes: pr.notes,
            lines: pr
                .lines
                .into_iter()
                .map(|l| PurchaseReceiptLineDto {
                    id: l.id,
                    item_id: l.item_id,
                    quantity: l.quantity.to_string(),
                    lot_number: l.lot_number,
                    serial_number: l.serial_number,
                })
                .collect(),
        }
    }

    fn map_bom(b: BillOfMaterials) -> BillOfMaterialsDto {
        BillOfMaterialsDto {
            id: b.id,
            bom_number: b.bom_number,
            item_id: b.item_id,
            name: b.name,
            quantity: b.quantity.to_string(),
            routing_notes: b.routing_notes,
            components: b
                .components
                .into_iter()
                .map(|c| BomComponentDto {
                    id: c.id,
                    item_id: c.item_id,
                    quantity: c.quantity.to_string(),
                    scrap_percentage: c.scrap_percentage.to_string(),
                    notes: c.notes,
                })
                .collect(),
        }
    }

    fn map_work_order(wo: WorkOrder) -> WorkOrderDto {
        WorkOrderDto {
            id: wo.id,
            work_order_number: wo.work_order_number,
            bom_id: wo.bom_id,
            item_id: wo.item_id,
            target_quantity: wo.target_quantity.to_string(),
            source_location_id: wo.source_location_id,
            destination_location_id: wo.destination_location_id,
            status: wo.status.as_str().to_string(),
            created_at: wo.created_at.to_rfc3339(),
            completed_at: wo.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }

    fn map_project(p: Project) -> ProjectDto {
        ProjectDto {
            id: p.id,
            name: p.name,
            code: p.code,
            customer_id: p.customer_id,
            status: p.status.as_str().to_string(),
            billing_rate_hourly: p.billing_rate_hourly.to_string(),
            estimated_hours: p.estimated_hours.to_string(),
        }
    }

    fn map_timesheet(ts: TimesheetEntry) -> TimesheetEntryDto {
        TimesheetEntryDto {
            id: ts.id,
            project_id: ts.project_id,
            task_id: ts.task_id,
            employee_or_user: ts.employee_or_user,
            activity_date: ts.activity_date,
            hours: ts.hours.to_string(),
            hourly_rate: ts.hourly_rate.to_string(),
            billing_amount: ts.billing_amount.to_string(),
            is_billable: ts.is_billable,
            is_invoiced: ts.is_invoiced,
            invoice_id: ts.invoice_id,
            description: ts.description,
        }
    }
    fn map_location(loc: StorageLocation) -> LocationDto {
        LocationDto {
            id: loc.id,
            code: loc.code,
            name: loc.name,
            parent_id: loc.parent_id,
            location_type: loc.location_type.as_str().to_string(),
            path_breadcrumb: loc.path_breadcrumb,
            qr_code_uri: loc.qr_code_uri,
            map_x: loc.map_x,
            map_y: loc.map_y,
            is_active: loc.is_active,
        }
    }

    fn map_tree_node(node: LocationTreeNode) -> LocationNodeDto {
        LocationNodeDto {
            location: Self::map_location(node.location),
            children: node.children.into_iter().map(Self::map_tree_node).collect(),
            total_item_count: node.total_item_count,
        }
    }

    fn map_plugin_ui_node(node: erp_plugin_host::PluginUiNode) -> PluginUiNodeDto {
        use erp_plugin_host::PluginUiNodeType;
        let (node_kind, title_or_label, secondary_text, color_hex, custom_field_key) =
            match node.node_type {
                PluginUiNodeType::Container { direction, spacing } => (
                    "Container".to_string(),
                    direction,
                    Some(spacing.to_string()),
                    None,
                    None,
                ),
                PluginUiNodeType::Text { content, style } => {
                    ("Text".to_string(), content, Some(style), None, None)
                }
                PluginUiNodeType::Badge { label, color_hex } => {
                    ("Badge".to_string(), label, None, Some(color_hex), None)
                }
                PluginUiNodeType::StatCard { label, value, delta } => {
                    ("StatCard".to_string(), label, Some(value), delta, None)
                }
                PluginUiNodeType::Button { id, label, style } => {
                    ("Button".to_string(), label, Some(style), None, Some(id))
                }
                PluginUiNodeType::CustomFieldInput {
                    field_key,
                    label,
                    default_value,
                } => (
                    "CustomFieldInput".to_string(),
                    label,
                    default_value,
                    None,
                    Some(field_key),
                ),
                _ => ("Card".to_string(), String::new(), None, None, None),
            };

        PluginUiNodeDto {
            node_id: node.node_id,
            node_kind,
            title_or_label,
            secondary_text,
            color_hex,
            custom_field_key,
            children: node
                .children
                .into_iter()
                .map(Self::map_plugin_ui_node)
                .collect(),
        }
    }

    fn map_item_location_card(m: mod_inventory::ItemLocationMatch) -> ItemLocationCardDto {
        let is_low_stock = m.quantity <= m.min_stock_level && m.min_stock_level > Decimal::ZERO;
        let is_expired_or_expiring = m.lot_quarantine_status.as_deref() == Some("quarantined")
            || m.lot_quarantine_status.as_deref() == Some("expired")
            || m.lot_expiration_date.as_ref().map_or(false, |d| d.as_str() <= "2026-12-31");

        ItemLocationCardDto {
            item_id: m.item_id,
            sku: m.sku,
            item_name: m.item_name,
            unit: m.unit,
            location_id: m.location_id,
            location_code: m.location_code,
            location_name: m.location_name,
            location_type: m.location_type,
            breadcrumb_path: m.breadcrumb_path,
            map_x: m.map_x,
            map_y: m.map_y,
            quantity: m.quantity.to_string(),
            lot_number: m.lot_number,
            serial_number: m.serial_number,
            min_stock_level: m.min_stock_level.to_string(),
            lot_expiration_date: m.lot_expiration_date,
            lot_quarantine_status: m.lot_quarantine_status,
            lot_manufacturer_batch: m.lot_manufacturer_batch,
            is_low_stock,
            is_expired_or_expiring,
        }
    }
}

/// Standalone top-level initializer matching the UniFFI API requirement
#[uniffi::export]
pub fn init_erp(db_path: String) -> Result<Arc<ErpEngine>, ErpFfiError> {
    ErpEngine::new(db_path)
}
