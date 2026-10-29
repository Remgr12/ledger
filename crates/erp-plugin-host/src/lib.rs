pub mod errors;
pub mod host;
pub mod manifest;
pub mod ui_node;

pub use errors::PluginHostError;
pub use host::{PluginHost, PluginQrInterceptResult};
pub use manifest::PluginManifest;
pub use ui_node::{PluginUiNode, PluginUiNodeType};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_permissions() {
        let manifest = PluginManifest {
            id: "test-plugin".into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            permissions: vec!["read:inventory".into(), "hook:on_qr_scanned".into()],
        };

        assert!(manifest.has_permission("read:inventory"));
        assert!(manifest.has_permission("hook:on_qr_scanned"));
        assert!(!manifest.has_permission("write:accounting"));
    }

    #[test]
    fn test_ui_node_composition() {
        let title = PluginUiNode::text("Supplier Lot Tracking", "Title");
        let badge = PluginUiNode::badge("GS1-128 Verified", "#10B981");
        let card = PluginUiNode::container("Row", 8, vec![title, badge]);

        assert_eq!(card.children.len(), 2);
        let serialized = serde_json::to_string(&card).expect("serialize");
        assert!(serialized.contains("GS1-128 Verified"));
    }

    #[test]
    fn test_plugin_installation_validation() {
        let host = PluginHost::new();

        // 8-byte valid WebAssembly module header
        let valid_wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        let manifest = PluginManifest {
            id: "gs1-scanner".into(),
            name: "GS1 Scanner".into(),
            version: "0.1.0".into(),
            description: "Decodes GS1 Datamatrix".into(),
            permissions: vec!["hook:on_qr_scanned".into()],
        };

        let result = host.install_plugin(manifest.clone(), valid_wasm);
        assert!(result.is_ok());

        let plugins = host.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "gs1-scanner");

        let uninstall = host.uninstall_plugin("gs1-scanner");
        assert!(uninstall.is_ok());
        assert_eq!(host.list_plugins().len(), 0);
    }

    #[test]
    fn test_wasm_plugin_execution_with_wat() {
        let wat_source = r#"
            (module
                (memory (export "memory") 1)
                (func (export "on_qr_scanned") (result i32)
                    i32.const 0
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat_source).expect("parse wat");

        let host = PluginHost::new();
        let manifest = PluginManifest {
            id: "wat-scanner".into(),
            name: "WAT Scanner".into(),
            version: "1.0.0".into(),
            description: "Sample WAT compiled plugin".into(),
            permissions: vec!["hook:on_qr_scanned".into()],
        };

        host.install_plugin(manifest, wasm_bytes).expect("install plugin");
        let result = host.hook_on_qr_scanned("vendor-raw-12345");
        assert!(result.is_ok());
    }
}
