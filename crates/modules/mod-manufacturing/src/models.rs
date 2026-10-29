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
    Draft,
    InProgress,
    Completed,
    Cancelled,
}

impl WorkOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
        }
    }
}

impl FromStr for WorkOrderStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "in progress" | "inprogress" => Self::InProgress,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        })
    }
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
