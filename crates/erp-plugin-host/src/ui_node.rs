use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginUiNodeType {
    Container {
        direction: String, // "Row" or "Column"
        spacing: u32,
    },
    Card {
        elevation: u32,
    },
    Text {
        content: String,
        style: String, // "Title", "Body", "Caption", "Headline"
    },
    Badge {
        label: String,
        color_hex: String,
    },
    StatCard {
        label: String,
        value: String,
        delta: Option<String>,
    },
    Button {
        id: String,
        label: String,
        style: String, // "Primary", "Secondary", "Danger"
    },
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    CustomFieldInput {
        field_key: String,
        label: String,
        default_value: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginUiNode {
    pub node_id: String,
    pub node_type: PluginUiNodeType,
    pub children: Vec<PluginUiNode>,
}

impl PluginUiNode {
    pub fn text(content: impl Into<String>, style: impl Into<String>) -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_type: PluginUiNodeType::Text {
                content: content.into(),
                style: style.into(),
            },
            children: Vec::new(),
        }
    }

    pub fn badge(label: impl Into<String>, color_hex: impl Into<String>) -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_type: PluginUiNodeType::Badge {
                label: label.into(),
                color_hex: color_hex.into(),
            },
            children: Vec::new(),
        }
    }

    pub fn stat_card(label: impl Into<String>, value: impl Into<String>, delta: Option<String>) -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_type: PluginUiNodeType::StatCard {
                label: label.into(),
                value: value.into(),
                delta,
            },
            children: Vec::new(),
        }
    }

    pub fn container(direction: impl Into<String>, spacing: u32, children: Vec<PluginUiNode>) -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_type: PluginUiNodeType::Container {
                direction: direction.into(),
                spacing,
            },
            children,
        }
    }
}
