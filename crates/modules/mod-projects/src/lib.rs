pub mod errors;
pub mod models;
pub mod module;
pub mod service;

pub use errors::ProjectsError;
pub use models::{Project, ProjectStatus, ProjectTask, TimesheetEntry};
pub use module::ProjectsModule;
pub use service::ProjectsService;

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use mod_contacts::{ContactType, ContactsModule, ContactsService};
    use mod_invoicing::InvoicingModule;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn setup_test_ctx() -> (Database, erp_kernel::ModuleContext, Arc<ModuleManager>) {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("node-projects-test".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc, bus));
        mm.register(Arc::new(ContactsModule::new()));
        mm.register(Arc::new(InvoicingModule::new()));
        mm.register(Arc::new(ProjectsModule::new()));
        mm.initialize().expect("init mm");

        mm.enable_module("mod-contacts").expect("enable contacts");
        mm.enable_module("mod-invoicing").expect("enable invoicing");
        mm.enable_module("mod-projects").expect("enable projects");

        let ctx = mm.context();
        (db, ctx, mm)
    }

    #[test]
    fn test_project_timesheet_and_automated_invoicing() {
        let (db, ctx, _) = setup_test_ctx();

        // 1. Create Client / Customer
        let client = ContactsService::create_contact(
            &ctx,
            ContactType::Customer,
            "Apex Tech Innovations",
            Some("accounts@apextech.io"),
            None,
            Some("US-EIN-5551234"),
            Some("Austin, TX"),
        )
        .expect("create client");

        // 2. Create Project ($150/hr standard billing rate)
        let rate = Decimal::from(150);
        let project = ProjectsService::create_project(
            &ctx,
            "Mobile App Revamp",
            "PRJ-APEX-01",
            Some(&client.id),
            rate,
            Decimal::from(40),
        )
        .expect("create project");

        // 3. Create Task
        let task = ProjectsService::create_task(
            &ctx,
            &project.id,
            "UniFFI Bridge Architecture",
            "High",
            Decimal::from(10),
        )
        .expect("create task");

        // 4. Log 2 Timesheet entries (3.5 hours + 4.5 hours = 8.0 hours @ $150 = $1200)
        let ts1 = ProjectsService::record_timesheet(
            &ctx,
            &project.id,
            Some(&task.id),
            "lead_architect",
            "2026-10-28",
            Decimal::from_str_radix("3.5", 10).unwrap(),
            None,
            true,
            "Designed Kotlin-to-Rust memory mapping layer",
        )
        .expect("record ts1");
        assert_eq!(ts1.billing_amount, Decimal::from_str_radix("525.0", 10).unwrap());

        let ts2 = ProjectsService::record_timesheet(
            &ctx,
            &project.id,
            Some(&task.id),
            "lead_architect",
            "2026-10-29",
            Decimal::from_str_radix("4.5", 10).unwrap(),
            None,
            true,
            "Implemented cross-platform Compose HUD",
        )
        .expect("record ts2");
        assert_eq!(ts2.billing_amount, Decimal::from_str_radix("675.0", 10).unwrap());

        // 5. Convert Unbilled Timesheets to Sales Invoice
        let invoice = ProjectsService::invoice_unbilled_timesheets(
            &ctx,
            &project.id,
            "INV-APEX-001",
            Decimal::ZERO,
        )
        .expect("invoice timesheets");

        assert_eq!(invoice.subtotal, Decimal::from(1200));
        assert_eq!(invoice.line_items.len(), 2);
        assert_eq!(invoice.customer_id, client.id);

        // 6. Verify timesheets are marked as invoiced
        let unbilled_after = ProjectsService::list_unbilled_timesheets(&db.conn(), &project.id)
            .expect("list unbilled");
        assert_eq!(unbilled_after.len(), 0);
    }
}
