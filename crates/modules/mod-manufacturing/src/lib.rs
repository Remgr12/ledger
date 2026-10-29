pub mod errors;
pub mod models;
pub mod module;
pub mod service;

pub use errors::ManufacturingError;
pub use models::{BillOfMaterials, BomComponent, WorkOrder, WorkOrderStatus};
pub use module::ManufacturingModule;
pub use service::ManufacturingService;

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use mod_inventory::{InventoryModule, LocationService, LocationType, MovementType, StockService};
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn setup_test_ctx() -> (Database, erp_kernel::ModuleContext, Arc<ModuleManager>) {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("node-mfg-test".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc, bus));
        mm.register(Arc::new(InventoryModule::new()));
        mm.register(Arc::new(ManufacturingModule::new()));
        mm.initialize().expect("init mm");

        mm.enable_module("mod-inventory").expect("enable inventory");
        mm.enable_module("mod-manufacturing").expect("enable manufacturing");

        let ctx = mm.context();
        (db, ctx, mm)
    }

    #[test]
    fn test_bom_creation_and_work_order_execution() {
        let (db, ctx, _) = setup_test_ctx();

        // 1. Storage locations: Source bin for parts, Destination bin for finished goods
        let room = LocationService::create_location(
            &ctx,
            "WS-01",
            "Production Workshop",
            None,
            LocationType::Room,
            None,
            None,
        )
        .unwrap();

        let source_bin = LocationService::create_location(
            &ctx,
            "BIN-PARTS",
            "Raw Parts Bin",
            Some(&room.id),
            LocationType::Bin,
            Some(10.0),
            Some(10.0),
        )
        .unwrap();

        let dest_bin = LocationService::create_location(
            &ctx,
            "BIN-FINISHED",
            "Finished Goods Bay",
            Some(&room.id),
            LocationType::Bin,
            Some(20.0),
            Some(20.0),
        )
        .unwrap();

        // 2. Catalog Items: Raw Chassis, Stepper Motor, and Finished Robot Arm
        let chassis = StockService::create_item(
            &ctx,
            "ALU-CHASSIS-01",
            "Aluminum CNC Chassis",
            None,
            "pcs",
            false,
            false,
            Decimal::ZERO,
            None,
        )
        .unwrap();

        let motor = StockService::create_item(
            &ctx,
            "STEPPER-NEMA17",
            "NEMA 17 Stepper Motor",
            None,
            "pcs",
            false,
            false,
            Decimal::ZERO,
            None,
        )
        .unwrap();

        let robot_arm = StockService::create_item(
            &ctx,
            "ROBOT-ARM-4AXIS",
            "4-Axis Desktop Robotic Arm",
            None,
            "unit",
            false,
            false,
            Decimal::ZERO,
            None,
        )
        .unwrap();

        // 3. Stock raw materials into source_bin (Receive 10 chassis, 20 motors)
        StockService::record_movement(
            &ctx,
            MovementType::Receive,
            &chassis.id,
            None,
            Some(&source_bin.id),
            Decimal::from(10),
            None,
            None,
            None,
            None,
            "test",
        )
        .unwrap();

        StockService::record_movement(
            &ctx,
            MovementType::Receive,
            &motor.id,
            None,
            Some(&source_bin.id),
            Decimal::from(20),
            None,
            None,
            None,
            None,
            "test",
        )
        .unwrap();

        // 4. Create BOM: 1 Robot Arm requires 1 Chassis + 3 Motors
        let bom = ManufacturingService::create_bom(
            &ctx,
            "BOM-ROBOT-01",
            &robot_arm.id,
            "Standard Robot Arm Assembly",
            Decimal::ONE,
            vec![
                (chassis.id.clone(), Decimal::ONE, Decimal::ZERO, None),
                (motor.id.clone(), Decimal::from(3), Decimal::ZERO, None),
            ],
            Some("Assemble base -> attach motors -> calibrate"),
        )
        .expect("create bom");

        assert_eq!(bom.components.len(), 2);

        // 5. Create Work Order: Produce 4 Robot Arms
        // Requires: 4 * 1 = 4 Chassis, 4 * 3 = 12 Motors
        let wo = ManufacturingService::create_work_order(
            &ctx,
            "WO-2026-001",
            &bom.id,
            Decimal::from(4),
            &source_bin.id,
            &dest_bin.id,
        )
        .expect("create wo");

        assert_eq!(wo.status, WorkOrderStatus::Draft);

        // 6. Execute Work Order
        let completed = ManufacturingService::execute_work_order(&ctx, &wo.id).expect("execute wo");
        assert_eq!(completed.status, WorkOrderStatus::Completed);
        assert!(completed.completed_at.is_some());

        // 7. Verify inventory deductions and additions
        // Chassis at source bin: 10 - 4 = 6
        let chassis_stock = StockService::locate_item(&db.conn(), &chassis.sku).unwrap();
        assert_eq!(chassis_stock[0].quantity, Decimal::from(6));

        // Motors at source bin: 20 - 12 = 8
        let motor_stock = StockService::locate_item(&db.conn(), &motor.sku).unwrap();
        assert_eq!(motor_stock[0].quantity, Decimal::from(8));

        // Finished Robot Arms at dest bin: 4
        let arm_stock = StockService::locate_item(&db.conn(), &robot_arm.sku).unwrap();
        assert_eq!(arm_stock.len(), 1);
        assert_eq!(arm_stock[0].location_code, "BIN-FINISHED");
        assert_eq!(arm_stock[0].quantity, Decimal::from(4));
    }
}
