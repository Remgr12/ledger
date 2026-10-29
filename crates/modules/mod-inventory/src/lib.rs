pub mod errors;
pub mod location_service;
pub mod migrations;
pub mod models;
pub mod module;
pub mod qr_service;
pub mod stock_service;

pub use errors::InventoryError;
pub use location_service::LocationService;
pub use models::*;
pub use module::InventoryModule;
pub use qr_service::QrService;
pub use stock_service::StockService;

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleContext, ModuleManager};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;

    fn setup_test_context() -> (ModuleContext, Arc<ModuleManager>) {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("test-node".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc.clone(), bus.clone()));
        mm.register(Arc::new(InventoryModule::new()));
        mm.initialize().expect("init mm");
        mm.enable_module("mod-inventory").expect("enable mod-inventory");

        (ModuleContext::new(db, hlc, bus), mm)
    }

    #[test]
    fn test_location_tree_breadcrumbs_and_closure() {
        let (ctx, _mm) = setup_test_context();

        // 1. Warehouse
        let wh = LocationService::create_location(
            &ctx,
            "WH-01",
            "Main Warehouse",
            None,
            LocationType::Warehouse,
            Some(10.0),
            Some(20.0),
        )
        .expect("create wh");
        assert_eq!(wh.path_breadcrumb, "Main Warehouse");

        // 2. Room
        let room = LocationService::create_location(
            &ctx,
            "RM-A",
            "Workshop",
            Some(&wh.id),
            LocationType::Room,
            Some(12.0),
            Some(22.0),
        )
        .expect("create room");
        assert_eq!(room.path_breadcrumb, "Main Warehouse > Workshop");

        // 3. Rack
        let rack = LocationService::create_location(
            &ctx,
            "RCK-02",
            "Rack 2",
            Some(&room.id),
            LocationType::Rack,
            Some(14.0),
            Some(24.0),
        )
        .expect("create rack");
        assert_eq!(rack.path_breadcrumb, "Main Warehouse > Workshop > Rack 2");

        // 4. Shelf
        let shelf = LocationService::create_location(
            &ctx,
            "SH-C",
            "Shelf C",
            Some(&rack.id),
            LocationType::Shelf,
            Some(14.0),
            Some(25.0),
        )
        .expect("create shelf");
        assert_eq!(
            shelf.path_breadcrumb,
            "Main Warehouse > Workshop > Rack 2 > Shelf C"
        );

        // 5. Bin
        let bin = LocationService::create_location(
            &ctx,
            "BIN-04",
            "Blue Bin #4",
            Some(&shelf.id),
            LocationType::Bin,
            Some(14.5),
            Some(25.5),
        )
        .expect("create bin");
        assert_eq!(
            bin.path_breadcrumb,
            "Main Warehouse > Workshop > Rack 2 > Shelf C > Blue Bin #4"
        );

        // Verify tree retrieval
        let tree = LocationService::get_location_tree(&ctx.db.conn()).expect("get tree");
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children.len(), 1);

        // 6. Test move_location: move Shelf C (and its child Blue Bin #4) under Room directly
        let moved_shelf = LocationService::move_location(&ctx, &shelf.id, Some(&room.id))
            .expect("move shelf");
        assert_eq!(moved_shelf.path_breadcrumb, "Main Warehouse > Workshop > Shelf C");

        // Verify child bin breadcrumb was recursively updated!
        let updated_bin = LocationService::get_location_by_id(&ctx.db.conn(), &bin.id)
            .expect("get updated bin");
        assert_eq!(
            updated_bin.path_breadcrumb,
            "Main Warehouse > Workshop > Shelf C > Blue Bin #4"
        );
    }

    #[test]
    fn test_gs1_parsing_and_cycle_count_audit() {
        let (ctx, _mm) = setup_test_context();

        let bin = LocationService::create_location(
            &ctx,
            "BIN-AUDIT",
            "Audit Bin",
            None,
            LocationType::Bin,
            None,
            None,
        )
        .expect("create bin");

        let item = StockService::create_item(
            &ctx,
            "07350012345678",
            "GS1 Tracked Component",
            None,
            "pcs",
            true,
            true,
            Decimal::from_str("5.0").unwrap(),
            Some("07350012345678"),
        )
        .expect("create item");

        // Initial receipt: 20 pcs
        StockService::record_movement(
            &ctx,
            MovementType::Receive,
            &item.id,
            None,
            Some(&bin.id),
            Decimal::from(20),
            Some("LOT-2026A"),
            None,
            None,
            None,
            "auditor",
        )
        .expect("receive");

        // Test GS1 barcode parser
        let gs1_str = "(01)07350012345678(10)LOT-2026A(21)SN9988";
        let parsed = QrService::parse_qr(gs1_str);
        match parsed {
            ParsedQr::Gs1(data) => {
                assert_eq!(data.gtin.as_deref(), Some("07350012345678"));
                assert_eq!(data.lot_number.as_deref(), Some("LOT-2026A"));
                assert_eq!(data.serial_number.as_deref(), Some("SN9988"));
            }
            _ => panic!("Expected ParsedQr::Gs1"),
        }

        // Test Cycle Count Audit: physical count is 23 (3 surplus units found)
        let adj = StockService::reconcile_audit(
            &ctx,
            &bin.id,
            &item.id,
            Decimal::from(23),
            Some("LOT-2026A"),
            None,
            "auditor",
        )
        .expect("audit reconciliation")
        .expect("surplus adjustment created");

        assert_eq!(adj.movement_type, MovementType::Adjust);
        assert_eq!(adj.quantity, Decimal::from(3));

        // Check new total stock
        let total = StockService::get_total_stock(&ctx.db.conn(), &item.id).expect("total stock");
        assert_eq!(total, Decimal::from(23));

        // Test Multi-Format Label Generation
        let thermal_svg = QrService::generate_label_svg(
            LabelFormat::Thermal4x6,
            "erp://item/07350012345678",
            "07350012345678",
            "GS1 Tracked Component",
            Some("Lot: LOT-2026A"),
        )
        .expect("generate 4x6");
        assert!(thermal_svg.contains("1D Vector Barcode"));
        assert!(thermal_svg.contains("07350012345678"));

        let a4_svg = QrService::generate_label_svg(
            LabelFormat::A4StickerSheet,
            "erp://item/07350012345678",
            "07350012345678",
            "GS1 Component",
            None,
        )
        .expect("generate a4");
        assert!(a4_svg.contains("800 1100"));
    }

    #[test]
    fn test_stock_ledger_and_where_is_it_locator() {
        let (ctx, _mm) = setup_test_context();

        let loc = LocationService::create_location(
            &ctx,
            "BIN-01",
            "Bin 1",
            None,
            LocationType::Bin,
            Some(5.0),
            Some(5.0),
        )
        .expect("create loc");

        let item = StockService::create_item(
            &ctx,
            "SKU-BOLT-M8",
            "M8 Stainless Bolt",
            Some("Hex bolt"),
            "pcs",
            false,
            true,
            Decimal::from_str("100.0").unwrap(),
            Some("735001234567"),
        )
        .expect("create item");

        // 1. Initial Receive
        let receive_qty = Decimal::from_str("250.0").unwrap();
        StockService::record_movement(
            &ctx,
            MovementType::Receive,
            &item.id,
            None,
            Some(&loc.id),
            receive_qty,
            Some("LOT-2026A"),
            None,
            Some("PO-9001"),
            Some("Vendor delivery"),
            "user-admin",
        )
        .expect("receive stock");

        // Check stock total
        let total = StockService::get_total_stock(&ctx.db.conn(), &item.id).expect("total stock");
        assert_eq!(total, Decimal::from_str("250.0").unwrap());

        // 2. Locate Item "Where is it?"
        let matches = StockService::locate_item(&ctx.db.conn(), "BOLT-M8").expect("locate item");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].sku, "SKU-BOLT-M8");
        assert_eq!(matches[0].location_name, "Bin 1");
        assert_eq!(matches[0].quantity, Decimal::from_str("250.0").unwrap());
        assert_eq!(matches[0].lot_number.as_deref(), Some("LOT-2026A"));

        // 3. Pick stock
        let pick_qty = Decimal::from_str("50.0").unwrap();
        StockService::record_movement(
            &ctx,
            MovementType::Pick,
            &item.id,
            Some(&loc.id),
            None,
            pick_qty,
            Some("LOT-2026A"),
            None,
            Some("SO-101"),
            Some("Customer shipment"),
            "user-admin",
        )
        .expect("pick stock");

        let total_after_pick =
            StockService::get_total_stock(&ctx.db.conn(), &item.id).expect("total stock");
        assert_eq!(total_after_pick, Decimal::from_str("200.0").unwrap());

        // 4. Test Insufficient stock error
        let excessive_qty = Decimal::from_str("999.0").unwrap();
        let err = StockService::record_movement(
            &ctx,
            MovementType::Pick,
            &item.id,
            Some(&loc.id),
            None,
            excessive_qty,
            Some("LOT-2026A"),
            None,
            None,
            None,
            "user-admin",
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_qr_resolution_and_svg_generation() {
        let (ctx, _mm) = setup_test_context();

        let loc = LocationService::create_location(
            &ctx,
            "SHELF-A1",
            "Shelf A1",
            None,
            LocationType::Shelf,
            None,
            None,
        )
        .expect("create loc");

        let item = StockService::create_item(
            &ctx,
            "WIDGET-PRO",
            "Pro Industrial Widget",
            None,
            "pcs",
            false,
            false,
            Decimal::from_str("10").unwrap(),
            None,
        )
        .expect("create item");

        // 1. Scan Location QR
        let res_loc = QrService::resolve_qr_scan(
            &ctx.db.conn(),
            &format!("erp://loc/{}", loc.id),
            None,
        )
        .expect("resolve loc");
        match res_loc {
            ScanResolution::LocationContext { location, .. } => {
                assert_eq!(location.id, loc.id);
            }
            _ => panic!("Expected LocationContext"),
        }

        // 2. Scan Item QR without context -> ItemLookup
        let res_item =
            QrService::resolve_qr_scan(&ctx.db.conn(), "erp://item/WIDGET-PRO", None)
                .expect("resolve item");
        match res_item {
            ScanResolution::ItemLookup { item: found, .. } => {
                assert_eq!(found.sku, "WIDGET-PRO");
            }
            _ => panic!("Expected ItemLookup"),
        }

        // 3. Scan Item QR WITH active location context -> RapidStockAction
        let res_action = QrService::resolve_qr_scan(
            &ctx.db.conn(),
            "erp://item/WIDGET-PRO",
            Some(&loc.id),
        )
        .expect("resolve rapid action");
        match res_action {
            ScanResolution::RapidStockAction {
                item: i,
                target_location: l,
                ..
            } => {
                assert_eq!(i.sku, "WIDGET-PRO");
                assert_eq!(l.id, loc.id);
            }
            _ => panic!("Expected RapidStockAction"),
        }

        // 4. Generate SVG Label
        let svg = QrService::generate_qr_label_svg(
            &item.qr_code_uri,
            &item.sku,
            &item.name,
            Some("Unit: pcs | Min: 10"),
        )
        .expect("generate svg");

        assert!(svg.contains("<svg"));
        assert!(svg.contains("WIDGET-PRO"));
        assert!(svg.contains("erp://item/WIDGET-PRO"));
        assert!(svg.contains("</svg>"));
    }
}
