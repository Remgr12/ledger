use thiserror::Error;

#[derive(Error, Debug)]
pub enum InventoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Kernel error: {0}")]
    Kernel(#[from] erp_kernel::ErpKernelError),

    #[error("Decimal parse error: {0}")]
    DecimalParse(#[from] rust_decimal::Error),

    #[error("Location not found: {0}")]
    LocationNotFound(String),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Insufficient stock for item '{item_id}' at location '{location_id}': available {available}, requested {requested}")]
    InsufficientStock {
        item_id: String,
        location_id: String,
        available: rust_decimal::Decimal,
        requested: rust_decimal::Decimal,
    },

    #[error("Invalid QR code URI: '{0}'")]
    InvalidQrUri(String),

    #[error("QR generation error: {0}")]
    QrGeneration(String),

    #[error("Invalid movement: {0}")]
    InvalidMovement(String),
}
