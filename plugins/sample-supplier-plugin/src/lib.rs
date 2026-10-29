use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct PluginQrInterceptResult {
    pub handled: bool,
    pub translated_sku: Option<String>,
    pub metadata: HashMap<String, String>,
    pub display_note: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum PluginUiNodeType {
    Container {
        direction: String,
        spacing: u32,
    },
    Text {
        content: String,
        style: String,
    },
    Badge {
        label: String,
        color_hex: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct PluginUiNode {
    pub node_id: String,
    pub node_type: PluginUiNodeType,
    pub children: Vec<PluginUiNode>,
}

#[plugin_fn]
pub fn on_qr_scanned(raw_payload: String) -> FnResult<Json<PluginQrInterceptResult>> {
    if raw_payload.starts_with("SUPP-") {
        let parts: Vec<&str> = raw_payload.split('-').collect();
        let mut metadata = HashMap::new();
        metadata.insert(
            "supplier_prefix".to_string(),
            parts.get(1).unwrap_or(&"UNKNOWN").to_string(),
        );
        metadata.insert(
            "supplier_name".to_string(),
            "Acme Fasteners Corp".to_string(),
        );

        let translated = if raw_payload.contains("M8BOLT") {
            Some("M8-HEX-BOLT-316".to_string())
        } else {
            None
        };

        Ok(Json(PluginQrInterceptResult {
            handled: true,
            translated_sku: translated,
            metadata,
            display_note: Some(
                "Decoded proprietary supplier barcode format [Acme Fasteners V1]".to_string(),
            ),
        }))
    } else {
        Ok(Json(PluginQrInterceptResult {
            handled: false,
            translated_sku: None,
            metadata: HashMap::new(),
            display_note: None,
        }))
    }
}

#[plugin_fn]
pub fn render_ui(context_view: String) -> FnResult<Json<Vec<PluginUiNode>>> {
    if context_view == "inventory_detail" {
        let nodes = vec![
            PluginUiNode {
                node_id: "badge-iso".into(),
                node_type: PluginUiNodeType::Badge {
                    label: "Supplier Verified: ISO-9001".into(),
                    color_hex: "#10B981".into(),
                },
                children: vec![],
            },
            PluginUiNode {
                node_id: "text-vendor".into(),
                node_type: PluginUiNodeType::Text {
                    content: "Primary Vendor: Acme Fasteners Corp (Net 30)".into(),
                    style: "Subheading".into(),
                },
                children: vec![],
            },
        ];
        Ok(Json(nodes))
    } else {
        Ok(Json(vec![]))
    }
}
