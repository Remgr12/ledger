use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactType {
    Customer,
    Supplier,
    Both,
}

impl ContactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Customer => "Customer",
            Self::Supplier => "Supplier",
            Self::Both => "Both",
        }
    }
}

impl FromStr for ContactType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Customer" => Self::Customer,
            "Supplier" => Self::Supplier,
            _ => Self::Both,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub contact_type: ContactType,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
