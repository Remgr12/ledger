uniffi::setup_scaffolding!();

pub mod dto;
pub mod engine;
pub mod errors;

pub use dto::*;
pub use engine::{init_erp, ErpEngine};
pub use errors::ErpFfiError;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_ffi_engine_lifecycle_and_workflows() {
        let engine = init_erp(":memory:".to_string()).expect("init erp");

        // 1. Module Management
        let modules = engine.get_available_modules().expect("list modules");
        assert_eq!(modules.len(), 7);
        assert!(modules.iter().any(|m| m.id == "mod-inventory"));
        assert!(modules.iter().any(|m| m.id == "mod-contacts"));
        assert!(modules.iter().any(|m| m.id == "mod-invoicing"));
        assert!(modules.iter().any(|m| m.id == "mod-accounting"));
        assert!(modules.iter().any(|m| m.id == "mod-purchasing"));
        assert!(modules.iter().any(|m| m.id == "mod-manufacturing"));
        assert!(modules.iter().any(|m| m.id == "mod-projects"));

        // Enable all modules
        for mod_id in &[
            "mod-inventory",
            "mod-contacts",
            "mod-invoicing",
            "mod-accounting",
            "mod-purchasing",
            "mod-manufacturing",
            "mod-projects",
        ] {
            engine.set_module_enabled(mod_id.to_string(), true).expect("enable module");
        }

        // 2. Spatial Inventory Hierarchy
        let wh = engine
            .create_location(
                "WH-MAIN".into(),
                "Main Facility".into(),
                None,
                "Warehouse".into(),
                Some(0.0),
                Some(0.0),
            )
            .expect("create wh");

        let rack = engine
            .create_location(
                "RACK-1".into(),
                "Rack 1".into(),
                Some(wh.id.clone()),
                "Rack".into(),
                Some(10.0),
                Some(20.0),
            )
            .expect("create rack");

        let bin = engine
            .create_location(
                "BIN-42".into(),
                "Bin 42".into(),
                Some(rack.id.clone()),
                "Bin".into(),
                Some(12.0),
                Some(22.0),
            )
            .expect("create bin");

        assert_eq!(bin.path_breadcrumb, "Main Facility > Rack 1 > Bin 42");

        // 3. Item Catalog & Initial Stock Movement
        let item = engine
            .create_item(
                "STAINLESS-SCREW-M4".into(),
                "M4 Pan Head Stainless Screw".into(),
                Some("Box of 100".into()),
                "box".into(),
                false,
                false,
                "10".into(),
                Some("998877665544".to_string()),
            )
            .expect("create item");

        // Move stock in (Receive)
        engine
            .move_stock(
                item.id.clone(),
                None,
                Some(bin.id.clone()),
                "50".into(),
                "Receive".into(),
                None,
                Some("Initial batch".into()),
            )
            .expect("receive stock");

        // 4. Instant "Where Is It?" search
        let matches = engine
            .search_item_locations("SCREW-M4".into())
            .expect("search locations");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].location_name, "Bin 42");
        assert_eq!(
            matches[0].breadcrumb_path,
            "Main Facility > Rack 1 > Bin 42"
        );
        assert_eq!(matches[0].quantity, "50");

        // 5. Dual-Entity Smart QR Resolution
        // Scan Location QR
        let res_loc = engine
            .resolve_qr_scan(format!("erp://loc/{}", bin.id), None)
            .expect("resolve loc qr");
        assert_eq!(res_loc.location.unwrap().code, "BIN-42");

        // Scan Item QR with active location context -> RapidStockAction
        let res_rapid = engine
            .resolve_qr_scan("erp://item/STAINLESS-SCREW-M4".into(), Some(bin.id.clone()))
            .expect("resolve rapid qr");
        assert_eq!(
            res_rapid.prompt_message,
            "Putaway/Move 'M4 Pan Head Stainless Screw' into 'Bin 42'?"
        );

        // 6. Offline SVG Label generation
        let svg = engine
            .generate_qr_label_svg(
                "erp://loc/BIN-42".into(),
                "BIN-42".into(),
                "Main Facility > Rack 1 > Bin 42".into(),
            )
            .expect("generate svg");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("BIN-42"));

        // 7. Contacts & Invoicing with Inventory Reaction
        let contact = engine
            .create_contact(
                "Customer".into(),
                "Global Logistics Corp".into(),
                Some("contact@globallogistics.com".into()),
                None,
                None,
                None,
            )
            .expect("create contact");

        let inv = mod_invoicing::InvoicingService::create_invoice(
            &engine.module_manager.context(),
            "INV-1001",
            &contact.id,
            vec![(
                Some(item.id.clone()),
                "M4 Screws".into(),
                rust_decimal::Decimal::from(5),
                rust_decimal::Decimal::from(12),
                rust_decimal::Decimal::ZERO,
            )],
            None,
        )
        .expect("create inv");

        // Finalize invoice -> emits InvoiceFinalized -> mod-inventory auto-picks 5 units!
        let finalized = engine.finalize_invoice(inv.id).expect("finalize");
        assert_eq!(finalized.status, "Finalized");

        // Check stock remaining at bin: 50 - 5 = 45!
        let matches_after = engine
            .search_item_locations("SCREW-M4".into())
            .expect("search locations after invoice");
        assert_eq!(matches_after[0].quantity, "45");

        // 8. Accounting & Trial Balance verification:
        // Invoice finalized -> mod-accounting auto-created balanced GL entries!
        let trial_balance = engine.get_trial_balance().expect("get trial balance");
        assert_eq!(trial_balance.total_debit, trial_balance.total_credit);
        assert_eq!(
            rust_decimal::Decimal::from_str(&trial_balance.total_debit).unwrap(),
            rust_decimal::Decimal::from(60)
        );

        // 9. Purchasing: Create PO and Receive Goods
        let po = engine
            .create_purchase_order(
                "PO-9001".into(),
                contact.id.clone(),
                Some("2026-11-20".into()),
                vec![PurchaseOrderLineDto {
                    id: String::new(),
                    item_id: item.id.clone(),
                    description: "M4 Screws Batch".into(),
                    quantity: "100".into(),
                    received_quantity: "0".into(),
                    unit_price: "8.50".into(),
                    tax_rate: "0.0".into(),
                    line_total: "850.00".into(),
                }],
                Some("Bulk supplier order".into()),
            )
            .expect("create po");

        let submitted_po = engine.submit_purchase_order(po.id.clone()).expect("submit po");
        assert_eq!(submitted_po.status, "Submitted");

        let receipt = engine
            .receive_goods(
                Some(po.id),
                contact.id.clone(),
                bin.id.clone(),
                vec![PurchaseReceiptLineDto {
                    id: String::new(),
                    item_id: item.id.clone(),
                    quantity: "100".into(),
                    lot_number: Some("LOT-PO-9001".into()),
                    serial_number: None,
                }],
                Some("Received at dock".into()),
            )
            .expect("receive goods");
        assert_eq!(receipt.lines.len(), 1);

        // Stock at bin: 45 without lot + 100 with lot = 145 total across 2 quants
        let matches_after_po = engine.search_item_locations("SCREW-M4".into()).unwrap();
        assert_eq!(matches_after_po.len(), 2);
        let total_on_hand: Decimal = matches_after_po
            .iter()
            .map(|m| Decimal::from_str(&m.quantity).unwrap())
            .sum();
        assert_eq!(total_on_hand, Decimal::from(145));

        // 10. Projects & Timesheets:
        let project = engine
            .create_project(
                "Warehouse Automation".into(),
                "PRJ-AUTO-01".into(),
                Some(contact.id.clone()),
                "120.00".into(),
                "20".into(),
            )
            .expect("create project");

        let task = engine
            .create_task(
                project.id.clone(),
                "Conveyor belt sensor setup".into(),
                "High".into(),
                "5".into(),
            )
            .expect("create task");

        let ts = engine
            .record_timesheet(
                project.id.clone(),
                Some(task.id),
                "eng_lead".into(),
                "2026-10-29".into(),
                "5.0".into(),
                None,
                true,
                "Installed photoelectric sensor and connected to ERP".into(),
            )
            .expect("record ts");
        assert_eq!(
            Decimal::from_str(&ts.billing_amount).unwrap(),
            Decimal::from(600)
        );

        let project_inv = engine
            .invoice_unbilled_timesheets(project.id, "INV-PRJ-001".into(), "0.0".into())
            .expect("invoice timesheets");
        assert_eq!(
            Decimal::from_str(&project_inv.subtotal).unwrap(),
            Decimal::from(600)
        );
    }

    #[test]
    fn test_wasm_plugin_real_execution() {
        let engine = init_erp(":memory:".to_string()).expect("init erp");
        engine
            .set_module_enabled("mod-inventory".into(), true)
            .expect("enable inventory");

        // Create the item that the plugin will translate to: "M8-HEX-BOLT-316"
        let _bolt = engine
            .create_item(
                "M8-HEX-BOLT-316".into(),
                "M8x40mm Hex Bolt Stainless 316".into(),
                Some("Industrial grade bolt".into()),
                "pcs".into(),
                false,
                false,
                "50".into(),
                None,
            )
            .expect("create bolt");

        // Read real compiled Wasm plugin binary
        let wasm_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../plugins/sample_supplier_plugin.wasm"
        );
        let wasm_bytes = std::fs::read(wasm_path).expect("read sample_supplier_plugin.wasm");

        let manifest_json = serde_json::json!({
            "id": "acme-supplier-plugin",
            "name": "Acme Supplier Barcode Parser",
            "version": "1.0.0",
            "description": "Intercepts SUPP- format supplier barcodes and renders ISO vendor cards",
            "permissions": ["hook:on_qr_scanned", "ui:custom_field"]
        }).to_string();

        engine
            .install_wasm_plugin(manifest_json, wasm_bytes)
            .expect("install wasm plugin");

        // Intercept vendor QR scan
        let res = engine
            .resolve_qr_scan("SUPP-VEND10-SKU-M8BOLT-LOT2026A".into(), None)
            .expect("resolve supplier qr scan");

        // The plugin translated the barcode to M8-HEX-BOLT-316!
        assert_eq!(res.resolution_type, ScanResolutionTypeDto::ItemLookup);
        assert_eq!(res.item.unwrap().sku, "M8-HEX-BOLT-316");

        // Verify dynamic UI nodes rendered by plugin
        let ui_nodes = engine
            .get_plugin_ui_nodes("inventory_detail".into())
            .expect("get plugin ui nodes");
        assert!(!ui_nodes.is_empty());
        assert_eq!(ui_nodes[0].node_kind, "Badge");
        assert_eq!(ui_nodes[0].title_or_label, "Supplier Verified: ISO-9001");
        assert_eq!(ui_nodes[0].color_hex.as_deref(), Some("#10B981"));
    }
}
