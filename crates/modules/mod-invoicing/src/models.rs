use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Finalized,
    Paid,
    Cancelled,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Finalized => "Finalized",
            Self::Paid => "Paid",
            Self::Cancelled => "Cancelled",
        }
    }
}

impl FromStr for InvoiceStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Draft" => Self::Draft,
            "Finalized" => Self::Finalized,
            "Paid" => Self::Paid,
            "Cancelled" => Self::Cancelled,
            _ => Self::Draft,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub id: String,
    pub item_id: Option<String>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal, // e.g. 0.20 for 20%
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub invoice_number: String,
    pub customer_id: String,
    pub status: InvoiceStatus,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub grand_total: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub line_items: Vec<InvoiceLineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringSchedule {
    pub id: String,
    pub customer_id: String,
    pub plan_name: String,
    pub frequency: String, // "Weekly", "Monthly", "Quarterly", "Annually"
    pub amount: Decimal,
    pub currency: String,
    pub next_run_date: String,
    pub is_active: bool,
    pub auto_send: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EInvoiceParty {
    pub name: String,
    pub vat_id: String,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country_code: String,
    pub iban: Option<String>,
    pub bic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EInvoiceValidationReport {
    pub is_valid: bool,
    pub standard_profile: String, // "Factur-X EN 16931 (COMFORT)"
    pub rule_checks_passed: usize,
    pub validation_messages: Vec<String>,
}
