use thiserror::Error;

#[derive(Error, Debug)]
pub enum ManufacturingError {
    #[error("BOM not found: '{0}'")]
    BomNotFound(String),

    #[error("Work order not found: '{0}'")]
    WorkOrderNotFound(String),

    #[error("Invalid status transition: {0}")]
    InvalidStatus(String),

    #[error("Invalid component or configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Inventory error: {0}")]
    Inventory(#[from] mod_inventory::InventoryError),

    #[error("Kernel error: {0}")]
    Kernel(#[from] erp_kernel::ErpKernelError),
}
