use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountingError {
    #[error("Unbalanced journal entry: Total Debits ({debits}) does not match Total Credits ({credits}) (difference: {diff})")]
    UnbalancedJournalEntry {
        debits: Decimal,
        credits: Decimal,
        diff: Decimal,
    },

    #[error("Account not found: '{0}'")]
    AccountNotFound(String),

    #[error("Cannot post to a group/parent account: '{0}'. Postings must be made to leaf accounts.")]
    CannotPostToGroupAccount(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Kernel error: {0}")]
    Kernel(#[from] erp_kernel::ErpKernelError),
}
