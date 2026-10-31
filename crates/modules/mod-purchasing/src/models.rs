use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseOrderStatus {
    Draft,
    Submitted,
    PartiallyReceived,
    Received,
    Cancelled,
}

impl PurchaseOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Submitted => "Submitted",
            Self::PartiallyReceived => "Partially Received",
            Self::Received => "Received",
            Self::Cancelled => "Cancelled",
        }
    }
}

impl FromStr for PurchaseOrderStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "submitted" => Self::Submitted,
            "partially received" | "partiallyreceived" => Self::PartiallyReceived,
            "received" => Self::Received,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub id: String,
    pub purchase_order_id: String,
    pub item_id: String,
    pub description: String,
    pub quantity: Decimal,
    pub received_quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: String,
    pub po_number: String,
    pub supplier_id: String,
    pub order_date: String,
    pub expected_delivery: Option<String>,
    pub status: PurchaseOrderStatus,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub grand_total: Decimal,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseOrderLine>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptLine {
    pub id: String,
    pub receipt_id: String,
    pub item_id: String,
    pub quantity: Decimal,
    pub lot_number: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceipt {
    pub id: String,
    pub receipt_number: String,
    pub purchase_order_id: Option<String>,
    pub supplier_id: String,
    pub destination_location_id: String,
    pub received_at: DateTime<Utc>,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseReceiptLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub field_name: String,
    pub recognized_value: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrReceiptLine {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total: Decimal,
    pub tax_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrReceiptResult {
    pub id: String,
    pub file_name: String,
    pub vendor_name: String,
    pub invoice_date: String,
    pub currency: String,
    pub subtotal: Decimal,
    pub tax_rate: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub lines: Vec<OcrReceiptLine>,
    pub bounding_boxes: Vec<OcrBoundingBox>,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingEInvoice {
    pub invoice_number: String,
    pub supplier_name: String,
    pub supplier_vat_id: Option<String>,
    pub invoice_date: String,
    pub currency: String,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub grand_total: Decimal,
    pub lines: Vec<OcrReceiptLine>,
}
