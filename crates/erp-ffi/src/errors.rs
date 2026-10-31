use thiserror::Error;

#[derive(Error, Debug, uniffi::Error)]
pub enum ErpFfiError {
    #[error("Database error: {details}")]
    DatabaseError { details: String },

    #[error("Module error: {details}")]
    ModuleError { details: String },

    #[error("Inventory error: {details}")]
    InventoryError { details: String },

    #[error("Plugin error: {details}")]
    PluginError { details: String },

    #[error("Invalid argument: {details}")]
    InvalidArgument { details: String },

    #[error("Not found: {details}")]
    NotFound { details: String },

    #[error("Internal error: {details}")]
    InternalError { details: String },
}

impl From<erp_kernel::ErpKernelError> for ErpFfiError {
    fn from(e: erp_kernel::ErpKernelError) -> Self {
        match e {
            erp_kernel::ErpKernelError::Database(d) => ErpFfiError::DatabaseError {
                details: d.to_string(),
            },
            erp_kernel::ErpKernelError::ModuleNotFound(m) => ErpFfiError::NotFound { details: m },
            other => ErpFfiError::ModuleError {
                details: other.to_string(),
            },
        }
    }
}

impl From<mod_inventory::InventoryError> for ErpFfiError {
    fn from(e: mod_inventory::InventoryError) -> Self {
        match e {
            mod_inventory::InventoryError::LocationNotFound(l) => ErpFfiError::NotFound {
                details: format!("Location not found: {}", l),
            },
            mod_inventory::InventoryError::ItemNotFound(i) => ErpFfiError::NotFound {
                details: format!("Item not found: {}", i),
            },
            other => ErpFfiError::InventoryError {
                details: other.to_string(),
            },
        }
    }
}

impl From<erp_plugin_host::PluginHostError> for ErpFfiError {
    fn from(e: erp_plugin_host::PluginHostError) -> Self {
        ErpFfiError::PluginError {
            details: e.to_string(),
        }
    }
}

impl From<mod_accounting::AccountingError> for ErpFfiError {
    fn from(e: mod_accounting::AccountingError) -> Self {
        match e {
            mod_accounting::AccountingError::AccountNotFound(a) => ErpFfiError::NotFound {
                details: format!("Account not found: {}", a),
            },
            other => ErpFfiError::ModuleError {
                details: other.to_string(),
            },
        }
    }
}

impl From<mod_purchasing::PurchasingError> for ErpFfiError {
    fn from(e: mod_purchasing::PurchasingError) -> Self {
        match e {
            mod_purchasing::PurchasingError::PurchaseOrderNotFound(p) => ErpFfiError::NotFound {
                details: format!("Purchase order not found: {}", p),
            },
            other => ErpFfiError::ModuleError {
                details: other.to_string(),
            },
        }
    }
}

impl From<mod_manufacturing::ManufacturingError> for ErpFfiError {
    fn from(e: mod_manufacturing::ManufacturingError) -> Self {
        match e {
            mod_manufacturing::ManufacturingError::BomNotFound(b) => ErpFfiError::NotFound {
                details: format!("BOM not found: {}", b),
            },
            mod_manufacturing::ManufacturingError::WorkOrderNotFound(w) => ErpFfiError::NotFound {
                details: format!("Work order not found: {}", w),
            },
            other => ErpFfiError::ModuleError {
                details: other.to_string(),
            },
        }
    }
}

impl From<mod_projects::ProjectsError> for ErpFfiError {
    fn from(e: mod_projects::ProjectsError) -> Self {
        match e {
            mod_projects::ProjectsError::ProjectNotFound(p) => ErpFfiError::NotFound {
                details: format!("Project not found: {}", p),
            },
            other => ErpFfiError::ModuleError {
                details: other.to_string(),
            },
        }
    }
}

impl From<rusqlite::Error> for ErpFfiError {
    fn from(e: rusqlite::Error) -> Self {
        ErpFfiError::DatabaseError {
            details: e.to_string(),
        }
    }
}
