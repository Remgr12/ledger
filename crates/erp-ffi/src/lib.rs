uniffi::setup_scaffolding!();

pub mod dto;
pub mod engine;
pub mod errors;
pub mod pdf_generator;

pub use dto::*;
pub use engine::{init_erp, ErpEngine};
pub use errors::ErpFfiError;
pub use pdf_generator::*;

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

    #[test]
    fn test_v0_0_3_core_capabilities() {
        println!(">>> STARTING test_v0_0_3_core_capabilities");
        let engine = init_erp(":memory:".to_string()).expect("init erp");
        for mod_id in [
            "mod-accounting",
            "mod-invoicing",
            "mod-purchasing",
            "mod-manufacturing",
            "mod-projects",
        ] {
            engine.set_module_enabled(mod_id.to_string(), true).expect("enable module");
        }
        println!(">>> Modules enabled");

        // 1. Vector PDF Generation (LEDGER-11)
        let contact = engine
            .create_contact(
                "Customer".into(),
                "Apex Robotics Inc".into(),
                Some("billing@apexrobotics.com".into()),
                None,
                None,
                None,
            )
            .expect("create contact");
        println!(">>> Contact created");

        let inv = mod_invoicing::InvoicingService::create_invoice(
            &engine.module_manager.context(),
            "INV-2026-999",
            &contact.id,
            vec![(
                None,
                "Precision CNC Milled Aluminum Bracket".into(),
                Decimal::from(4),
                Decimal::from(150),
                Decimal::from_str("0.20").unwrap(),
            )],
            Some("Net 30 SEPA payment transfer".into()),
        )
        .expect("create invoice");
        println!(">>> Invoice created");

        let pdf_bytes = engine.generate_invoice_pdf(inv.id.clone()).expect("generate invoice pdf");
        println!(">>> PDF generated: {} bytes", pdf_bytes.len());
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.starts_with("%PDF-1.4"));
        assert!(pdf_str.ends_with("%%EOF\n"));
        assert!(pdf_str.contains("INV-2026-999"));
        assert!(pdf_str.contains("Apex Robotics Inc"));
        // Check EPC QR payment code markers
        assert!(pdf_str.contains("SEPA EPC QR Payment"));
        assert!(pdf_str.contains("DE89 3704 0044 0532 0130 00"));
        println!(">>> Invoice PDF verified");

        // Purchase Order PDF
        let po = engine
            .create_purchase_order(
                "PO-2026-888".into(),
                contact.id.clone(),
                Some("2026-12-01".into()),
                vec![PurchaseOrderLineDto {
                    id: String::new(),
                    item_id: "item-sample-1".into(),
                    description: "Raw 6061-T6 Aluminum Billet 50mm".into(),
                    quantity: "20".into(),
                    received_quantity: "0".into(),
                    unit_price: "45.00".into(),
                    tax_rate: "0.0".into(),
                    line_total: "900.00".into(),
                }],
                Some("Expedited delivery requested".into()),
            )
            .expect("create po");

        let po_pdf = engine.generate_purchase_order_pdf(po.id.clone()).expect("generate po pdf");
        println!(">>> PO PDF generated");
        let po_pdf_str = String::from_utf8_lossy(&po_pdf);
        assert!(po_pdf_str.starts_with("%PDF-1.4"));
        assert!(po_pdf_str.contains("PO-2026-888"));
        assert!(po_pdf_str.contains("Raw 6061-T6 Aluminum Billet 50mm"));

        // 2. Peer-to-Peer LAN / mDNS Sync (LEDGER-12)
        println!(">>> Checking sync status");
        let sync_status = engine.get_sync_status().expect("get sync status");
        assert_eq!(sync_status.connected_peers_count, 2);
        assert!(!sync_status.local_hlc.is_empty());

        engine
            .register_peer(
                "node-laptop-03".into(),
                "Craftsman Linux Laptop".into(),
                "192.168.1.199".into(),
                8080,
            )
            .expect("register peer");
        println!(">>> Peer registered");

        let updated_sync = engine.get_sync_status().expect("updated sync");
        assert_eq!(updated_sync.connected_peers_count, 3);
        assert!(updated_sync.peers.iter().any(|p| p.node_id == "node-laptop-03"));

        println!(">>> Triggering LAN sync");
        let synced = engine.trigger_lan_sync().expect("trigger lan sync");
        println!(">>> Synced: {:?}", synced.convergence_state);
        assert_eq!(synced.unsynced_events_count, 0);
        assert_eq!(synced.convergence_state, "Converged");

        // 3. Low-Stock Alerts & Automated Reorder PO (LEDGER-13)
        println!(">>> Checking low-stock alerts");
        let low_stock_item = engine
            .create_item(
                "CRITICAL-BEARING-608".into(),
                "High Speed Bearing 608".into(),
                Some("Essential motor part".into()),
                "pcs".into(),
                false,
                false,
                "100".into(), // min stock = 100
                None,
            )
            .expect("create item");

        let alerts = engine.get_low_stock_alerts().expect("get low stock alerts");
        assert!(alerts.iter().any(|a| a.sku == "CRITICAL-BEARING-608"));

        let reorder_po = engine.generate_reorder_po(low_stock_item.id).expect("generate reorder po");
        assert!(reorder_po.po_number.starts_with("PO-REORDER-"));
        assert!(!reorder_po.lines.is_empty());
        println!(">>> Reorder PO generated");

        // 4. Kanban Workflow Board & Component Genealogy (LEDGER-14 & LEDGER-15)
        let wh = engine
            .create_location(
                "WH-TEST".into(),
                "Assembly Bay".into(),
                None,
                "Warehouse".into(),
                Some(0.0),
                Some(0.0),
            )
            .expect("create loc");

        let raw_comp = engine
            .create_item(
                "RAW-MOTOR-01".into(),
                "NEMA 17 Stepper Motor".into(),
                None,
                "pcs".into(),
                false,
                true,
                "10".into(),
                None,
            )
            .expect("create raw comp");

        let finished_good = engine
            .create_item(
                "FINISHED-ASSEMBLY-01".into(),
                "Actuator Assembly Model X".into(),
                None,
                "pcs".into(),
                false,
                true,
                "5".into(),
                None,
            )
            .expect("create finished good");

        // Receive stock of raw component with lot
        println!(">>> Moving stock");
        engine
            .move_stock(
                raw_comp.id.clone(),
                None,
                Some(wh.id.clone()),
                "20".into(),
                "Receive".into(),
                Some("LOT-MOTOR-BATCH-01".into()),
                Some("Initial batch".into()),
            )
            .expect("receive motor stock");

        // Register lot details in inventory_lots
        println!(">>> Registering lot details");
        engine
            .create_or_update_lot(
                "LOT-MOTOR-BATCH-01".into(),
                raw_comp.id.clone(),
                Some("2029-06-30".into()),
                Some("released".into()),
                Some("STEPPER-MFR-771".into()),
            )
            .expect("create lot");

        let lot_info = engine
            .get_lot_info("LOT-MOTOR-BATCH-01".into())
            .expect("get lot info")
            .expect("lot found");
        assert_eq!(lot_info.manufacturer_batch.as_deref(), Some("STEPPER-MFR-771"));
        assert_eq!(lot_info.quarantine_status, "released");
        println!(">>> Lot info verified");

        // Create BOM and Work Order
        println!(">>> Creating BOM");
        let bom = engine
            .create_bom(
                "BOM-ACTUATOR-01".into(),
                finished_good.id.clone(),
                "Actuator Assembly Formula".into(),
                "1".into(),
                vec![BomComponentDto {
                    id: String::new(),
                    item_id: raw_comp.id.clone(),
                    quantity: "2".into(),
                    scrap_percentage: "0.0".into(),
                    notes: None,
                }],
                Some("Precision calibration step required".into()),
            )
            .expect("create bom");

        println!(">>> Creating Work Order");
        let wo = engine
            .create_work_order(
                "WO-ACTUATOR-101".into(),
                bom.id,
                "2".into(),
                wh.id.clone(),
                wh.id.clone(),
            )
            .expect("create wo");
        assert_eq!(wo.status, "Draft");
        println!(">>> WO created: {}", wo.status);

        // Kanban status advance: Draft/Planned -> In Progress -> Quality Check
        let wo_prog = engine
            .update_work_order_status(wo.id.clone(), "In Progress".into())
            .expect("advance to in progress");
        assert_eq!(wo_prog.status, "In Progress");
        println!(">>> WO In Progress");

        let wo_qc = engine
            .update_work_order_status(wo.id.clone(), "Quality Check".into())
            .expect("advance to qc");
        assert_eq!(wo_qc.status, "Quality Check");
        println!(">>> WO Quality Check");

        // Complete work order -> triggers stock consumption and genealogy linking
        let wo_done = engine
            .update_work_order_status(wo.id.clone(), "Completed".into())
            .expect("advance to completed");
        assert_eq!(wo_done.status, "Completed");
        println!(">>> WO Completed");

        // Verify component genealogy tree
        println!(">>> Getting genealogy");
        let genealogy = engine
            .get_work_order_genealogy(wo.id.clone())
            .expect("get genealogy")
            .expect("genealogy tree found");
        println!(">>> Got genealogy: components len = {}", genealogy.components.len());
        assert_eq!(genealogy.work_order_number, "WO-ACTUATOR-101");
        assert!(genealogy.finished_lot_number.starts_with("LOT-MFG-"));
        assert!(!genealogy.components.is_empty());
        assert_eq!(genealogy.components[0].component_sku, "RAW-MOTOR-01");
        assert_eq!(genealogy.components[0].component_lot_number.as_deref(), Some("LOT-MOTOR-BATCH-01"));

        // 5. Project Task Kanban & Unbilled Hours
        println!(">>> Creating project");
        let project = engine
            .create_project(
                "Robotic Cell Installation".into(),
                "PRJ-ROBOT-01".into(),
                Some(contact.id.clone()),
                "150.00".into(),
                "40".into(),
            )
            .expect("create project");

        println!(">>> Creating task 1");
        let task1 = engine
            .create_task(project.id.clone(), "Safety Enclosure Assembly".into(), "High".into(), "12".into())
            .expect("create task 1");
        println!(">>> Creating task 2");
        let task2 = engine
            .create_task(project.id.clone(), "Wiring & Sensor Setup".into(), "Urgent".into(), "8".into())
            .expect("create task 2");

        println!(">>> Listing tasks");
        let tasks = engine.list_project_tasks(Some(project.id.clone())).expect("list tasks");
        assert_eq!(tasks.len(), 2);

        // Update task status on Kanban board
        println!(">>> Updating task status");
        let task1_prog = engine
            .update_project_task_status(task1.id, "In Progress".into())
            .expect("update task status");
        assert_eq!(task1_prog.status, "In Progress");

        // Record time and verify unbilled hours summary
        println!(">>> Recording timesheet");
        engine
            .record_timesheet(
                project.id.clone(),
                Some(task2.id),
                "robotics_tech".into(),
                "2026-10-31".into(),
                "4.5".into(),
                None,
                true,
                "Mounted electrical box and ran harness".into(),
            )
            .expect("record ts");

        println!(">>> Getting unbilled summary");
        let unbilled = engine.get_unbilled_hours_summary(Some(project.id)).expect("get unbilled summary");
        assert_eq!(unbilled.total_unbilled_hours, "4.5");
        assert_eq!(Decimal::from_str(&unbilled.total_unbilled_amount).unwrap(), Decimal::from_str("675.00").unwrap());
        assert_eq!(unbilled.unbilled_entries_count, 1);
        println!(">>> ALL V0.0.3 TESTS COMPLETED SUCCESSFULLY!");
    }
}
