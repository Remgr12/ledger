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
            ("2150", "Customer Retainer Deposits (Deferred Revenue)", AccountRootType::Liability, Some("2000"), false),
            ("2200", "Sales Tax / VAT Payable", AccountRootType::Liability, Some("2000"), false),

            // Equity
            ("3000", "Equity", AccountRootType::Equity, None, true),
            ("3100", "Owner's Capital", AccountRootType::Equity, Some("3000"), false),
            ("3200", "Retained Earnings", AccountRootType::Equity, Some("3000"), false),

            // Income / Revenue
            ("4000", "Income", AccountRootType::Income, None, true),
            ("4100", "Sales Revenue", AccountRootType::Income, Some("4000"), false),
            ("4200", "Service & Consulting Revenue", AccountRootType::Income, Some("4000"), false),
            ("4900", "Realized Foreign Exchange Gain", AccountRootType::Income, Some("4000"), false),

            // Expenses
            ("5000", "Expenses", AccountRootType::Expense, None, true),
            ("5100", "Cost of Goods Sold (COGS)", AccountRootType::Expense, Some("5000"), false),
            ("5200", "Office Supplies & Hardware", AccountRootType::Expense, Some("5000"), false),
            ("5300", "Software & Cloud Services", AccountRootType::Expense, Some("5000"), false),
            ("5400", "Professional & Legal Fees", AccountRootType::Expense, Some("5000"), false),
            ("5900", "Realized Foreign Exchange Loss", AccountRootType::Expense, Some("5000"), false),
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
        let entries = Self::post_journal_entry_conn(
            &ctx.db.conn(),
            posting_date,
            lines,
            voucher_type,
            voucher_id,
            remarks,
        )?;

        let entry_number = entries.first().map(|e| e.entry_number.clone()).unwrap_or_default();
        let _ = ctx.record_and_publish_event(
            "GlEntriesPosted",
            "GeneralLedger",
            &entry_number,
            serde_json::to_value(&entries).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(entries)
    }

    pub fn post_journal_entry_conn(
        conn: &Connection,
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

        let now = Utc::now();
        let entry_number = format!("GL-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
        let mut created_entries = Vec::new();

        conn.execute("BEGIN TRANSACTION;", [])?;

        for line in lines {
            let acc = match Self::get_account_by_id(conn, &line.account_id) {
                Ok(a) => a,
                Err(e) => {
                    let _ = conn.execute("ROLLBACK;", []);
                    return Err(e);
                }
            };
            if acc.is_group {
                let _ = conn.execute("ROLLBACK;", []);
                return Err(AccountingError::CannotPostToGroupAccount(acc.name));
            }

            let entry_id = Uuid::new_v4().to_string();

            if let Err(e) = conn.execute(
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
            ) {
                let _ = conn.execute("ROLLBACK;", []);
                return Err(AccountingError::Database(e));
            }

            // Update materialized account balance
            let net_change = if acc.root_type.is_debit_normal() {
                line.debit - line.credit
            } else {
                line.credit - line.debit
            };

            let new_balance = acc.balance + net_change;
            if let Err(e) = conn.execute(
                "UPDATE accounts SET balance = ?1 WHERE id = ?2",
                params![new_balance.to_string(), line.account_id],
            ) {
                let _ = conn.execute("ROLLBACK;", []);
                return Err(AccountingError::Database(e));
            }

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

        conn.execute("COMMIT;", [])?;

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

    // --- Multi-Currency & FX Rate Methods (LEDGER-18) ---

    pub fn set_exchange_rate(
        conn: &Connection,
        from_currency: &str,
        to_currency: &str,
        rate: Decimal,
        effective_date: &str,
        source: &str,
    ) -> Result<crate::models::ExchangeRate, AccountingError> {
        let id = format!("fx-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
        let now = Utc::now();
        conn.execute(
            r#"
            INSERT INTO exchange_rates (id, from_currency, to_currency, rate, effective_date, source, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![id, from_currency, to_currency, rate.to_string(), effective_date, source, now.to_rfc3339()],
        )?;

        Ok(crate::models::ExchangeRate {
            id,
            from_currency: from_currency.to_string(),
            to_currency: to_currency.to_string(),
            rate,
            effective_date: effective_date.to_string(),
            source: source.to_string(),
            created_at: now,
        })
    }

    pub fn get_exchange_rate(conn: &Connection, from_currency: &str, to_currency: &str) -> Result<Decimal, AccountingError> {
        if from_currency.eq_ignore_ascii_case(to_currency) {
            return Ok(Decimal::ONE);
        }
        let rate_str: Option<String> = conn
            .query_row(
                "SELECT rate FROM exchange_rates WHERE from_currency = ?1 AND to_currency = ?2 ORDER BY effective_date DESC LIMIT 1",
                params![from_currency, to_currency],
                |r| r.get(0),
            )
            .ok();

        if let Some(s) = rate_str {
            if let Ok(d) = Decimal::from_str(&s) {
                return Ok(d);
            }
        }
        // Fallback default conversion rates for common pairs
        match (from_currency.to_uppercase().as_str(), to_currency.to_uppercase().as_str()) {
            ("EUR", "USD") => Ok(Decimal::from_str("1.08").unwrap()),
            ("USD", "EUR") => Ok(Decimal::from_str("0.9259").unwrap()),
            ("GBP", "EUR") => Ok(Decimal::from_str("1.18").unwrap()),
            ("EUR", "GBP") => Ok(Decimal::from_str("0.8475").unwrap()),
            _ => Ok(Decimal::ONE),
        }
    }

    pub fn list_exchange_rates(conn: &Connection) -> Result<Vec<crate::models::ExchangeRate>, AccountingError> {
        let mut stmt = conn.prepare(
            "SELECT id, from_currency, to_currency, rate, effective_date, source, created_at FROM exchange_rates ORDER BY effective_date DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            let from_curr: String = r.get(1)?;
            let to_curr: String = r.get(2)?;
            let rate_str: String = r.get(3)?;
            let date_str: String = r.get(4)?;
            let source: String = r.get(5)?;
            let created_at_str: String = r.get(6)?;

            let rate = Decimal::from_str(&rate_str).unwrap_or(Decimal::ONE);
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(crate::models::ExchangeRate {
                id,
                from_currency: from_curr,
                to_currency: to_curr,
                rate,
                effective_date: date_str,
                source,
                created_at,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // --- Client Retainer Accounts (LEDGER-19) ---

    pub fn get_or_create_retainer_account(conn: &Connection, customer_id: &str, currency: &str) -> Result<crate::models::RetainerAccount, AccountingError> {
        let row: Option<(String, String, String, String, String)> = conn
            .query_row(
                "SELECT id, customer_id, balance, currency, liability_account_id FROM retainer_accounts WHERE customer_id = ?1",
                params![customer_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .ok();

        if let Some((id, cid, bal_str, curr, liab_id)) = row {
            return Ok(crate::models::RetainerAccount {
                id,
                customer_id: cid,
                balance: Decimal::from_str(&bal_str).unwrap_or(Decimal::ZERO),
                currency: curr,
                liability_account_id: liab_id,
                updated_at: Utc::now(),
            });
        }

        let id = format!("ret-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
        let liab_acc = Self::get_account_by_code(conn, "2150")?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO retainer_accounts (id, customer_id, balance, currency, liability_account_id, updated_at)
            VALUES (?1, ?2, '0', ?3, ?4, ?5)
            "#,
            params![id, customer_id, currency, liab_acc.id, now],
        )?;

        Ok(crate::models::RetainerAccount {
            id,
            customer_id: customer_id.to_string(),
            balance: Decimal::ZERO,
            currency: currency.to_string(),
            liability_account_id: liab_acc.id,
            updated_at: Utc::now(),
        })
    }

    pub fn deposit_retainer(
        conn: &Connection,
        customer_id: &str,
        amount: Decimal,
        currency: &str,
    ) -> Result<crate::models::RetainerAccount, AccountingError> {
        let mut ret = Self::get_or_create_retainer_account(conn, customer_id, currency)?;
        ret.balance += amount;

        conn.execute(
            "UPDATE retainer_accounts SET balance = ?1, updated_at = ?2 WHERE id = ?3",
            params![ret.balance.to_string(), Utc::now().to_rfc3339(), ret.id],
        )?;

        // Book GL: DR 1120 Bank, CR 2150 Retainer Liability
        let bank_acc = Self::get_account_by_code(conn, "1120")?;
        let liab_acc = Self::get_account_by_code(conn, "2150")?;

        let lines = vec![
            JournalEntryLine {
                account_id: bank_acc.id,
                debit: amount,
                credit: Decimal::ZERO,
                party_type: Some("Customer".to_string()),
                party_id: Some(customer_id.to_string()),
                remarks: Some("Customer retainer deposit received".to_string()),
            },
            JournalEntryLine {
                account_id: liab_acc.id,
                debit: Decimal::ZERO,
                credit: amount,
                party_type: Some("Customer".to_string()),
                party_id: Some(customer_id.to_string()),
                remarks: Some("Retainer deferred revenue liability".to_string()),
            },
        ];

        Self::post_journal_entry_conn(
            conn,
            &Utc::now().format("%Y-%m-%d").to_string(),
            lines,
            "RetainerDeposit",
            &ret.id,
            Some(&format!("Retainer deposit for customer {}", customer_id)),
        )?;

        Ok(ret)
    }

    pub fn drawdown_retainer(
        conn: &Connection,
        customer_id: &str,
        amount: Decimal,
        invoice_id: Option<&str>,
    ) -> Result<crate::models::RetainerAccount, AccountingError> {
        let mut ret = Self::get_or_create_retainer_account(conn, customer_id, "EUR")?;
        if ret.balance < amount {
            return Err(AccountingError::InvalidTransaction(format!(
                "Insufficient retainer balance (Available: {}, Requested drawdown: {})",
                ret.balance, amount
            )));
        }

        ret.balance -= amount;
        conn.execute(
            "UPDATE retainer_accounts SET balance = ?1, updated_at = ?2 WHERE id = ?3",
            params![ret.balance.to_string(), Utc::now().to_rfc3339(), ret.id],
        )?;

        // Book GL: DR 2150 Retainer Liability, CR 4100 Revenue
        let liab_acc = Self::get_account_by_code(conn, "2150")?;
        let rev_acc = Self::get_account_by_code(conn, "4100")?;

        let ref_id = invoice_id.unwrap_or(&ret.id);
        let lines = vec![
            JournalEntryLine {
                account_id: liab_acc.id,
                debit: amount,
                credit: Decimal::ZERO,
                party_type: Some("Customer".to_string()),
                party_id: Some(customer_id.to_string()),
                remarks: Some(format!("Drawdown against retainer for {}", ref_id)),
            },
            JournalEntryLine {
                account_id: rev_acc.id,
                debit: Decimal::ZERO,
                credit: amount,
                party_type: Some("Customer".to_string()),
                party_id: Some(customer_id.to_string()),
                remarks: Some(format!("Retainer revenue recognized for {}", ref_id)),
            },
        ];

        Self::post_journal_entry_conn(
            conn,
            &Utc::now().format("%Y-%m-%d").to_string(),
            lines,
            "RetainerDrawdown",
            ref_id,
            Some(&format!("Drawdown retainer for customer {}", customer_id)),
        )?;

        Ok(ret)
    }

    // --- POS Shifts & Cashier Mode (LEDGER-24) ---

    pub fn open_pos_shift(conn: &Connection, cashier_id: &str, opening_float: Decimal) -> Result<crate::models::PosShift, AccountingError> {
        let id = format!("shift-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO pos_shifts (
                id, cashier_id, opened_at, opening_float, cash_sales, card_sales, qr_sales, status
            ) VALUES (?1, ?2, ?3, ?4, '0', '0', '0', 'Open')
            "#,
            params![id, cashier_id, now, opening_float.to_string()],
        )?;

        Ok(crate::models::PosShift {
            id,
            cashier_id: cashier_id.to_string(),
            opened_at: now,
            closed_at: None,
            opening_float,
            cash_sales: Decimal::ZERO,
            card_sales: Decimal::ZERO,
            qr_sales: Decimal::ZERO,
            closing_cash_actual: None,
            discrepancy: None,
            status: "Open".to_string(),
        })
    }

    pub fn record_pos_sale(
        conn: &Connection,
        shift_id: &str,
        amount: Decimal,
        tender_type: &str,
    ) -> Result<(), AccountingError> {
        let col = match tender_type.to_lowercase().as_str() {
            "cash" => "cash_sales",
            "card" => "card_sales",
            _ => "qr_sales",
        };

        let current_val_str: String = conn.query_row(
            &format!("SELECT {} FROM pos_shifts WHERE id = ?1", col),
            params![shift_id],
            |r| r.get(0),
        )?;

        let current_val = Decimal::from_str(&current_val_str).unwrap_or(Decimal::ZERO);
        let new_val = current_val + amount;

        conn.execute(
            &format!("UPDATE pos_shifts SET {} = ?1 WHERE id = ?2", col),
            params![new_val.to_string(), shift_id],
        )?;

        Ok(())
    }

    pub fn close_pos_shift(
        conn: &Connection,
        shift_id: &str,
        closing_cash_actual: Decimal,
    ) -> Result<crate::models::PosShift, AccountingError> {
        let row: (String, String, String, String, String, String) = conn.query_row(
            "SELECT cashier_id, opened_at, opening_float, cash_sales, card_sales, qr_sales FROM pos_shifts WHERE id = ?1",
            params![shift_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )?;

        let cashier_id = row.0;
        let opened_at = row.1;
        let opening_float = Decimal::from_str(&row.2).unwrap_or(Decimal::ZERO);
        let cash_sales = Decimal::from_str(&row.3).unwrap_or(Decimal::ZERO);
        let card_sales = Decimal::from_str(&row.4).unwrap_or(Decimal::ZERO);
        let qr_sales = Decimal::from_str(&row.5).unwrap_or(Decimal::ZERO);

        let expected_cash = opening_float + cash_sales;
        let discrepancy = closing_cash_actual - expected_cash;
        let closed_at = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            UPDATE pos_shifts
            SET closed_at = ?1, closing_cash_actual = ?2, discrepancy = ?3, status = 'Closed'
            WHERE id = ?4
            "#,
            params![closed_at, closing_cash_actual.to_string(), discrepancy.to_string(), shift_id],
        )?;

        // Automatically post daily cash summary to GL: DR 1110 Cash on Hand, CR 4100 Sales Revenue
        let total_sales = cash_sales + card_sales + qr_sales;
        if total_sales > Decimal::ZERO {
            let cash_acc = Self::get_account_by_code(conn, "1110")?;
            let rev_acc = Self::get_account_by_code(conn, "4100")?;

            let lines = vec![
                JournalEntryLine {
                    account_id: cash_acc.id,
                    debit: total_sales,
                    credit: Decimal::ZERO,
                    party_type: Some("POS".to_string()),
                    party_id: Some(cashier_id.clone()),
                    remarks: Some(format!("Daily POS sales summary for shift {}", shift_id)),
                },
                JournalEntryLine {
                    account_id: rev_acc.id,
                    debit: Decimal::ZERO,
                    credit: total_sales,
                    party_type: Some("POS".to_string()),
                    party_id: Some(cashier_id.clone()),
                    remarks: Some(format!("Daily POS sales summary revenue for shift {}", shift_id)),
                },
            ];

            let _ = Self::post_journal_entry_conn(
                conn,
                &Utc::now().format("%Y-%m-%d").to_string(),
                lines,
                "PosDailySummary",
                shift_id,
                Some(&format!("POS Shift {} Z-Report Reconciliation", shift_id)),
            );
        }

        Ok(crate::models::PosShift {
            id: shift_id.to_string(),
            cashier_id,
            opened_at,
            closed_at: Some(closed_at),
            opening_float,
            cash_sales,
            card_sales,
            qr_sales,
            closing_cash_actual: Some(closing_cash_actual),
            discrepancy: Some(discrepancy),
            status: "Closed".to_string(),
        })
    }

    pub fn generate_shift_report(conn: &Connection, shift_id: &str, is_z_report: bool) -> Result<String, AccountingError> {
        let shift: (String, String, Option<String>, String, String, String, String, Option<String>, Option<String>, String) = conn.query_row(
            "SELECT id, cashier_id, closed_at, opening_float, cash_sales, card_sales, qr_sales, closing_cash_actual, discrepancy, status FROM pos_shifts WHERE id = ?1",
            params![shift_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
        )?;

        let report_type = if is_z_report { "FINAL SHIFT Z-REPORT" } else { "MID-SHIFT X-REPORT" };
        let open_float = Decimal::from_str(&shift.3).unwrap_or(Decimal::ZERO);
        let cash_sales = Decimal::from_str(&shift.4).unwrap_or(Decimal::ZERO);
        let card_sales = Decimal::from_str(&shift.5).unwrap_or(Decimal::ZERO);
        let qr_sales = Decimal::from_str(&shift.6).unwrap_or(Decimal::ZERO);
        let total_sales = cash_sales + card_sales + qr_sales;

        let mut out = String::new();
        out.push_str("========================================\n");
        out.push_str(&format!("          LEDGER ERP POS\n"));
        out.push_str(&format!("       {}\n", report_type));
        out.push_str("========================================\n");
        out.push_str(&format!("Shift ID    : {}\n", shift.0));
        out.push_str(&format!("Cashier     : {}\n", shift.1));
        out.push_str(&format!("Status      : {}\n", shift.9));
        out.push_str("----------------------------------------\n");
        out.push_str(&format!("Opening Float   : {:>10.2}\n", open_float));
        out.push_str(&format!("Cash Sales      : {:>10.2}\n", cash_sales));
        out.push_str(&format!("Card Sales      : {:>10.2}\n", card_sales));
        out.push_str(&format!("EPC QR Sales    : {:>10.2}\n", qr_sales));
        out.push_str("----------------------------------------\n");
        out.push_str(&format!("TOTAL REVENUE   : {:>10.2}\n", total_sales));
        out.push_str(&format!("Expected In Cash: {:>10.2}\n", open_float + cash_sales));
        if let Some(actual) = shift.7 {
            out.push_str(&format!("Actual Cash     : {:>10.2}\n", Decimal::from_str(&actual).unwrap_or(Decimal::ZERO)));
        }
        if let Some(disc) = shift.8 {
            out.push_str(&format!("Discrepancy     : {:>10.2}\n", Decimal::from_str(&disc).unwrap_or(Decimal::ZERO)));
        }
        out.push_str("========================================\n");

        Ok(out)
    }

    pub fn get_pos_shift(conn: &Connection, shift_id: &str) -> Result<crate::models::PosShift, AccountingError> {
        let (id, cashier_id, opened_at, closed_at, opening_float_str, cash_sales_str, card_sales_str, qr_sales_str, closing_cash_str, discrepancy_str, status): (
            String, String, String, Option<String>, String, String, String, String, Option<String>, Option<String>, String
        ) = conn.query_row(
            "SELECT id, cashier_id, opened_at, closed_at, opening_float, cash_sales, card_sales, qr_sales, closing_cash_actual, discrepancy, status FROM pos_shifts WHERE id = ?1",
            params![shift_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
        )?;

        Ok(crate::models::PosShift {
            id,
            cashier_id,
            opened_at,
            closed_at,
            opening_float: Decimal::from_str(&opening_float_str).unwrap_or(Decimal::ZERO),
            cash_sales: Decimal::from_str(&cash_sales_str).unwrap_or(Decimal::ZERO),
            card_sales: Decimal::from_str(&card_sales_str).unwrap_or(Decimal::ZERO),
            qr_sales: Decimal::from_str(&qr_sales_str).unwrap_or(Decimal::ZERO),
            closing_cash_actual: closing_cash_str.and_then(|s| Decimal::from_str(&s).ok()),
            discrepancy: discrepancy_str.and_then(|s| Decimal::from_str(&s).ok()),
            status,
        })
    }
}
