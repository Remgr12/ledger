use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectsError {
    #[error("Project not found: '{0}'")]
    ProjectNotFound(String),

    #[error("Task not found: '{0}'")]
    TaskNotFound(String),

    #[error("No unbilled timesheets found to invoice for project: '{0}'")]
    NoUnbilledTimesheets(String),

    #[error("Cannot invoice project without linked customer: '{0}'")]
    ProjectHasNoCustomer(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Kernel error: {0}")]
    Kernel(#[from] erp_kernel::ErpKernelError),
}
