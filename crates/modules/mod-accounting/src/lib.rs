pub mod errors;
pub mod models;
pub mod module;
pub mod service;

pub use errors::AccountingError;
pub use models::{
    Account, AccountRootType, ExchangeRate, FinancialStatement, GlEntry, JournalEntryLine,
    PaymentEntry, PaymentType, PosShift, RetainerAccount, TrialBalanceRow,
};
pub use module::AccountingModule;
pub use service::AccountingService;

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;

    fn setup_test_ctx() -> (Database, erp_kernel::ModuleContext, Arc<ModuleManager>) {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("node-acc-test".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc, bus));
        mm.register(Arc::new(AccountingModule::new()));
        mm.initialize().expect("init mm");
        mm.enable_module("mod-accounting").expect("enable accounting");

        let ctx = mm.context();
        (db, ctx, mm)
    }

    #[test]
    fn test_double_entry_balance_invariant_enforcement() {
        let (db, ctx, _) = setup_test_ctx();
        AccountingService::seed_default_chart_of_accounts(&db.conn()).expect("seed coa");

        let bank = AccountingService::get_account_by_code(&db.conn(), "1120").unwrap();
        let revenue = AccountingService::get_account_by_code(&db.conn(), "4100").unwrap();

        // 1. Unbalanced entry (Debit 100, Credit 90) -> MUST FAIL
        let unbalanced_lines = vec![
            JournalEntryLine {
                account_id: bank.id.clone(),
                debit: Decimal::from(100),
                credit: Decimal::ZERO,
                party_type: None,
                party_id: None,
                remarks: None,
            },
            JournalEntryLine {
                account_id: revenue.id.clone(),
                debit: Decimal::ZERO,
                credit: Decimal::from(90),
                party_type: None,
                party_id: None,
                remarks: None,
            },
        ];

        let result = AccountingService::post_journal_entry(
            &ctx,
            "2026-10-29",
            unbalanced_lines,
            "Journal Entry",
            "JV-001",
            Some("Testing unbalanced entry"),
        );

        match result {
            Err(AccountingError::UnbalancedJournalEntry { debits, credits, diff }) => {
                assert_eq!(debits, Decimal::from(100));
                assert_eq!(credits, Decimal::from(90));
                assert_eq!(diff, Decimal::from(10));
            }
            _ => panic!("Expected UnbalancedJournalEntry error, got {:?}", result),
        }

        // 2. Balanced entry (Debit 100, Credit 100) -> MUST SUCCEED
        let balanced_lines = vec![
            JournalEntryLine {
                account_id: bank.id.clone(),
                debit: Decimal::from(100),
                credit: Decimal::ZERO,
                party_type: None,
                party_id: None,
                remarks: None,
            },
            JournalEntryLine {
                account_id: revenue.id.clone(),
                debit: Decimal::ZERO,
                credit: Decimal::from(100),
                party_type: None,
                party_id: None,
                remarks: None,
            },
        ];

        let gl_entries = AccountingService::post_journal_entry(
            &ctx,
            "2026-10-29",
            balanced_lines,
            "Journal Entry",
            "JV-002",
            Some("Balanced sale deposit"),
        )
        .expect("post balanced entry");

        assert_eq!(gl_entries.len(), 2);

        // Verify account balances
        let bank_after = AccountingService::get_account_by_code(&db.conn(), "1120").unwrap();
        let rev_after = AccountingService::get_account_by_code(&db.conn(), "4100").unwrap();
        assert_eq!(bank_after.balance, Decimal::from(100));
        assert_eq!(rev_after.balance, Decimal::from(100));
    }

    #[test]
    fn test_trial_balance_and_profit_loss() {
        let (db, ctx, _) = setup_test_ctx();
        AccountingService::seed_default_chart_of_accounts(&db.conn()).expect("seed coa");

        let bank = AccountingService::get_account_by_code(&db.conn(), "1120").unwrap();
        let revenue = AccountingService::get_account_by_code(&db.conn(), "4100").unwrap();
        let supplies = AccountingService::get_account_by_code(&db.conn(), "5200").unwrap();

        // Transaction 1: Revenue of 250 deposited to Bank
        AccountingService::post_journal_entry(
            &ctx,
            "2026-10-01",
            vec![
                JournalEntryLine {
                    account_id: bank.id.clone(),
                    debit: Decimal::from(250),
                    credit: Decimal::ZERO,
                    party_type: None,
                    party_id: None,
                    remarks: None,
                },
                JournalEntryLine {
                    account_id: revenue.id.clone(),
                    debit: Decimal::ZERO,
                    credit: Decimal::from(250),
                    party_type: None,
                    party_id: None,
                    remarks: None,
                },
            ],
            "Journal Entry",
            "JV-1",
            None,
        )
        .unwrap();

        // Transaction 2: Office Supplies expense of 50 paid from Bank
        AccountingService::post_journal_entry(
            &ctx,
            "2026-10-05",
            vec![
                JournalEntryLine {
                    account_id: supplies.id.clone(),
                    debit: Decimal::from(50),
                    credit: Decimal::ZERO,
                    party_type: None,
                    party_id: None,
                    remarks: None,
                },
                JournalEntryLine {
                    account_id: bank.id.clone(),
                    debit: Decimal::ZERO,
                    credit: Decimal::from(50),
                    party_type: None,
                    party_id: None,
                    remarks: None,
                },
            ],
            "Journal Entry",
            "JV-2",
            None,
        )
        .unwrap();

        // Check Trial Balance
        let tb = AccountingService::get_trial_balance(&db.conn()).expect("get tb");
        assert_eq!(tb.total_debit, tb.total_credit);
        assert_eq!(tb.total_debit, Decimal::from(300)); // 250 + 50

        // Check Profit and Loss: Revenue 250 - Expenses 50 = Net Profit 200
        let pnl = AccountingService::get_profit_and_loss(&db.conn(), "2026-10-01", "2026-10-31")
            .expect("get pnl");
        assert_eq!(pnl.total_credit, Decimal::from(250)); // Income
        assert_eq!(pnl.total_debit, Decimal::from(50));   // Expenses
        assert_eq!(pnl.net_income_or_equity, Decimal::from(200)); // Net profit
    }

    #[test]
    fn test_multi_currency_and_retainers_and_pos() {
        let (db, _, _) = setup_test_ctx();
        AccountingService::seed_default_chart_of_accounts(&db.conn()).expect("seed coa");

        // 1. FX Rates
        AccountingService::set_exchange_rate(&db.conn(), "USD", "EUR", Decimal::from_str("0.92").unwrap(), "2026-10-31", "Manual").unwrap();
        let rate = AccountingService::get_exchange_rate(&db.conn(), "USD", "EUR").unwrap();
        assert_eq!(rate, Decimal::from_str("0.92").unwrap());
        let rates = AccountingService::list_exchange_rates(&db.conn()).unwrap();
        assert_eq!(rates.len(), 1);

        // 2. Retainers
        let deposit = AccountingService::deposit_retainer(&db.conn(), "cust-1", Decimal::from(1000), "EUR").unwrap();
        assert_eq!(deposit.balance, Decimal::from(1000));
        let drawdown = AccountingService::drawdown_retainer(&db.conn(), "cust-1", Decimal::from(350), Some("inv-101")).unwrap();
        assert_eq!(drawdown.balance, Decimal::from(650));

        // 3. POS Cashier Mode
        let shift = AccountingService::open_pos_shift(&db.conn(), "cashier-jane", Decimal::from(200)).unwrap();
        assert_eq!(shift.status, "Open");
        AccountingService::record_pos_sale(&db.conn(), &shift.id, Decimal::from(50), "Cash").unwrap();
        AccountingService::record_pos_sale(&db.conn(), &shift.id, Decimal::from(80), "Card").unwrap();
        let closed_shift = AccountingService::close_pos_shift(&db.conn(), &shift.id, Decimal::from(250)).unwrap();
        assert_eq!(closed_shift.status, "Closed");
        assert_eq!(closed_shift.discrepancy, Some(Decimal::ZERO)); // 200 float + 50 cash = 250 actual

        let report = AccountingService::generate_shift_report(&db.conn(), &shift.id, true).unwrap();
        assert!(report.contains("FINAL SHIFT Z-REPORT"));
        assert!(report.contains("TOTAL REVENUE"));
    }
}
