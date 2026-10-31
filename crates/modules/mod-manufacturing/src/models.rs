use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomComponent {
    pub id: String,
    pub bom_id: String,
    pub item_id: String,
    pub quantity: Decimal,
    pub scrap_percentage: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillOfMaterials {
    pub id: String,
    pub bom_number: String,
    pub item_id: String,
    pub name: String,
    pub quantity: Decimal,
    pub routing_notes: Option<String>,
    pub is_active: bool,
    pub components: Vec<BomComponent>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkOrderStatus {
    Planned,
    Draft,
    InProgress,
    QualityCheck,
    Completed,
    Cancelled,
}

impl WorkOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Draft => "Draft",
            Self::InProgress => "In Progress",
            Self::QualityCheck => "Quality Check",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
        }
    }
}

impl FromStr for WorkOrderStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "draft" => Self::Draft,
            "planned" => Self::Planned,
            "in progress" | "inprogress" => Self::InProgress,
            "quality check" | "qualitycheck" | "qc" => Self::QualityCheck,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            _ => Self::Planned,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentGenealogyEntry {
    pub component_item_id: String,
    pub component_sku: String,
    pub component_name: String,
    pub component_lot_number: Option<String>,
    pub component_quantity: Decimal,
    pub expiration_date: Option<String>,
    pub quarantine_status: Option<String>,
    pub manufacturer_batch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOrderGenealogyTree {
    pub work_order_id: String,
    pub work_order_number: String,
    pub finished_item_id: String,
    pub finished_sku: String,
    pub finished_name: String,
    pub finished_lot_number: String,
    pub quantity: Decimal,
    pub completed_at: Option<DateTime<Utc>>,
    pub components: Vec<ComponentGenealogyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOrder {
    pub id: String,
    pub work_order_number: String,
    pub bom_id: String,
    pub item_id: String,
    pub target_quantity: Decimal,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub status: WorkOrderStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
