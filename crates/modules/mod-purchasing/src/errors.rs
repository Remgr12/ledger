use thiserror::Error;

#[derive(Error, Debug)]
pub enum PurchasingError {
    #[error("Purchase order not found: '{0}'")]
    PurchaseOrderNotFound(String),

    #[error("Invalid purchase order status: {0}")]
    InvalidStatus(String),

    #[error("Invalid line item: {0}")]
    InvalidLine(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Inventory error: {0}")]
    Inventory(#[from] mod_inventory::InventoryError),

    #[error("Kernel error: {0}")]
    Kernel(#[from] erp_kernel::ErpKernelError),
}
