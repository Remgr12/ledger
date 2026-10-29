use erp_kernel::{DomainEvent, ErpKernelError, ErpModule, Migration, ModuleContext, ModuleRoute};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::migrations::get_migrations;
use crate::models::MovementType;
use crate::stock_service::StockService;

pub struct InventoryModule;

impl Default for InventoryModule {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for InventoryModule {
    fn id(&self) -> &'static str {
        "mod-inventory"
    }

    fn name(&self) -> &'static str {
        "Spatial Inventory & Stock Ledger"
    }

    fn description(&self) -> &'static str {
        "Hierarchical storage locations, double-entry stock ledger, and smart QR code workflows"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn migrations(&self) -> Vec<Migration> {
        get_migrations()
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "inventory/locator".to_string(),
                title: "Where Is It?".to_string(),
                icon: "search".to_string(),
                display_order: 10,
            },
            ModuleRoute {
                route_id: "inventory/scanner".to_string(),
                title: "Scan & Move HUD".to_string(),
                icon: "qr_code_scanner".to_string(),
                display_order: 11,
            },
            ModuleRoute {
                route_id: "inventory/tree".to_string(),
                title: "Storage Tree".to_string(),
                icon: "account_tree".to_string(),
                display_order: 12,
            },
            ModuleRoute {
                route_id: "inventory/items".to_string(),
                title: "Item Catalog".to_string(),
                icon: "inventory_2".to_string(),
                display_order: 13,
            },
            ModuleRoute {
                route_id: "inventory/labels".to_string(),
                title: "Label Printer".to_string(),
                icon: "print".to_string(),
                display_order: 14,
            },
        ]
    }

    fn on_event(&self, event: &DomainEvent, ctx: &ModuleContext) -> Result<(), ErpKernelError> {
        // Inter-module automation: Automatically pick stock when an invoice is finalized
        if event.event_type == "InvoiceFinalized" {
            tracing::info!(
                "mod-inventory: processing InvoiceFinalized event {}",
                event.event_id
            );

            if let Some(items) = event.payload.get("line_items").and_then(|v| v.as_array()) {
                for item_val in items {
                    let item_id = item_val
                        .get("item_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let qty_str = item_val
                        .get("quantity")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");

                    let requested_qty = Decimal::from_str(qty_str).unwrap_or(Decimal::ZERO);
                    if requested_qty <= Decimal::ZERO || item_id.is_empty() {
                        continue;
                    }

                    // Find location that currently holds stock for this item
                    let source_loc_id = {
                        let conn = ctx.db.conn();
                        conn.query_row(
                            r#"
                            SELECT location_id FROM stock_quants
                            WHERE item_id = ?1 AND CAST(quantity AS REAL) >= CAST(?2 AS REAL)
                            LIMIT 1
                            "#,
                            rusqlite::params![item_id, qty_str],
                            |r| r.get::<_, String>(0),
                        )
                        .ok()
                    };

                    if let Some(from_loc) = source_loc_id {
                        let _ = StockService::record_movement(
                            ctx,
                            MovementType::Pick,
                            item_id,
                            Some(&from_loc),
                            None,
                            requested_qty,
                            None,
                            None,
                            Some(&event.aggregate_id),
                            Some("Automatic deduction from finalized invoice"),
                            &event.actor_id,
                        )
                        .map_err(|e| {
                            tracing::warn!("Failed automatic stock pick for invoice: {}", e);
                            e
                        });
                    } else {
                        tracing::warn!(
                            "No location with sufficient stock found for item {} on invoice {}",
                            item_id,
                            event.aggregate_id
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
