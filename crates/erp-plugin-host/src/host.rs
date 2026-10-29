use std::collections::HashMap;
use extism::{Manifest as ExtismManifest, Plugin, Wasm};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::errors::PluginHostError;
use crate::manifest::PluginManifest;
use crate::ui_node::PluginUiNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginQrInterceptResult {
    pub handled: bool,
    pub translated_sku: Option<String>,
    pub metadata: HashMap<String, String>,
    pub display_note: Option<String>,
}

struct LoadedPlugin {
    manifest: PluginManifest,
    wasm_bytes: Vec<u8>,
}

pub struct PluginHost {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Load and validate a new Wasm plugin with its declared permissions
    pub fn install_plugin(
        &self,
        manifest: PluginManifest,
        wasm_bytes: Vec<u8>,
    ) -> Result<(), PluginHostError> {
        // Validate that Wasm compiles and instantiates cleanly
        let extism_manifest = ExtismManifest::new([Wasm::data(wasm_bytes.clone())]);
        let _ = Plugin::new(&extism_manifest, [], true)
            .map_err(|e| PluginHostError::Extism(e.to_string()))?;

        let mut lock = self.plugins.write();
        lock.insert(
            manifest.id.clone(),
            LoadedPlugin {
                manifest,
                wasm_bytes,
            },
        );

        Ok(())
    }

    /// Unload an installed plugin
    pub fn uninstall_plugin(&self, plugin_id: &str) -> Result<(), PluginHostError> {
        let mut lock = self.plugins.write();
        if lock.remove(plugin_id).is_some() {
            Ok(())
        } else {
            Err(PluginHostError::PluginNotFound(plugin_id.to_string()))
        }
    }

    /// List all installed plugin manifests
    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        let lock = self.plugins.read();
        lock.values().map(|p| p.manifest.clone()).collect()
    }

    /// Hook: `on_qr_scanned`
    /// Allows plugins with `hook:on_qr_scanned` permission to intercept proprietary supplier barcodes.
    pub fn hook_on_qr_scanned(
        &self,
        raw_payload: &str,
    ) -> Result<Option<PluginQrInterceptResult>, PluginHostError> {
        let lock = self.plugins.read();

        for (plugin_id, plugin) in lock.iter() {
            if !plugin.manifest.has_permission("hook:on_qr_scanned") {
                continue;
            }

            let extism_manifest = ExtismManifest::new([Wasm::data(plugin.wasm_bytes.clone())]);
            let mut instance = Plugin::new(&extism_manifest, [], true)
                .map_err(|e| PluginHostError::Extism(e.to_string()))?;

            if !instance.function_exists("on_qr_scanned") {
                continue;
            }

            let output_bytes = instance
                .call("on_qr_scanned", raw_payload)
                .map_err(|e| PluginHostError::ExecutionFailed {
                    plugin_id: plugin_id.clone(),
                    function: "on_qr_scanned".into(),
                    error: e.to_string(),
                })?;

            if let Ok(res) = serde_json::from_slice::<PluginQrInterceptResult>(output_bytes) {
                if res.handled {
                    return Ok(Some(res));
                }
            }
        }

        Ok(None)
    }

    /// Hook: `render_ui`
    /// Plugins can declare dynamic UI extensions rendered natively in Compose Multiplatform.
    pub fn hook_render_ui(
        &self,
        context_view: &str,
    ) -> Result<Vec<PluginUiNode>, PluginHostError> {
        let lock = self.plugins.read();
        let mut nodes = Vec::new();

        for (plugin_id, plugin) in lock.iter() {
            if !plugin.manifest.has_permission("ui:custom_field")
                && !plugin.manifest.has_permission("ui:extension")
            {
                continue;
            }

            let extism_manifest = ExtismManifest::new([Wasm::data(plugin.wasm_bytes.clone())]);
            let mut instance = Plugin::new(&extism_manifest, [], true)
                .map_err(|e| PluginHostError::Extism(e.to_string()))?;

            if !instance.function_exists("render_ui") {
                continue;
            }

            let output_bytes = instance
                .call("render_ui", context_view)
                .map_err(|e| PluginHostError::ExecutionFailed {
                    plugin_id: plugin_id.clone(),
                    function: "render_ui".into(),
                    error: e.to_string(),
                })?;

            if let Ok(mut plugin_nodes) = serde_json::from_slice::<Vec<PluginUiNode>>(output_bytes) {
                nodes.append(&mut plugin_nodes);
            }
        }

        Ok(nodes)
    }
}
