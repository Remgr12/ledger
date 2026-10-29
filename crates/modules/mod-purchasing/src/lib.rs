pub mod errors;
pub mod models;
pub mod module;
pub mod service;

pub use errors::PurchasingError;
pub use models::{
    PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus, PurchaseReceipt, PurchaseReceiptLine,
};
pub use module::PurchasingModule;
pub use service::PurchasingService;

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use mod_contacts::{ContactType, ContactsModule, ContactsService};
    use mod_inventory::{InventoryModule, LocationService, LocationType, StockService};
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn setup_test_ctx() -> (Database, erp_kernel::ModuleContext, Arc<ModuleManager>) {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("node-purchasing-test".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc, bus));
        mm.register(Arc::new(ContactsModule::new()));
        mm.register(Arc::new(InventoryModule::new()));
        mm.register(Arc::new(PurchasingModule::new()));
        mm.initialize().expect("init mm");

        mm.enable_module("mod-contacts").expect("enable contacts");
        mm.enable_module("mod-inventory").expect("enable inventory");
        mm.enable_module("mod-purchasing").expect("enable purchasing");

        let ctx = mm.context();
        (db, ctx, mm)
    }

    #[test]
    fn test_purchase_order_lifecycle_and_goods_receipt() {
        let (db, ctx, _) = setup_test_ctx();

        // 1. Create Supplier Contact
        let supplier = ContactsService::create_contact(
            &ctx,
            ContactType::Supplier,
            "Industrial Fasteners GmbH",
            Some("orders@fasteners.de"),
            None,
            Some("DE-VAT-123456"),
            Some("Munich, Germany"),
        )
        .expect("create supplier");

        // 2. Create Destination Warehouse Bin in mod-inventory
        let warehouse = LocationService::create_location(
            &ctx,
            "WH-CENTRAL",
            "Central Warehouse",
            None,
            LocationType::Warehouse,
            None,
            None,
        )
        .expect("create wh");

        let bin = LocationService::create_location(
            &ctx,
            "BIN-P1",
            "Receiving Bin P1",
            Some(&warehouse.id),
            LocationType::Bin,
            Some(10.0),
            Some(20.0),
        )
        .expect("create bin");

        // 3. Create Inventory Item
        let item = StockService::create_item(
            &ctx,
            "M6-HEX-NUT",
            "M6 Stainless Steel Hex Nut",
            Some("Box of 500"),
            "box",
            false,
            false,
            Decimal::from(10),
            None,
        )
        .expect("create item");

        // 4. Create Purchase Order: 20 boxes @ 15.00 each, 19% VAT
        let price = Decimal::from(15);
        let tax = Decimal::from_str_radix("0.19", 10).unwrap_or(Decimal::ZERO);
        let po = PurchasingService::create_purchase_order(
            &ctx,
            "PO-2026-001",
            &supplier.id,
            Some("2026-11-15"),
            vec![(
                item.id.clone(),
                "M6 Hex Nuts".into(),
                Decimal::from(20),
                price,
                tax,
            )],
            Some("Urgent restock"),
        )
        .expect("create po");

        assert_eq!(po.status, PurchaseOrderStatus::Draft);
        assert_eq!(po.subtotal, Decimal::from(300)); // 20 * 15
        assert_eq!(po.tax_total, Decimal::from(57));  // 300 * 0.19
        assert_eq!(po.grand_total, Decimal::from(357));

        // Submit PO
        let submitted = PurchasingService::submit_purchase_order(&ctx, &po.id).expect("submit po");
        assert_eq!(submitted.status, PurchaseOrderStatus::Submitted);

        // 5. Receive Goods (all 20 boxes) directly into Receiving Bin P1
        let receipt = PurchasingService::receive_goods(
            &ctx,
            Some(&po.id),
            &supplier.id,
            &bin.id,
            vec![(item.id.clone(), Decimal::from(20), Some("LOT-FAST-26".into()), None)],
            Some("Inspected and verified intact"),
        )
        .expect("receive goods");

        assert_eq!(receipt.lines.len(), 1);
        assert_eq!(receipt.lines[0].quantity, Decimal::from(20));

        // Verify PO status is now Received!
        let po_after = PurchasingService::get_purchase_order_by_id(&db.conn(), &po.id).unwrap();
        assert_eq!(po_after.status, PurchaseOrderStatus::Received);
        assert_eq!(po_after.lines[0].received_quantity, Decimal::from(20));

        // Verify stock in mod-inventory at BIN-P1 is now 20!
        let located = StockService::locate_item(&db.conn(), &item.sku).expect("locate stock");
        assert_eq!(located.len(), 1);
        assert_eq!(located[0].location_code, "BIN-P1");
        assert_eq!(located[0].quantity, Decimal::from(20));
        assert_eq!(located[0].lot_number.as_deref(), Some("LOT-FAST-26"));
    }
}
