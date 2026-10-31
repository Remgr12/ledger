use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationType {
    Warehouse,
    Room,
    Zone,
    Aisle,
    Rack,
    Shelf,
    Bin,
    Container,
}

impl LocationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warehouse => "Warehouse",
            Self::Room => "Room",
            Self::Zone => "Zone",
            Self::Aisle => "Aisle",
            Self::Rack => "Rack",
            Self::Shelf => "Shelf",
            Self::Bin => "Bin",
            Self::Container => "Container",
        }
    }
}

impl FromStr for LocationType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Warehouse" => Self::Warehouse,
            "Room" => Self::Room,
            "Zone" => Self::Zone,
            "Aisle" => Self::Aisle,
            "Rack" => Self::Rack,
            "Shelf" => Self::Shelf,
            "Bin" => Self::Bin,
            "Container" => Self::Container,
            _ => Self::Bin,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLocation {
    pub id: String,
    pub code: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub location_type: LocationType,
    pub path_breadcrumb: String,
    pub qr_code_uri: String,
    pub map_x: Option<f64>,
    pub map_y: Option<f64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationTreeNode {
    pub location: StorageLocation,
    pub children: Vec<LocationTreeNode>,
    pub total_item_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub track_serial: bool,
    pub track_lot: bool,
    pub min_stock_level: Decimal,
    pub qr_code_uri: String,
    pub barcode: Option<String>,
    pub standard_cost: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Receive,
    Putaway,
    Transfer,
    Pick,
    Adjust,
    Scrap,
}

impl MovementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Receive => "Receive",
            Self::Putaway => "Putaway",
            Self::Transfer => "Transfer",
            Self::Pick => "Pick",
            Self::Adjust => "Adjust",
            Self::Scrap => "Scrap",
        }
    }
}

impl FromStr for MovementType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Receive" => Self::Receive,
            "Putaway" => Self::Putaway,
            "Transfer" => Self::Transfer,
            "Pick" => Self::Pick,
            "Adjust" => Self::Adjust,
            "Scrap" => Self::Scrap,
            _ => Self::Transfer,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub movement_type: MovementType,
    pub item_id: String,
    pub from_location_id: Option<String>,
    pub to_location_id: Option<String>,
    pub quantity: Decimal,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
    pub hlc_timestamp: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuant {
    pub id: String,
    pub item_id: String,
    pub location_id: String,
    pub quantity: Decimal,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub updated_at: DateTime<Utc>,
}

/// Lot tracking model for quality, compliance, and expiration monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotInfo {
    pub lot_number: String,
    pub item_id: String,
    pub expiration_date: Option<String>,
    pub quarantine_status: String,
    pub manufacturer_batch: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Instant "Where Is It?" search result model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemLocationMatch {
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
    pub quantity: Decimal,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub min_stock_level: Decimal,
    pub lot_expiration_date: Option<String>,
    pub lot_quarantine_status: Option<String>,
    pub lot_manufacturer_batch: Option<String>,
}

/// Parsed GS1 Barcode with Standard Application Identifiers (AI)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gs1BarcodeData {
    pub gtin: Option<String>,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
    pub expiration_date: Option<String>,
    pub count: Option<Decimal>,
}

/// Parsed result of an erp:// QR code or standard barcode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedQr {
    Location {
        id_or_code: String,
    },
    Item {
        sku: String,
        lot: Option<String>,
        serial: Option<String>,
    },
    Serial {
        serial_number: String,
    },
    Gs1(Gs1BarcodeData),
    ExternalBarcode {
        raw: String,
    },
}

/// Supported offline label print formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelFormat {
    ThermalRoll62mm, // Brother QL / Dymo continuous 62mm
    Thermal4x6,      // Zebra / standard 4x6" shipping & bin label
    A4StickerSheet,  // 24-up A4 sheet (3x8 grid)
}

/// Output of QR Resolution Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanResolution {
    LocationContext {
        location: StorageLocation,
        items: Vec<ItemLocationMatch>,
        prompt_message: String,
    },
    ItemLookup {
        item: InventoryItem,
        locations: Vec<ItemLocationMatch>,
        total_quantity: Decimal,
    },
    RapidStockAction {
        item: InventoryItem,
        target_location: StorageLocation,
        current_quantity_at_location: Decimal,
        suggested_action: String,
    },
    AuditCount {
        location: StorageLocation,
        item: InventoryItem,
        system_expected_qty: Decimal,
        scanned_count: Decimal,
        discrepancy: Decimal,
    },
    Unknown {
        raw_payload: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedAsset {
    pub id: String,
    pub item_id: String,
    pub serial_number: String,
    pub status: String,
    pub warranty_start_date: Option<String>,
    pub warranty_end_date: Option<String>,
    pub customer_id: Option<String>,
    pub current_location_id: Option<String>,
    pub next_calibration_date: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMaintenanceLog {
    pub id: String,
    pub asset_id: String,
    pub log_type: String,
    pub notes: String,
    pub performed_by: String,
    pub service_date: String,
    pub next_due_date: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmaClaim {
    pub id: String,
    pub rma_number: String,
    pub asset_id: String,
    pub customer_id: Option<String>,
    pub issue_description: String,
    pub status: String,
    pub resolution: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransferLine {
    pub id: String,
    pub transfer_id: String,
    pub item_id: String,
    pub sku: String,
    pub item_name: String,
    pub requested_qty: Decimal,
    pub dispatched_qty: Decimal,
    pub received_qty: Decimal,
    pub variance_qty: Decimal,
    pub variance_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransferOrder {
    pub id: String,
    pub transfer_number: String,
    pub origin_location_id: String,
    pub origin_location_name: String,
    pub destination_location_id: String,
    pub destination_location_name: String,
    pub in_transit_location_id: String,
    pub status: String,
    pub carrier_name: Option<String>,
    pub tracking_number: Option<String>,
    pub driver_signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub dispatched_at: Option<String>,
    pub received_at: Option<String>,
    pub lines: Vec<StockTransferLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbcItemTier {
    pub item_id: String,
    pub sku: String,
    pub name: String,
    pub annual_consumption_value: Decimal,
    pub cumulative_value_percentage: Decimal,
    pub tier: String, // "A", "B", "C"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbcAnalysisResult {
    pub items: Vec<AbcItemTier>,
    pub total_consumption_value: Decimal,
    pub class_a_count: usize,
    pub class_b_count: usize,
    pub class_c_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAging {
    pub location_id: String,
    pub location_code: String,
    pub location_name: String,
    pub map_x: Option<f64>,
    pub map_y: Option<f64>,
    pub days_since_last_movement: u32,
    pub total_quant_value: Decimal,
    pub aging_status: String, // "Fresh" (0-30d), "Normal" (31-90d), "Aging" (91-180d), "DeadStock" (>180d)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EoqCalculation {
    pub item_id: String,
    pub sku: String,
    pub recommended_eoq: Decimal,
    pub dynamic_safety_stock: Decimal,
    pub reorder_point: Decimal,
}
