use erp_kernel::{DomainEvent, ErpKernelError, ErpModule, Migration, ModuleContext, ModuleRoute};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::models::JournalEntryLine;
use crate::service::AccountingService;

pub struct AccountingModule;

impl Default for AccountingModule {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountingModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for AccountingModule {
    fn id(&self) -> &'static str {
        "mod-accounting"
    }

    fn name(&self) -> &'static str {
        "General Ledger & Double-Entry Accounting"
    }

    fn description(&self) -> &'static str {
        "Chart of Accounts, balanced General Ledger, Journal Entries, Payment reconciliation, Trial Balance, P&L"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "create_accounting_tables",
                sql: r#"
                CREATE TABLE IF NOT EXISTS accounts (
                    id TEXT PRIMARY KEY,
                    code TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    root_type TEXT NOT NULL,
                    parent_id TEXT,
                    is_group INTEGER NOT NULL DEFAULT 0,
                    balance TEXT NOT NULL DEFAULT '0',
                    is_active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(parent_id) REFERENCES accounts(id)
                );
                CREATE INDEX IF NOT EXISTS idx_accounts_code ON accounts(code);
                CREATE INDEX IF NOT EXISTS idx_accounts_root ON accounts(root_type);

                CREATE TABLE IF NOT EXISTS gl_entries (
                    id TEXT PRIMARY KEY,
                    entry_number TEXT NOT NULL,
                    posting_date TEXT NOT NULL,
                    account_id TEXT NOT NULL,
                    party_type TEXT,
                    party_id TEXT,
                    debit TEXT NOT NULL DEFAULT '0',
                    credit TEXT NOT NULL DEFAULT '0',
                    against_account TEXT,
                    voucher_type TEXT NOT NULL,
                    voucher_id TEXT NOT NULL,
                    remarks TEXT,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(account_id) REFERENCES accounts(id)
                );
                CREATE INDEX IF NOT EXISTS idx_gl_account ON gl_entries(account_id);
                CREATE INDEX IF NOT EXISTS idx_gl_voucher ON gl_entries(voucher_type, voucher_id);
                CREATE INDEX IF NOT EXISTS idx_gl_date ON gl_entries(posting_date);

                CREATE TABLE IF NOT EXISTS payments (
                    id TEXT PRIMARY KEY,
                    payment_type TEXT NOT NULL,
                    posting_date TEXT NOT NULL,
                    party_id TEXT NOT NULL,
                    paid_from_account_id TEXT NOT NULL,
                    paid_to_account_id TEXT NOT NULL,
                    amount TEXT NOT NULL,
                    reference_invoice_id TEXT,
                    payment_method TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(paid_from_account_id) REFERENCES accounts(id),
                    FOREIGN KEY(paid_to_account_id) REFERENCES accounts(id)
                );
                CREATE INDEX IF NOT EXISTS idx_payments_party ON payments(party_id);
                CREATE INDEX IF NOT EXISTS idx_payments_invoice ON payments(reference_invoice_id);
                "#,
            },
        ]
    }

    fn on_event(&self, event: &DomainEvent, ctx: &ModuleContext) -> Result<(), ErpKernelError> {
        // Ensure default accounts exist
        let _ = AccountingService::seed_default_chart_of_accounts(&ctx.db.conn());

        if event.event_type == "InvoiceFinalized" {
            let payload = &event.payload;
            let invoice_id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let inv_number = payload
                .get("invoice_number")
                .and_then(|v| v.as_str())
                .unwrap_or("INV-UNKNOWN");
            let customer_id = payload
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let grand_total = payload
                .get("grand_total")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or(Decimal::ZERO);
            let subtotal = payload
                .get("subtotal")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or(Decimal::ZERO);
            let tax_total = payload
                .get("tax_total")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or(Decimal::ZERO);

            if grand_total > Decimal::ZERO {
                let conn = ctx.db.conn();
                let ar_acc = AccountingService::get_account_by_code(&conn, "1200")
                    .map_err(|e| ErpKernelError::Module(e.to_string()))?;
                let rev_acc = AccountingService::get_account_by_code(&conn, "4100")
                    .map_err(|e| ErpKernelError::Module(e.to_string()))?;
                let tax_acc = AccountingService::get_account_by_code(&conn, "2200")
                    .map_err(|e| ErpKernelError::Module(e.to_string()))?;
                drop(conn);

                let now_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

                let mut gl_lines = vec![
                    // Debit Accounts Receivable for the full grand total
                    JournalEntryLine {
                        account_id: ar_acc.id,
                        debit: grand_total,
                        credit: Decimal::ZERO,
                        party_type: Some("Customer".to_string()),
                        party_id: Some(customer_id.to_string()),
                        remarks: Some(format!("Sales invoice {}", inv_number)),
                    },
                    // Credit Sales Revenue for the net subtotal
                    JournalEntryLine {
                        account_id: rev_acc.id,
                        debit: Decimal::ZERO,
                        credit: subtotal,
                        party_type: None,
                        party_id: None,
                        remarks: Some(format!("Sales revenue for {}", inv_number)),
                    },
                ];

                // If tax exists, credit Sales Tax Payable
                if tax_total > Decimal::ZERO {
                    gl_lines.push(JournalEntryLine {
                        account_id: tax_acc.id,
                        debit: Decimal::ZERO,
                        credit: tax_total,
                        party_type: None,
                        party_id: None,
                        remarks: Some(format!("Sales tax on {}", inv_number)),
                    });
                }

                AccountingService::post_journal_entry(
                    ctx,
                    &now_date,
                    gl_lines,
                    "Sales Invoice",
                    invoice_id,
                    Some(&format!("Automated GL posting for finalized invoice {}", inv_number)),
                )
                .map_err(|e| ErpKernelError::Module(e.to_string()))?;
            }
        }

        Ok(())
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "accounting/gl".to_string(),
                title: "General Ledger".to_string(),
                icon: "account_balance".to_string(),
                display_order: 40,
            },
            ModuleRoute {
                route_id: "accounting/coa".to_string(),
                title: "Chart of Accounts".to_string(),
                icon: "account_tree".to_string(),
                display_order: 41,
            },
            ModuleRoute {
                route_id: "accounting/trial_balance".to_string(),
                title: "Trial Balance".to_string(),
                icon: "assessment".to_string(),
                display_order: 42,
            },
        ]
    }
}
