pub mod models;
pub mod service;

pub use models::{
    EInvoiceParty, EInvoiceValidationReport, Invoice, InvoiceLineItem, InvoiceStatus,
    RecurringSchedule,
};
pub use service::InvoicingService;

use erp_kernel::{ErpModule, Migration, ModuleRoute};

pub struct InvoicingModule;

impl Default for InvoicingModule {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoicingModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for InvoicingModule {
    fn id(&self) -> &'static str {
        "mod-invoicing"
    }

    fn name(&self) -> &'static str {
        "Invoicing & Billing"
    }

    fn description(&self) -> &'static str {
        "Quotations, customer invoices, tax computation, and payment tracking"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["mod-contacts"]
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_invoicing_tables",
            sql: r#"
            CREATE TABLE IF NOT EXISTS invoices (
                id TEXT PRIMARY KEY,
                invoice_number TEXT NOT NULL UNIQUE,
                customer_id TEXT NOT NULL REFERENCES contacts(id),
                status TEXT NOT NULL,
                subtotal TEXT NOT NULL,
                tax_total TEXT NOT NULL,
                grand_total TEXT NOT NULL,
                notes TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS invoice_lines (
                id TEXT PRIMARY KEY,
                invoice_id TEXT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
                item_id TEXT,
                description TEXT NOT NULL,
                quantity TEXT NOT NULL,
                unit_price TEXT NOT NULL,
                tax_rate TEXT NOT NULL,
                line_total TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_invoices_customer ON invoices(customer_id);
            CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
            CREATE INDEX IF NOT EXISTS idx_lines_invoice ON invoice_lines(invoice_id);
            "#,
        },
        Migration {
            version: 2,
            name: "create_recurring_schedules_table",
            sql: r#"
            CREATE TABLE IF NOT EXISTS recurring_schedules (
                id TEXT PRIMARY KEY,
                customer_id TEXT NOT NULL REFERENCES contacts(id),
                plan_name TEXT NOT NULL,
                frequency TEXT NOT NULL,
                amount TEXT NOT NULL,
                currency TEXT NOT NULL DEFAULT 'EUR',
                next_run_date TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                auto_send INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_recurring_cust ON recurring_schedules(customer_id);
            "#,
        }]
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "invoicing/invoices".to_string(),
                title: "Invoices".to_string(),
                icon: "receipt_long".to_string(),
                display_order: 30,
            },
            ModuleRoute {
                route_id: "invoicing/create".to_string(),
                title: "New Invoice".to_string(),
                icon: "post_add".to_string(),
                display_order: 31,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use mod_contacts::{ContactType, ContactsModule, ContactsService};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;

    #[test]
    fn test_invoice_creation_and_precision() {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("test-node".into()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc.clone(), bus.clone()));
        mm.register(Arc::new(ContactsModule::new()));
        mm.register(Arc::new(InvoicingModule::new()));
        mm.initialize().expect("init mm");

        mm.enable_module("mod-contacts").expect("enable contacts");
        mm.enable_module("mod-invoicing").expect("enable invoicing");

        let ctx = mm.context();
        let customer = ContactsService::create_contact(
            &ctx,
            ContactType::Customer,
            "Acme Corp",
            Some("billing@acme.com"),
            None,
            Some("US12345678"),
            Some("123 Industrial Pkwy"),
        )
        .expect("create contact");

        // 3 items @ 19.99 with 20% tax = 59.97 + 11.994 = 71.964
        let qty = Decimal::from_str("3.0").unwrap();
        let price = Decimal::from_str("19.99").unwrap();
        let tax = Decimal::from_str("0.20").unwrap();

        let inv = InvoicingService::create_invoice(
            &ctx,
            "INV-2026-001",
            &customer.id,
            vec![(None, "Standard Widget".into(), qty, price, tax)],
            Some("Net 30 terms"),
        )
        .expect("create invoice");

        assert_eq!(inv.subtotal, Decimal::from_str("59.97").unwrap());
        assert_eq!(inv.tax_total, Decimal::from_str("11.994").unwrap());
        assert_eq!(inv.grand_total, Decimal::from_str("71.964").unwrap());
        assert_eq!(inv.status, InvoiceStatus::Draft);

        // Finalize invoice
        let finalized =
            InvoicingService::finalize_invoice(&ctx, &inv.id, "accountant").expect("finalize");
        assert_eq!(finalized.status, InvoiceStatus::Finalized);

        // 1. Recurring schedules (LEDGER-19)
        let sched = InvoicingService::create_recurring_schedule(
            &ctx.db.conn(),
            &customer.id,
            "SaaS Pro Maintenance",
            "Monthly",
            Decimal::from(199),
            "EUR",
            "2026-11-01",
            true,
        )
        .unwrap();
        assert_eq!(sched.plan_name, "SaaS Pro Maintenance");
        let all_scheds = InvoicingService::list_recurring_schedules(&ctx.db.conn()).unwrap();
        assert_eq!(all_scheds.len(), 1);

        let triggered = InvoicingService::trigger_recurring_invoices(&ctx).unwrap();
        assert_eq!(triggered.len(), 1);

        // 2. Factur-X / ZUGFeRD 2.2 XML Generation & EN 16931 Validation (LEDGER-28)
        let seller = crate::models::EInvoiceParty {
            name: "Ledger ERP Solutions GmbH".into(),
            vat_id: "DE999888777".into(),
            street: "Musterstrasse 42".into(),
            city: "Berlin".into(),
            postal_code: "10115".into(),
            country_code: "DE".into(),
            iban: Some("DE89370400440532013000".into()),
            bic: Some("COBADEFFXXX".into()),
        };
        let buyer = crate::models::EInvoiceParty {
            name: customer.name.clone(),
            vat_id: customer.tax_id.clone().unwrap_or("US12345678".into()),
            street: "123 Industrial Pkwy".into(),
            city: "Austin".into(),
            postal_code: "78701".into(),
            country_code: "US".into(),
            iban: None,
            bic: None,
        };

        let xml = InvoicingService::generate_factur_x_xml(&finalized, &seller, &buyer);
        assert!(xml.contains("<rsm:CrossIndustryInvoice"));
        assert!(xml.contains("urn:factur-x.eu:1p0:comfort"));
        assert!(xml.contains("DE999888777"));

        let report = InvoicingService::validate_en16931(&xml);
        assert!(report.is_valid);
        assert!(report.rule_checks_passed >= 6);
    }
}
