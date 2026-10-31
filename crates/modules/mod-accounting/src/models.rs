use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountRootType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl AccountRootType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Asset => "Asset",
            Self::Liability => "Liability",
            Self::Equity => "Equity",
            Self::Income => "Income",
            Self::Expense => "Expense",
        }
    }

    /// Debit increases Assets and Expenses; Credit increases Liabilities, Equity, and Income.
    pub fn is_debit_normal(&self) -> bool {
        matches!(self, Self::Asset | Self::Expense)
    }
}

impl FromStr for AccountRootType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "asset" => Self::Asset,
            "liability" => Self::Liability,
            "equity" => Self::Equity,
            "income" | "revenue" => Self::Income,
            "expense" => Self::Expense,
            _ => Self::Expense,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub code: String,
    pub name: String,
    pub root_type: AccountRootType,
    pub parent_id: Option<String>,
    pub is_group: bool,
    pub balance: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlEntry {
    pub id: String,
    pub entry_number: String,
    pub posting_date: String,
    pub account_id: String,
    pub party_type: Option<String>,
    pub party_id: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub against_account: Option<String>,
    pub voucher_type: String,
    pub voucher_id: String,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntryLine {
    pub account_id: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub party_type: Option<String>,
    pub party_id: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentType {
    Receive, // From customer (inbound)
    Pay,     // To supplier / vendor (outbound)
}

impl PaymentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Receive => "Receive",
            Self::Pay => "Pay",
        }
    }
}

impl FromStr for PaymentType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "receive" => Self::Receive,
            "pay" => Self::Pay,
            _ => Self::Receive,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEntry {
    pub id: String,
    pub payment_type: PaymentType,
    pub posting_date: String,
    pub party_id: String,
    pub paid_from_account_id: String,
    pub paid_to_account_id: String,
    pub amount: Decimal,
    pub reference_invoice_id: Option<String>,
    pub payment_method: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalanceRow {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub root_type: AccountRootType,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub net_balance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialStatement {
    pub title: String,
    pub as_of_or_period: String,
    pub rows: Vec<TrialBalanceRow>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub net_income_or_equity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainerAccount {
    pub id: String,
    pub customer_id: String,
    pub balance: Decimal,
    pub currency: String,
    pub liability_account_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosShift {
    pub id: String,
    pub cashier_id: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub opening_float: Decimal,
    pub cash_sales: Decimal,
    pub card_sales: Decimal,
    pub qr_sales: Decimal,
    pub closing_cash_actual: Option<Decimal>,
    pub discrepancy: Option<Decimal>,
    pub status: String, // "Open" or "Closed"
}
