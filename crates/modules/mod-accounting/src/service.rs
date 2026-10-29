use chrono::Utc;
use erp_kernel::ModuleContext;
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::AccountingError;
use crate::models::{
    Account, AccountRootType, FinancialStatement, GlEntry, JournalEntryLine, PaymentEntry,
    PaymentType, TrialBalanceRow,
};

pub struct AccountingService;

impl AccountingService {
    /// Seed a standard, production-ready Chart of Accounts if empty
    pub fn seed_default_chart_of_accounts(conn: &Connection) -> Result<(), AccountingError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM accounts",
            [],
            |r| r.get(0),
        )?;

        if count > 0 {
            return Ok(());
        }

        let now = Utc::now().to_rfc3339();

        let defaults = vec![
            // Assets
            ("1000", "Assets", AccountRootType::Asset, None, true),
            ("1110", "Cash on Hand", AccountRootType::Asset, Some("1000"), false),
            ("1120", "Operating Bank Account", AccountRootType::Asset, Some("1000"), false),
            ("1200", "Accounts Receivable (Debtors)", AccountRootType::Asset, Some("1000"), false),
            ("1300", "Inventory Asset (Stock in Hand)", AccountRootType::Asset, Some("1000"), false),

            // Liabilities
            ("2000", "Liabilities", AccountRootType::Liability, None, true),
            ("2100", "Accounts Payable (Creditors)", AccountRootType::Liability, Some("2000"), false),
            ("2200", "Sales Tax / VAT Payable", AccountRootType::Liability, Some("2000"), false),

            // Equity
            ("3000", "Equity", AccountRootType::Equity, None, true),
            ("3100", "Owner's Capital", AccountRootType::Equity, Some("3000"), false),
            ("3200", "Retained Earnings", AccountRootType::Equity, Some("3000"), false),

            // Income / Revenue
            ("4000", "Income", AccountRootType::Income, None, true),
            ("4100", "Sales Revenue", AccountRootType::Income, Some("4000"), false),
            ("4200", "Service & Consulting Revenue", AccountRootType::Income, Some("4000"), false),

            // Expenses
            ("5000", "Expenses", AccountRootType::Expense, None, true),
            ("5100", "Cost of Goods Sold (COGS)", AccountRootType::Expense, Some("5000"), false),
            ("5200", "Office Supplies & Hardware", AccountRootType::Expense, Some("5000"), false),
            ("5300", "Software & Cloud Services", AccountRootType::Expense, Some("5000"), false),
            ("5400", "Professional & Legal Fees", AccountRootType::Expense, Some("5000"), false),
        ];

        let mut stmt = conn.prepare(
            r#"
            INSERT INTO accounts (id, code, name, root_type, parent_id, is_group, balance, is_active, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, '0', 1, ?7)
            "#,
        )?;

        for (code, name, root_type, parent_code, is_group) in defaults {
            let id = format!("acc-{}", code);
            let parent_id = parent_code.map(|pc| format!("acc-{}", pc));
            stmt.execute(params![
                id,
                code,
                name,
                root_type.as_str(),
                parent_id,
                if is_group { 1 } else { 0 },
                now
            ])?;
        }

        Ok(())
    }

    /// Create a custom account in the Chart of Accounts tree
    pub fn create_account(
        ctx: &ModuleContext,
        code: &str,
        name: &str,
        root_type: AccountRootType,
        parent_id: Option<&str>,
        is_group: bool,
    ) -> Result<Account, AccountingError> {
        let account_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO accounts (id, code, name, root_type, parent_id, is_group, balance, is_active, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, '0', 1, ?7)
                "#,
                params![
                    account_id,
                    code,
                    name,
                    root_type.as_str(),
                    parent_id,
                    if is_group { 1 } else { 0 },
                    now.to_rfc3339()
                ],
            )?;
        }

        let account = Account {
            id: account_id.clone(),
            code: code.to_string(),
            name: name.to_string(),
            root_type,
            parent_id: parent_id.map(String::from),
            is_group,
            balance: Decimal::ZERO,
            is_active: true,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "AccountCreated",
            "Account",
            &account_id,
            serde_json::to_value(&account).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(account)
    }

    pub fn get_account_by_id(conn: &Connection, id: &str) -> Result<Account, AccountingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, root_type, parent_id, is_group, balance, is_active, created_at
            FROM accounts
            WHERE id = ?1
            "#,
        )?;
        stmt.query_row(params![id], Self::map_account_row)
            .map_err(|_| AccountingError::AccountNotFound(id.to_string()))
    }

    pub fn get_account_by_code(conn: &Connection, code: &str) -> Result<Account, AccountingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, root_type, parent_id, is_group, balance, is_active, created_at
            FROM accounts
            WHERE code = ?1
            "#,
        )?;
        stmt.query_row(params![code], Self::map_account_row)
            .map_err(|_| AccountingError::AccountNotFound(code.to_string()))
    }

    pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, AccountingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, root_type, parent_id, is_group, balance, is_active, created_at
            FROM accounts
            WHERE is_active = 1
            ORDER BY code ASC
            "#,
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Post a double-entry Journal Entry to the General Ledger.
    /// Strictly verifies the fundamental accounting equation: Total Debits == Total Credits.
    pub fn post_journal_entry(
        ctx: &ModuleContext,
        posting_date: &str,
        lines: Vec<JournalEntryLine>,
        voucher_type: &str,
        voucher_id: &str,
        remarks: Option<&str>,
    ) -> Result<Vec<GlEntry>, AccountingError> {
        if lines.len() < 2 {
            return Err(AccountingError::InvalidTransaction(
                "A journal entry requires at least two lines (one debit, one credit)".into(),
            ));
        }

        let mut total_debit = Decimal::ZERO;
        let mut total_credit = Decimal::ZERO;

        for line in &lines {
            if line.debit < Decimal::ZERO || line.credit < Decimal::ZERO {
                return Err(AccountingError::InvalidTransaction(
                    "Debit and Credit amounts must be non-negative".into(),
                ));
            }
            if line.debit > Decimal::ZERO && line.credit > Decimal::ZERO {
                return Err(AccountingError::InvalidTransaction(
                    "A single line cannot have both Debit and Credit amounts".into(),
                ));
            }
            total_debit += line.debit;
            total_credit += line.credit;
        }

        // ENFORCE DOUBLE-ENTRY INVARIANT
        if total_debit != total_credit {
            let diff = (total_debit - total_credit).abs();
            return Err(AccountingError::UnbalancedJournalEntry {
                debits: total_debit,
                credits: total_credit,
                diff,
            });
        }

        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        let now = Utc::now();
        let entry_number = format!("GL-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
        let mut created_entries = Vec::new();

        for line in lines {
            let acc = Self::get_account_by_id(&tx, &line.account_id)?;
            if acc.is_group {
                return Err(AccountingError::CannotPostToGroupAccount(acc.name));
            }

            let entry_id = Uuid::new_v4().to_string();

            tx.execute(
                r#"
                INSERT INTO gl_entries (
                    id, entry_number, posting_date, account_id, party_type, party_id,
                    debit, credit, voucher_type, voucher_id, remarks, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                params![
                    entry_id,
                    entry_number,
                    posting_date,
                    line.account_id,
                    line.party_type,
                    line.party_id,
                    line.debit.to_string(),
                    line.credit.to_string(),
                    voucher_type,
                    voucher_id,
                    line.remarks.as_deref().or(remarks),
                    now.to_rfc3339()
                ],
            )?;

            // Update materialized account balance
            let net_change = if acc.root_type.is_debit_normal() {
                line.debit - line.credit
            } else {
                line.credit - line.debit
            };

            let new_balance = acc.balance + net_change;
            tx.execute(
                "UPDATE accounts SET balance = ?1 WHERE id = ?2",
                params![new_balance.to_string(), line.account_id],
            )?;

            created_entries.push(GlEntry {
                id: entry_id,
                entry_number: entry_number.clone(),
                posting_date: posting_date.to_string(),
                account_id: line.account_id,
                party_type: line.party_type,
                party_id: line.party_id,
                debit: line.debit,
                credit: line.credit,
                against_account: None,
                voucher_type: voucher_type.to_string(),
                voucher_id: voucher_id.to_string(),
                remarks: line.remarks.or_else(|| remarks.map(String::from)),
                created_at: now,
            });
        }

        tx.commit()?;
        drop(conn);

        let _ = ctx.record_and_publish_event(
            "GlEntriesPosted",
            "GeneralLedger",
            &entry_number,
            serde_json::to_value(&created_entries).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(created_entries)
    }

    /// Record a payment (receive from customer or pay to supplier) and post balancing GL entry.
    #[allow(clippy::too_many_arguments)]
    pub fn record_payment(
        ctx: &ModuleContext,
        payment_type: PaymentType,
        party_id: &str,
        paid_from_account_id: &str,
        paid_to_account_id: &str,
        amount: Decimal,
        reference_invoice_id: Option<&str>,
        payment_method: &str,
    ) -> Result<PaymentEntry, AccountingError> {
        if amount <= Decimal::ZERO {
            return Err(AccountingError::InvalidTransaction(
                "Payment amount must be greater than zero".into(),
            ));
        }

        let payment_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let posting_date = now.format("%Y-%m-%d").to_string();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO payments (
                    id, payment_type, posting_date, party_id, paid_from_account_id,
                    paid_to_account_id, amount, reference_invoice_id, payment_method, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    payment_id,
                    payment_type.as_str(),
                    posting_date,
                    party_id,
                    paid_from_account_id,
                    paid_to_account_id,
                    amount.to_string(),
                    reference_invoice_id,
                    payment_method,
                    now.to_rfc3339()
                ],
            )?;
        }

        // Generate balancing GL entries
        // Paid To account gets debited, Paid From account gets credited
        let gl_lines = vec![
            JournalEntryLine {
                account_id: paid_to_account_id.to_string(),
                debit: amount,
                credit: Decimal::ZERO,
                party_type: Some(match payment_type {
                    PaymentType::Receive => "Customer".to_string(),
                    PaymentType::Pay => "Supplier".to_string(),
                }),
                party_id: Some(party_id.to_string()),
                remarks: Some(format!("Payment via {}", payment_method)),
            },
            JournalEntryLine {
                account_id: paid_from_account_id.to_string(),
                debit: Decimal::ZERO,
                credit: amount,
                party_type: None,
                party_id: None,
                remarks: Some(format!("Payment via {}", payment_method)),
            },
        ];

        Self::post_journal_entry(
            ctx,
            &posting_date,
            gl_lines,
            "Payment Entry",
            &payment_id,
            Some(&format!(
                "Payment of {} via {}",
                amount, payment_method
            )),
        )?;

        let payment = PaymentEntry {
            id: payment_id,
            payment_type,
            posting_date,
            party_id: party_id.to_string(),
            paid_from_account_id: paid_from_account_id.to_string(),
            paid_to_account_id: paid_to_account_id.to_string(),
            amount,
            reference_invoice_id: reference_invoice_id.map(String::from),
            payment_method: payment_method.to_string(),
            created_at: now,
        };

        Ok(payment)
    }

    /// Retrieve the Trial Balance report, verifying total debits == total credits across the ledger
    pub fn get_trial_balance(conn: &Connection) -> Result<FinancialStatement, AccountingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                a.id,
                a.code,
                a.name,
                a.root_type,
                COALESCE(SUM(CAST(g.debit AS REAL)), 0.0) AS total_debit,
                COALESCE(SUM(CAST(g.credit AS REAL)), 0.0) AS total_credit,
                a.balance
            FROM accounts a
            LEFT JOIN gl_entries g ON a.id = g.account_id
            WHERE a.is_group = 0
            GROUP BY a.id, a.code, a.name, a.root_type, a.balance
            ORDER BY a.code ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let code: String = row.get(1)?;
            let name: String = row.get(2)?;
            let root_type_str: String = row.get(3)?;
            let debit_f: f64 = row.get(4)?;
            let credit_f: f64 = row.get(5)?;
            let balance_str: String = row.get(6)?;

            let root_type = AccountRootType::from_str(&root_type_str).unwrap_or(AccountRootType::Expense);
            let total_debit = Decimal::from_str(&format!("{:.4}", debit_f)).unwrap_or(Decimal::ZERO);
            let total_credit = Decimal::from_str(&format!("{:.4}", credit_f)).unwrap_or(Decimal::ZERO);
            let net_balance = Decimal::from_str(&balance_str).unwrap_or(Decimal::ZERO);

            Ok(TrialBalanceRow {
                account_id: id,
                account_code: code,
                account_name: name,
                root_type,
                total_debit,
                total_credit,
                net_balance,
            })
        })?;

        let mut row_list = Vec::new();
        let mut sum_debit = Decimal::ZERO;
        let mut sum_credit = Decimal::ZERO;

        for r in rows {
            let row = r?;
            sum_debit += row.total_debit;
            sum_credit += row.total_credit;
            row_list.push(row);
        }

        let now = Utc::now().format("%Y-%m-%d").to_string();

        Ok(FinancialStatement {
            title: "Trial Balance".to_string(),
            as_of_or_period: now,
            rows: row_list,
            total_debit: sum_debit,
            total_credit: sum_credit,
            net_income_or_equity: Decimal::ZERO,
        })
    }

    /// Retrieve the Profit and Loss Statement (Income - Expenses = Net Profit)
    pub fn get_profit_and_loss(
        conn: &Connection,
        start_date: &str,
        end_date: &str,
    ) -> Result<FinancialStatement, AccountingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                a.id,
                a.code,
                a.name,
                a.root_type,
                COALESCE(SUM(CAST(g.debit AS REAL)), 0.0) AS total_debit,
                COALESCE(SUM(CAST(g.credit AS REAL)), 0.0) AS total_credit,
                a.balance
            FROM accounts a
            INNER JOIN gl_entries g ON a.id = g.account_id
            WHERE a.root_type IN ('Income', 'Expense')
              AND g.posting_date >= ?1 AND g.posting_date <= ?2
            GROUP BY a.id, a.code, a.name, a.root_type, a.balance
            ORDER BY a.code ASC
            "#,
        )?;

        let rows = stmt.query_map(params![start_date, end_date], |row| {
            let id: String = row.get(0)?;
            let code: String = row.get(1)?;
            let name: String = row.get(2)?;
            let root_type_str: String = row.get(3)?;
            let debit_f: f64 = row.get(4)?;
            let credit_f: f64 = row.get(5)?;
            let balance_str: String = row.get(6)?;

            let root_type = AccountRootType::from_str(&root_type_str).unwrap_or(AccountRootType::Expense);
            let total_debit = Decimal::from_str(&format!("{:.4}", debit_f)).unwrap_or(Decimal::ZERO);
            let total_credit = Decimal::from_str(&format!("{:.4}", credit_f)).unwrap_or(Decimal::ZERO);
            let net_balance = Decimal::from_str(&balance_str).unwrap_or(Decimal::ZERO);

            Ok(TrialBalanceRow {
                account_id: id,
                account_code: code,
                account_name: name,
                root_type,
                total_debit,
                total_credit,
                net_balance,
            })
        })?;

        let mut row_list = Vec::new();
        let mut total_income = Decimal::ZERO;
        let mut total_expense = Decimal::ZERO;

        for r in rows {
            let row = r?;
            if row.root_type == AccountRootType::Income {
                total_income += row.total_credit - row.total_debit;
            } else if row.root_type == AccountRootType::Expense {
                total_expense += row.total_debit - row.total_credit;
            }
            row_list.push(row);
        }

        let net_profit = total_income - total_expense;

        Ok(FinancialStatement {
            title: "Profit and Loss Statement".to_string(),
            as_of_or_period: format!("{} to {}", start_date, end_date),
            rows: row_list,
            total_debit: total_expense,
            total_credit: total_income,
            net_income_or_equity: net_profit,
        })
    }

    fn map_account_row(row: &Row<'_>) -> Result<Account, rusqlite::Error> {
        let id: String = row.get(0)?;
        let code: String = row.get(1)?;
        let name: String = row.get(2)?;
        let root_type_str: String = row.get(3)?;
        let parent_id: Option<String> = row.get(4)?;
        let is_group_int: i32 = row.get(5)?;
        let balance_str: String = row.get(6)?;
        let is_active_int: i32 = row.get(7)?;
        let created_at_str: String = row.get(8)?;

        let root_type = AccountRootType::from_str(&root_type_str).unwrap_or(AccountRootType::Expense);
        let balance = Decimal::from_str(&balance_str).unwrap_or(Decimal::ZERO);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Account {
            id,
            code,
            name,
            root_type,
            parent_id,
            is_group: is_group_int != 0,
            balance,
            is_active: is_active_int != 0,
            created_at,
        })
    }
}
