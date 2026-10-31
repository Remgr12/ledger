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

    #[test]
    fn test_v004_features_integration() {
        let engine = init_erp(":memory:".to_string()).expect("init erp for v0.0.4 tests");

        // 1. LEDGER-16: Multi-up Sheet PDF Layouts & Barcode Printing
        let labels = vec![
            MultiUpLabelItemDto {
                item_id: "item-1".into(),
                sku: "SKU-BOLT-M8".into(),
                name: "Hex Bolt M8x40".into(),
                qr_code_uri: "erp://sku/SKU-BOLT-M8".into(),
                barcode: Some("4012345678901".into()),
                lot_or_serial: Some("LOT-2026-A".into()),
                price: Some("0.45".into()),
            },
            MultiUpLabelItemDto {
                item_id: "item-2".into(),
                sku: "SKU-NUT-M8".into(),
                name: "Hex Nut M8 Zinc".into(),
                qr_code_uri: "erp://sku/SKU-NUT-M8".into(),
                barcode: Some("4012345678902".into()),
                lot_or_serial: Some("LOT-2026-B".into()),
                price: Some("0.20".into()),
            },
        ];
        let pdf_avery = engine.generate_multi_up_labels_pdf("Avery5160".into(), labels.clone()).expect("generate avery pdf");
        assert!(pdf_avery.starts_with(b"%PDF-1.4"));
        assert!(pdf_avery.len() > 200);

        let print_progress = engine.batch_stream_thermal_labels(labels).expect("batch stream");
        assert_eq!(print_progress.total_labels, 2);
        assert!(print_progress.is_completed);

        // 2. LEDGER-17: BLE Barcode Scanners & Digital Weight Scale Integration
        let ble_devs = engine.list_ble_devices().expect("list ble");
        assert!(ble_devs.len() >= 3);
        let scanner = engine.connect_ble_device("ble-ds3678".into()).expect("connect scanner");
        assert!(scanner.is_connected);

        let scale_reading = engine.get_scale_reading("ble-ps60".into(), Some(50.0), Some(2.5)).expect("scale reading");
        assert_eq!(scale_reading.raw_weight_grams, 1450.0);
        assert_eq!(scale_reading.tare_weight_grams, 50.0);
        assert_eq!(scale_reading.net_weight_grams, 1400.0);
        assert_eq!(scale_reading.calculated_pieces, Some(560)); // 1400 / 2.5 = 560

        let pieces = engine.calculate_piece_count(500.0, 5.0).expect("piece count");
        assert_eq!(pieces, 100);

        // 3. LEDGER-18: Multi-Currency Accounting & Offline FX Tables
        let rate_usd = engine.set_exchange_rate("EUR".into(), "USD".into(), 1.085, "2026-10-31".into()).expect("set fx");
        assert_eq!(rate_usd.target_currency, "USD");
        assert_eq!(rate_usd.rate, 1.085);

        let rates = engine.get_exchange_rates().expect("get fx rates");
        assert!(!rates.is_empty());

        let fx_res = engine.record_fx_gain_loss("INV-2026-001".into(), 1.08, 1.10, "1000.00".into(), "USD".into()).expect("record fx");
        assert_eq!(fx_res.realized_gain_loss, "20.00");

        // 4. LEDGER-19: Recurring Subscription Invoicing & Client Retainer Deposits
        let cust = engine.create_contact("Customer".into(), "Enterprise Client GmbH".into(), Some("info@client.de".into()), None, None, None).expect("create contact");
        let sched = engine.create_recurring_schedule(cust.id.clone(), "Monthly Maintenance Tier 1".into(), "Monthly".into(), "499.00".into(), "EUR".into(), "2026-11-01".into()).expect("create sched");
        assert_eq!(sched.frequency, "Monthly");
        let sched_list = engine.list_recurring_schedules().expect("list sched");
        assert_eq!(sched_list.len(), 1);

        let triggered = engine.trigger_recurring_invoices("2026-11-01".into()).expect("trigger recurring");
        assert_eq!(triggered, 1);

        let ret_dep = engine.deposit_retainer(cust.id.clone(), "2500.00".into(), "EUR".into()).expect("deposit retainer");
        assert_eq!(ret_dep.balance, "2500.00");

        let ret_draw = engine.drawdown_retainer(cust.id.clone(), "500.00".into(), "INV-999".into()).expect("drawdown retainer");
        assert_eq!(ret_draw.balance, "2000.00");

        // 5. LEDGER-20: ABC Inventory Pareto Analysis & Dynamic EOQ
        let loc1 = engine.create_location("WH-ABC".into(), "ABC Warehouse".into(), None, "Warehouse".into(), None, None).expect("loc");
        let it1 = engine.create_item("IT-HIGH".into(), "High Value Engine".into(), None, "ea".into(), false, false, "5".into(), None).expect("it1");
        let it2 = engine.create_item("IT-LOW".into(), "Low Value Washer".into(), None, "ea".into(), false, false, "100".into(), None).expect("it2");
        engine.move_stock(it1.id.clone(), None, Some(loc1.id.clone()), "10".into(), "Receive".into(), None, None).expect("move it1");
        engine.move_stock(it2.id.clone(), None, Some(loc1.id.clone()), "500".into(), "Receive".into(), None, None).expect("move it2");

        let abc = engine.run_abc_analysis().expect("abc analysis");
        assert!(abc.items.len() >= 2);
        assert_eq!(abc.items[0].tier, "A");

        let heatmap = engine.get_inventory_aging_heatmap().expect("aging heatmap");
        assert!(!heatmap.is_empty());

        let eoq = engine.calculate_eoq_and_safety_stock(it1.id.clone(), 60.0, 50.0, 0.2, 250.0, 7.0, 3.0).expect("calc eoq");
        assert!(eoq.economic_order_quantity > 0.0);
        assert!(eoq.safety_stock > 0.0);

        // 6. LEDGER-21: Database Encryption at Rest & Authenticated P2P Sync
        let sec_status = engine.get_security_status().expect("security status");
        assert!(sec_status.is_encrypted);

        let qr_payload = engine.generate_pairing_qr().expect("gen qr pair");
        assert!(qr_payload.qr_uri.starts_with("erp://pair?"));
        let parsed_pair = engine.parse_pairing_qr(qr_payload.qr_uri.clone()).expect("parse qr pair");
        assert_eq!(parsed_pair.node_id, qr_payload.node_id);

        let challenge = "auth_handshake_challenge_12345";
        let valid_token = engine.security.compute_handshake_token(&qr_payload.node_id, challenge).expect("token");
        let is_valid = engine.verify_peer_handshake(qr_payload.node_id.clone(), challenge.into(), valid_token).expect("verify");
        assert!(is_valid);

        // 7. LEDGER-22: On-Device Offline OCR Intake for Receipts & Vendor Invoices
        let ocr_sample = "Vendor: Bavarian Machine Tools GmbH\nDate: 2026-10-31\nPrecision End Mill 4 x 30.00\nTotal: 120.00 EUR";
        let ocr_res = engine.process_ocr_receipt_text(ocr_sample.into()).expect("process ocr");
        assert_eq!(ocr_res.merchant_name.as_deref(), Some("Bavarian Machine Tools GmbH"));

        let supplier = engine.create_contact("Supplier".into(), "Bavarian Machine Tools GmbH".into(), None, None, None, None).expect("supplier");
        let po_from_ocr = engine.convert_ocr_to_draft_po(ocr_res, supplier.name.clone(), loc1.id.clone()).expect("ocr to po");
        assert_eq!(po_from_ocr.supplier_id, supplier.name);

        // 8. LEDGER-23: Serial Number Tracking, Warranty Passport & Maintenance Logs
        let serialized_item = engine.create_item("ROBOT-ARM-6AXIS".into(), "6-Axis Articulated Robot Arm".into(), None, "unit".into(), true, false, "1".into(), None).expect("ser item");
        let passport = engine.register_serialized_asset("SN-ROBOT-9901".into(), serialized_item.id.clone(), Some("2029-10-31".into()), Some(loc1.id.clone())).expect("reg asset");
        assert_eq!(passport.serial_number, "SN-ROBOT-9901");
        assert_eq!(passport.qr_code_uri, "erp://serial/SN-ROBOT-9901");

        let fetched_pass = engine.get_asset_passport("SN-ROBOT-9901".into()).expect("get pass");
        assert_eq!(fetched_pass.id, passport.id);

        let maint_log = engine.add_maintenance_log(passport.id.clone(), "Calibration".into(), "Optical joint encoder calibration".into(), "lead_engineer".into(), Some("2027-10-31".into())).expect("maint log");
        assert_eq!(maint_log.maintenance_type, "Calibration");

        let logs = engine.list_maintenance_logs(passport.id.clone()).expect("list logs");
        assert_eq!(logs.len(), 1);

        let rma = engine.create_rma_claim(passport.id.clone(), cust.id.clone(), "Joint 3 harmonic drive backlash".into()).expect("rma claim");
        assert_eq!(rma.status, "Open");
        let rmas = engine.list_rma_claims().expect("list rmas");
        assert_eq!(rmas.len(), 1);

        // 9. LEDGER-24: Offline POS Cashier Mode, Cash Drawer Kick & Shift Reconciliation
        let shift = engine.open_pos_shift("REG-01".into(), "cashier_sarah".into(), "200.00".into()).expect("open shift");
        assert_eq!(shift.status, "Open");
        assert_eq!(shift.starting_cash, "200.00");

        let shift_after_sale = engine.record_pos_sale(shift.id.clone(), "75.50".into(), "cash".into()).expect("pos sale");
        assert_eq!(shift_after_sale.cash_sales, "75.50");

        let shift_closed = engine.close_pos_shift(shift.id.clone(), "275.50".into()).expect("close shift");
        assert_eq!(shift_closed.status, "Closed");
        assert_eq!(shift_closed.variance, Some("0.00".into()));

        let z_report = engine.generate_shift_report(shift.id.clone(), "Z".into()).expect("z report");
        assert!(z_report.content.contains("FINAL SHIFT Z-REPORT"));

        // 10. LEDGER-25: Multi-Warehouse Transfer Orders & In-Transit Tracking
        let loc2 = engine.create_location("WH-BRANCH".into(), "Branch Facility".into(), None, "Warehouse".into(), None, None).expect("loc2");
        let xfer = engine.create_transfer_order(
            loc1.id.clone(),
            loc2.id.clone(),
            vec![StockTransferLineDto {
                id: "".into(),
                item_id: it1.id.clone(),
                sku: it1.sku.clone(),
                item_name: it1.name.clone(),
                requested_qty: "4".into(),
                shipped_qty: "0".into(),
                received_qty: "0".into(),
            }],
        ).expect("create xfer");
        assert_eq!(xfer.status, "Draft");

        let xfer_disp = engine.dispatch_transfer_order(xfer.id.clone()).expect("disp xfer");
        assert_eq!(xfer_disp.status, "In-Transit");

        let xfer_rec = engine.receive_transfer_order(xfer.id.clone(), vec!["4".into()]).expect("rec xfer");
        assert_eq!(xfer_rec.status, "Completed");
        assert_eq!(xfer_rec.lines[0].received_qty, "4");

        let all_xfers = engine.list_transfer_orders().expect("list xfers");
        assert!(!all_xfers.is_empty());

        // 11. LEDGER-26: Workstation Capacity Planning & Gantt Production Scheduling
        let ws = engine.create_workstation("WS-CNC-01".into(), "5-Axis CNC Milling Center".into(), 16.0, 85.0).expect("ws");
        assert_eq!(ws.code, "WS-CNC-01");

        let bom = engine.create_bom(
            "BOM-ROBOT-01".into(),
            it1.id.clone(),
            "Engine BOM".into(),
            "1".into(),
            vec![BomComponentDto {
                id: "".into(),
                item_id: it2.id.clone(),
                quantity: "10".into(),
                scrap_percentage: "0".into(),
                notes: None,
            }],
            None,
        ).expect("bom");
        let ro = engine.add_routing_operation(bom.id.clone(), ws.id.clone(), "Precision Milling Block".into(), 1, 45).expect("ro");
        assert_eq!(ro.operation_name, "Precision Milling Block");

        let wo = engine.create_work_order("WO-ROBOT-01".into(), bom.id.clone(), "10".into(), loc1.id.clone(), loc2.id.clone()).expect("wo");
        let alloc = engine.schedule_work_order(wo.id.clone(), ws.id.clone(), ro.id.clone(), "2026-11-02T08:00:00Z".into(), 120).expect("alloc");
        assert_eq!(alloc.allocated_minutes, 120);

        let cap_summary = engine.get_workstation_capacity_schedule(ws.id.clone(), "2026-11-02".into()).expect("cap sched");
        assert_eq!(cap_summary.workstation_code, "WS-CNC-01");

        // 12. LEDGER-27: Automated NAS/SFTP Backup Scheduler & USB Drive Sync
        let backup_cfg = engine.configure_backup_policy("/tmp/ledger_test_backups".into(), 5, 12, true).expect("config backup");
        assert!(backup_cfg.auto_backup_enabled);
        assert_eq!(backup_cfg.retention_count, 5);

        let backup_archive = engine.trigger_automated_backup().expect("trigger backup");
        assert!(backup_archive.is_verified);
        assert!(backup_archive.size_bytes > 0);

        let archives = engine.list_backup_archives().expect("list backups");
        assert!(!archives.is_empty());

        let is_ok = engine.verify_backup_file(backup_archive.filepath.clone()).expect("verify backup file");
        assert!(is_ok);

        let _usbs = engine.detect_usb_drives().expect("detect usbs");

        // 13. LEDGER-28: European E-Invoicing Compliance (Factur-X / ZUGFeRD 2.2 / UBL 2.1)
        let invoice = engine.create_invoice(
            "INV-2026-EUR-01".into(),
            cust.id.clone(),
            vec![InvoiceLineInputDto {
                item_id: Some(it1.id.clone()),
                description: "Precision Engineered Valve".into(),
                quantity: "2".into(),
                unit_price: "150.00".into(),
                tax_rate: "0.19".into(),
            }],
            Some("Net 30 Payment Terms".into()),
        ).expect("create invoice");

        let factur_xml = engine.generate_factur_x_xml(invoice.id.clone()).expect("generate factur-x xml");
        assert!(factur_xml.contains("<rsm:CrossIndustryInvoice"));
        assert!(factur_xml.contains("INV-2026-EUR-01"));

        let val_report = engine.validate_einvoice_en16931(factur_xml.clone()).expect("validate einvoice");
        assert!(val_report.is_valid);
        assert_eq!(val_report.errors.len(), 0);

        let hybrid_pdf = engine.generate_hybrid_pdf_a3_invoice(invoice.id.clone()).expect("hybrid pdf");
        assert!(hybrid_pdf.starts_with(b"%PDF-1.4"));
        assert!(hybrid_pdf.len() > 500);

        let incoming_parsed = engine.parse_incoming_einvoice_xml(factur_xml.clone()).expect("parse incoming");
        assert_eq!(incoming_parsed.invoice_number, "INV-2026-EUR-01");

        let einvoice_po = engine.convert_einvoice_to_draft_po(factur_xml, "Industrial Parts Vendor".into(), loc1.id.clone()).expect("einvoice to po");
        assert_eq!(einvoice_po.supplier_id, "Industrial Parts Vendor");

        println!(">>> ALL 13 V0.0.4 FEATURES TESTED & VERIFIED OK!");
    }
}
