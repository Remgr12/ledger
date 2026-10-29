use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErpKernelError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Module error: {0}")]
    Module(String),

    #[error("Module dependency error: module '{module}' requires '{dependency}' which is not enabled")]
    MissingDependency {
        module: String,
        dependency: String,
    },

    #[error("Module dependency error: cannot disable module '{module}' because enabled module '{dependent}' depends on it")]
    DependentModuleActive {
        module: String,
        dependent: String,
    },

    #[error("Module not found: '{0}'")]
    ModuleNotFound(String),

    #[error("Migration error in module '{module}' v{version} ({name}): {error}")]
    MigrationFailed {
        module: String,
        version: u32,
        name: String,
        error: String,
    },

    #[error("HLC parse error: {0}")]
    HlcParse(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
