use serde::{Deserialize, Serialize};

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ModuleRouteDto {
    pub route_id: String,
    pub title: String,
    pub icon: String,
    pub display_order: u32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfoDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub is_enabled: bool,
    pub installed_version: u32,
    pub routes: Vec<ModuleRouteDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct LocationDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub location_type: String,
    pub path_breadcrumb: String,
    pub qr_code_uri: String,
    pub map_x: Option<f64>,
    pub map_y: Option<f64>,
    pub is_active: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct LocationNodeDto {
    pub location: LocationDto,
    pub children: Vec<LocationNodeDto>,
    pub total_item_count: u64,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ItemDto {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub track_serial: bool,
    pub track_lot: bool,
    pub min_stock_level: String,
    pub qr_code_uri: String,
    pub barcode: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ItemLocationCardDto {
    pub item_id: String,
    pub sku: String,
    pub item_name: String,
    pub unit: String,
    pub location_id: String,
    pub location_code: String,
    pub location_name: String,
    pub location_type: String,
    pub breadcrumb_path: String,
    pub map_x: Option<f64>,
    pub map_y: Option<f64>,
    pub quantity: String,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub min_stock_level: String,
    pub lot_expiration_date: Option<String>,
    pub lot_quarantine_status: Option<String>,
    pub lot_manufacturer_batch: Option<String>,
    pub is_low_stock: bool,
    pub is_expired_or_expiring: bool,
}

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanResolutionTypeDto {
    LocationContext,
    ItemLookup,
    RapidStockAction,
    AuditCount,
    Unknown,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ScanResolutionDto {
    pub resolution_type: ScanResolutionTypeDto,
    pub prompt_message: String,
    pub location: Option<LocationDto>,
    pub item: Option<ItemDto>,
    pub item_locations: Vec<ItemLocationCardDto>,
    pub total_quantity: Option<String>,
    pub current_quantity_at_location: Option<String>,
    pub suggested_action: Option<String>,
    pub raw_payload: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ContactDto {
    pub id: String,
    pub contact_type: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceLineDto {
    pub id: String,
    pub item_id: Option<String>,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub tax_rate: String,
    pub line_total: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceLineInputDto {
    pub item_id: Option<String>,
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub tax_rate: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceDto {
    pub id: String,
    pub invoice_number: String,
    pub customer_id: String,
    pub status: String,
    pub subtotal: String,
    pub tax_total: String,
    pub grand_total: String,
    pub notes: Option<String>,
    pub lines: Vec<InvoiceLineDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PluginUiNodeDto {
    pub node_id: String,
    pub node_kind: String, // "Container", "Text", "Badge", "StatCard", "Button", "Table"
    pub title_or_label: String,
    pub secondary_text: Option<String>,
    pub color_hex: Option<String>,
    pub custom_field_key: Option<String>,
    pub children: Vec<PluginUiNodeDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifestDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
}

// ==========================================
// 1. Accounting DTOs
// ==========================================

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct AccountDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub root_type: String,
    pub parent_id: Option<String>,
    pub is_group: bool,
    pub balance: String,
    pub is_active: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct GlEntryDto {
    pub id: String,
    pub entry_number: String,
    pub posting_date: String,
    pub account_id: String,
    pub party_type: Option<String>,
    pub party_id: Option<String>,
    pub debit: String,
    pub credit: String,
    pub voucher_type: String,
    pub voucher_id: String,
    pub remarks: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct JournalLineInputDto {
    pub account_id: String,
    pub debit: String,
    pub credit: String,
    pub party_type: Option<String>,
    pub party_id: Option<String>,
    pub remarks: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PaymentEntryDto {
    pub id: String,
    pub payment_type: String,
    pub posting_date: String,
    pub party_id: String,
    pub paid_from_account_id: String,
    pub paid_to_account_id: String,
    pub amount: String,
    pub reference_invoice_id: Option<String>,
    pub payment_method: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct TrialBalanceRowDto {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub root_type: String,
    pub total_debit: String,
    pub total_credit: String,
    pub net_balance: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct FinancialStatementDto {
    pub title: String,
    pub as_of_or_period: String,
    pub rows: Vec<TrialBalanceRowDto>,
    pub total_debit: String,
    pub total_credit: String,
    pub net_income_or_equity: String,
}

// ==========================================
// 2. Purchasing DTOs
// ==========================================

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseOrderLineDto {
    pub id: String,
    pub item_id: String,
    pub description: String,
    pub quantity: String,
    pub received_quantity: String,
    pub unit_price: String,
    pub tax_rate: String,
    pub line_total: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseOrderDto {
    pub id: String,
    pub po_number: String,
    pub supplier_id: String,
    pub order_date: String,
    pub expected_delivery: Option<String>,
    pub status: String,
    pub subtotal: String,
    pub tax_total: String,
    pub grand_total: String,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseOrderLineDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseReceiptLineDto {
    pub id: String,
    pub item_id: String,
    pub quantity: String,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseReceiptDto {
    pub id: String,
    pub receipt_number: String,
    pub purchase_order_id: Option<String>,
    pub supplier_id: String,
    pub destination_location_id: String,
    pub received_at: String,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseReceiptLineDto>,
}

// ==========================================
// 3. Manufacturing DTOs
// ==========================================

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BomComponentDto {
    pub id: String,
    pub item_id: String,
    pub quantity: String,
    pub scrap_percentage: String,
    pub notes: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BillOfMaterialsDto {
    pub id: String,
    pub bom_number: String,
    pub item_id: String,
    pub name: String,
    pub quantity: String,
    pub routing_notes: Option<String>,
    pub components: Vec<BomComponentDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct WorkOrderDto {
    pub id: String,
    pub work_order_number: String,
    pub bom_id: String,
    pub item_id: String,
    pub target_quantity: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

// ==========================================
// 4. Projects & Timesheets DTOs
// ==========================================

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub code: String,
    pub customer_id: Option<String>,
    pub status: String,
    pub billing_rate_hourly: String,
    pub estimated_hours: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ProjectTaskDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub estimated_hours: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct TimesheetEntryDto {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub employee_or_user: String,
    pub activity_date: String,
    pub hours: String,
    pub hourly_rate: String,
    pub billing_amount: String,
    pub is_billable: bool,
    pub is_invoiced: bool,
    pub invoice_id: Option<String>,
    pub description: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct LowStockAlertDto {
    pub item_id: String,
    pub sku: String,
    pub name: String,
    pub unit: String,
    pub current_stock: String,
    pub min_stock_level: String,
    pub suggested_reorder_qty: String,
    pub preferred_supplier_id: Option<String>,
    pub preferred_supplier_name: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PeerNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub ip_address: String,
    pub port: u16,
    pub last_seen_seconds_ago: u64,
    pub is_synced: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct SyncStatusDto {
    pub connected_peers_count: u32,
    pub unsynced_events_count: u32,
    pub total_events_count: u32,
    pub last_sync_timestamp: String,
    pub convergence_state: String,
    pub local_hlc: String,
    pub peers: Vec<PeerNodeDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct UnbilledHoursSummaryDto {
    pub total_unbilled_hours: String,
    pub total_unbilled_amount: String,
    pub unbilled_entries_count: u32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct LotInfoDto {
    pub lot_number: String,
    pub item_id: String,
    pub expiration_date: Option<String>,
    pub quarantine_status: String,
    pub manufacturer_batch: Option<String>,
    pub created_at: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ComponentGenealogyEntryDto {
    pub component_item_id: String,
    pub component_sku: String,
    pub component_name: String,
    pub component_lot_number: Option<String>,
    pub component_quantity: String,
    pub expiration_date: Option<String>,
    pub quarantine_status: Option<String>,
    pub manufacturer_batch: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ComponentGenealogyDto {
    pub work_order_id: String,
    pub work_order_number: String,
    pub finished_item_id: String,
    pub finished_sku: String,
    pub finished_name: String,
    pub finished_lot_number: String,
    pub quantity: String,
    pub completed_at: Option<String>,
    pub components: Vec<ComponentGenealogyEntryDto>,
}

// ============================================================================
// v0.0.4 Release DTOs (LEDGER-16 to LEDGER-28)
// ============================================================================

// LEDGER-16: Multi-up Sheet PDF Layouts & Barcode Printing
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct MultiUpLabelItemDto {
    pub item_id: String,
    pub sku: String,
    pub name: String,
    pub qr_code_uri: String,
    pub barcode: Option<String>,
    pub lot_or_serial: Option<String>,
    pub price: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct MultiUpSheetLayoutDto {
    pub name: String,
    pub columns: u32,
    pub rows: u32,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub label_width_mm: f64,
    pub label_height_mm: f64,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BatchPrintProgressDto {
    pub total_labels: u32,
    pub printed_labels: u32,
    pub is_completed: bool,
}

// LEDGER-17: BLE Barcode Scanners & Digital Weight Scale Integration
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BleDeviceDto {
    pub id: String,
    pub name: String,
    pub address: String,
    pub device_type: String, // "scanner" | "scale" | "printer"
    pub is_connected: bool,
    pub battery_level: Option<u8>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ScaleReadingDto {
    pub raw_weight_grams: f64,
    pub tare_weight_grams: f64,
    pub net_weight_grams: f64,
    pub is_stable: bool,
    pub unit: String,
    pub unit_piece_weight_grams: Option<f64>,
    pub calculated_pieces: Option<u32>,
}

// LEDGER-18: Multi-Currency Accounting & Offline FX Tables
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ExchangeRateDto {
    pub base_currency: String,
    pub target_currency: String,
    pub rate: f64,
    pub effective_date: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct FxGainLossResultDto {
    pub invoice_id: String,
    pub realized_gain_loss: String,
    pub currency: String,
    pub journal_entry_id: Option<String>,
}

// LEDGER-19: Recurring Subscription Invoicing & Client Retainer Deposits
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct RecurringScheduleDto {
    pub id: String,
    pub customer_id: String,
    pub frequency: String,
    pub next_run_date: String,
    pub amount: String,
    pub currency: String,
    pub description: String,
    pub is_active: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct RetainerAccountDto {
    pub customer_id: String,
    pub balance: String,
    pub currency: String,
    pub last_updated: String,
}

// LEDGER-20: ABC Inventory Pareto Analysis, Aging Heatmaps, Dynamic Safety Stock & EOQ
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct AbcItemTierDto {
    pub item_id: String,
    pub sku: String,
    pub name: String,
    pub annual_value: f64,
    pub cumulative_percentage: f64,
    pub tier: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct AbcAnalysisDto {
    pub total_value: f64,
    pub tier_a_count: u32,
    pub tier_b_count: u32,
    pub tier_c_count: u32,
    pub items: Vec<AbcItemTierDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct LocationAgingDto {
    pub location_id: String,
    pub location_code: String,
    pub average_age_days: f64,
    pub oldest_item_days: u32,
    pub color_code: String,
    pub item_count: u32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct EoqCalculationDto {
    pub item_id: String,
    pub sku: String,
    pub economic_order_quantity: f64,
    pub safety_stock: f64,
    pub reorder_point: f64,
    pub annual_demand: f64,
}

// LEDGER-21: Database Encryption at Rest & Authenticated P2P Sync
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct SecurityStatusDto {
    pub is_encrypted: bool,
    pub cipher_suite: String,
    pub known_psk_peers_count: u32,
    pub local_node_id: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PeerPairingPayloadDto {
    pub node_id: String,
    pub psk: String,
    pub qr_uri: String,
    pub fingerprint: String,
}

// LEDGER-22: On-Device Offline OCR Intake for Receipts & Vendor Invoices
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct OcrBoundingBoxDto {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct OcrReceiptLineDto {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_amount: f64,
    pub confidence: f32,
    pub bounding_box: Option<OcrBoundingBoxDto>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct OcrReceiptResultDto {
    pub merchant_name: Option<String>,
    pub date: Option<String>,
    pub total_amount: Option<f64>,
    pub tax_amount: Option<f64>,
    pub currency: Option<String>,
    pub lines: Vec<OcrReceiptLineDto>,
    pub confidence: f32,
    pub raw_text: String,
}

// LEDGER-23: Serial Number Tracking, Warranty Passport & Maintenance Logs
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct SerializedAssetPassportDto {
    pub id: String,
    pub serial_number: String,
    pub item_id: String,
    pub item_name: String,
    pub sku: String,
    pub status: String,
    pub warranty_expiry: Option<String>,
    pub current_location_id: Option<String>,
    pub created_at: String,
    pub qr_code_uri: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct AssetMaintenanceLogDto {
    pub id: String,
    pub asset_id: String,
    pub maintenance_type: String,
    pub description: String,
    pub performed_by: String,
    pub performed_at: String,
    pub next_due_date: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct RmaClaimDto {
    pub id: String,
    pub rma_number: String,
    pub asset_id: String,
    pub customer_id: String,
    pub issue_description: String,
    pub status: String,
    pub resolution: Option<String>,
    pub created_at: String,
}

// LEDGER-24: Offline POS Cashier Mode, Cash Drawer Kick & Shift Reconciliation
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct PosShiftDto {
    pub id: String,
    pub register_id: String,
    pub cashier_id: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub starting_cash: String,
    pub cash_sales: String,
    pub expected_cash: String,
    pub actual_cash: Option<String>,
    pub variance: Option<String>,
    pub status: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShiftReportDto {
    pub shift_id: String,
    pub report_type: String,
    pub generated_at: String,
    pub content: String,
}

// LEDGER-25: Multi-Warehouse Transfer Orders & In-Transit Tracking
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct StockTransferLineDto {
    pub id: String,
    pub item_id: String,
    pub sku: String,
    pub item_name: String,
    pub requested_qty: String,
    pub shipped_qty: String,
    pub received_qty: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct StockTransferOrderDto {
    pub id: String,
    pub transfer_number: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub in_transit_location_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub dispatched_at: Option<String>,
    pub received_at: Option<String>,
    pub lines: Vec<StockTransferLineDto>,
}

// LEDGER-26: Workstation Capacity Planning & Gantt Production Scheduling
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct WorkstationDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub capacity_hours_per_day: f64,
    pub hourly_rate: f64,
    pub is_active: bool,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct RoutingOperationDto {
    pub id: String,
    pub bom_id: String,
    pub workstation_id: String,
    pub operation_name: String,
    pub sequence: u32,
    pub standard_duration_minutes: u32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct WorkOrderAllocationDto {
    pub id: String,
    pub work_order_id: String,
    pub workstation_id: String,
    pub operation_id: String,
    pub start_time: String,
    pub end_time: String,
    pub allocated_minutes: u32,
    pub status: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct WorkstationCapacitySummaryDto {
    pub workstation_id: String,
    pub workstation_code: String,
    pub date: String,
    pub capacity_minutes: u32,
    pub allocated_minutes: u32,
    pub utilization_percent: f64,
    pub is_overloaded: bool,
}

// LEDGER-27: Automated NAS/SFTP Backup Scheduler & USB Drive Sync
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BackupArchiveDto {
    pub filename: String,
    pub filepath: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub is_verified: bool,
    pub checksum: String,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct BackupPolicyConfigDto {
    pub auto_backup_enabled: bool,
    pub target_directory: String,
    pub retention_count: u32,
    pub backup_frequency_hours: u32,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct UsbDriveInfoDto {
    pub device_path: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

// LEDGER-28: European E-Invoicing Compliance (Factur-X / ZUGFeRD 2.2 / UBL 2.1)
#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct EInvoiceValidationReportDto {
    pub is_valid: bool,
    pub profile: String,
    pub syntax: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct IncomingEInvoiceDto {
    pub invoice_number: String,
    pub seller_name: String,
    pub seller_vat: Option<String>,
    pub issue_date: String,
    pub total_amount: f64,
    pub currency: String,
    pub lines_count: u32,
    pub xml_content: String,
}
