use thiserror::Error;

#[derive(Error, Debug, uniffi::Error)]
pub enum ErpFfiError {
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Module error: {message}")]
    ModuleError { message: String },

    #[error("Inventory error: {message}")]
    InventoryError { message: String },

    #[error("Plugin error: {message}")]
    PluginError { message: String },

    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl From<erp_kernel::ErpKernelError> for ErpFfiError {
    fn from(e: erp_kernel::ErpKernelError) -> Self {
        match e {
            erp_kernel::ErpKernelError::Database(d) => ErpFfiError::DatabaseError {
                message: d.to_string(),
            },
            erp_kernel::ErpKernelError::ModuleNotFound(m) => ErpFfiError::NotFound { message: m },
            other => ErpFfiError::ModuleError {
                message: other.to_string(),
            },
        }
    }
}

impl From<mod_inventory::InventoryError> for ErpFfiError {
    fn from(e: mod_inventory::InventoryError) -> Self {
        match e {
            mod_inventory::InventoryError::LocationNotFound(l) => ErpFfiError::NotFound {
                message: format!("Location not found: {}", l),
            },
            mod_inventory::InventoryError::ItemNotFound(i) => ErpFfiError::NotFound {
                message: format!("Item not found: {}", i),
            },
            other => ErpFfiError::InventoryError {
                message: other.to_string(),
            },
        }
    }
}

impl From<erp_plugin_host::PluginHostError> for ErpFfiError {
    fn from(e: erp_plugin_host::PluginHostError) -> Self {
        ErpFfiError::PluginError {
            message: e.to_string(),
        }
    }
}

impl From<mod_accounting::AccountingError> for ErpFfiError {
    fn from(e: mod_accounting::AccountingError) -> Self {
        match e {
            mod_accounting::AccountingError::AccountNotFound(a) => ErpFfiError::NotFound {
                message: format!("Account not found: {}", a),
            },
            other => ErpFfiError::ModuleError {
                message: other.to_string(),
            },
        }
    }
}

impl From<mod_purchasing::PurchasingError> for ErpFfiError {
    fn from(e: mod_purchasing::PurchasingError) -> Self {
        match e {
            mod_purchasing::PurchasingError::PurchaseOrderNotFound(p) => ErpFfiError::NotFound {
                message: format!("Purchase order not found: {}", p),
            },
            other => ErpFfiError::ModuleError {
                message: other.to_string(),
            },
        }
    }
}

impl From<mod_manufacturing::ManufacturingError> for ErpFfiError {
    fn from(e: mod_manufacturing::ManufacturingError) -> Self {
        match e {
            mod_manufacturing::ManufacturingError::BomNotFound(b) => ErpFfiError::NotFound {
                message: format!("BOM not found: {}", b),
            },
            mod_manufacturing::ManufacturingError::WorkOrderNotFound(w) => ErpFfiError::NotFound {
                message: format!("Work order not found: {}", w),
            },
            other => ErpFfiError::ModuleError {
                message: other.to_string(),
            },
        }
    }
}

impl From<mod_projects::ProjectsError> for ErpFfiError {
    fn from(e: mod_projects::ProjectsError) -> Self {
        match e {
            mod_projects::ProjectsError::ProjectNotFound(p) => ErpFfiError::NotFound {
                message: format!("Project not found: {}", p),
            },
            other => ErpFfiError::ModuleError {
                message: other.to_string(),
            },
        }
    }
}
