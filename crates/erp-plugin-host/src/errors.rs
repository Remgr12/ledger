use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginHostError {
    #[error("Extism Wasm error: {0}")]
    Extism(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Permission denied: plugin '{plugin_id}' lacks permission '{required_permission}'")]
    PermissionDenied {
        plugin_id: String,
        required_permission: String,
    },

    #[error("Plugin not found: '{0}'")]
    PluginNotFound(String),

    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    #[error("Plugin execution error in '{plugin_id}' -> '{function}': {error}")]
    ExecutionFailed {
        plugin_id: String,
        function: String,
        error: String,
    },
}
