use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Open,
    InProgress,
    Completed,
    OnHold,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
            Self::OnHold => "On Hold",
        }
    }
}

impl FromStr for ProjectStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "in progress" | "inprogress" => Self::InProgress,
            "completed" => Self::Completed,
            "on hold" | "onhold" => Self::OnHold,
            _ => Self::Open,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub code: String,
    pub customer_id: Option<String>,
    pub status: ProjectStatus,
    pub billing_rate_hourly: Decimal,
    pub estimated_hours: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTask {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub estimated_hours: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimesheetEntry {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub employee_or_user: String,
    pub activity_date: String,
    pub hours: Decimal,
    pub hourly_rate: Decimal,
    pub billing_amount: Decimal,
    pub is_billable: bool,
    pub is_invoiced: bool,
    pub invoice_id: Option<String>,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
