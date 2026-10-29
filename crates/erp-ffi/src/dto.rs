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
