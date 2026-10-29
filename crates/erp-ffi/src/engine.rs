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

        // Seed initial default accounts
        let _ = AccountingService::seed_default_chart_of_accounts(&db.conn());

        let plugin_host = Arc::new(PluginHost::new());

        Ok(Arc::new(Self {
            db,
            hlc,
            bus,
            module_manager,
            plugin_host,
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
        Ok(matches
            .into_iter()
            .map(|m| ItemLocationCardDto {
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
            })
            .collect())
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
            message: format!("Invalid quantity decimal '{}': {}", qty, e),
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
                item_locations: items
                    .into_iter()
                    .map(|m| ItemLocationCardDto {
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
                    })
                    .collect(),
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
                item_locations: locations
                    .into_iter()
                    .map(|m| ItemLocationCardDto {
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
                    })
                    .collect(),
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
            message: format!("Invalid count decimal: {}", e),
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
            message: e.to_string(),
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
            message: e.to_string(),
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
                message: format!("Invalid manifest JSON: {}", e),
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
            message: format!("Invalid payment amount: {}", e),
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
}

/// Standalone top-level initializer matching the UniFFI API requirement
#[uniffi::export]
pub fn init_erp(db_path: String) -> Result<Arc<ErpEngine>, ErpFfiError> {
    ErpEngine::new(db_path)
}
