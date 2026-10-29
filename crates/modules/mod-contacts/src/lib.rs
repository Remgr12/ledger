pub mod models;
pub mod service;

pub use models::{Contact, ContactType};
pub use service::ContactsService;

use erp_kernel::{ErpModule, Migration, ModuleRoute};

pub struct ContactsModule;

impl Default for ContactsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ContactsModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for ContactsModule {
    fn id(&self) -> &'static str {
        "mod-contacts"
    }

    fn name(&self) -> &'static str {
        "Contacts & CRM"
    }

    fn description(&self) -> &'static str {
        "Customers, suppliers, vendor profiles, and addresses"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_contacts_table",
            sql: r#"
            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                contact_type TEXT NOT NULL,
                name TEXT NOT NULL,
                email TEXT,
                phone TEXT,
                tax_id TEXT,
                address TEXT,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_contacts_name ON contacts(name);
            CREATE INDEX IF NOT EXISTS idx_contacts_type ON contacts(contact_type);
            "#,
        }]
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![ModuleRoute {
            route_id: "contacts/list".to_string(),
            title: "Contacts".to_string(),
            icon: "people".to_string(),
            display_order: 20,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_kernel::{Database, EventBus, HybridLogicalClock, ModuleManager};
    use std::sync::Arc;

    #[test]
    fn test_contacts_lifecycle_and_lookup() {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("node-test-contacts".to_string()));
        let bus = Arc::new(EventBus::new());

        let mm = Arc::new(ModuleManager::new(db.clone(), hlc, bus));
        mm.register(Arc::new(ContactsModule::new()));
        mm.initialize().expect("init mm");
        mm.enable_module("mod-contacts").expect("enable contacts");

        let ctx = mm.context();

        let contact = ContactsService::create_contact(
            &ctx,
            ContactType::Customer,
            "Acme Retailers Ltd",
            Some("purchasing@acme.com"),
            Some("+1-555-0199"),
            Some("US-TAX-98214"),
            Some("100 Industrial Parkway, Chicago, IL"),
        )
        .expect("Failed to create customer");

        assert_eq!(contact.name, "Acme Retailers Ltd");
        assert_eq!(contact.contact_type, ContactType::Customer);

        let fetched = ContactsService::get_contact_by_id(&db.conn(), &contact.id)
            .expect("Failed to fetch contact by ID");
        assert_eq!(fetched.name, "Acme Retailers Ltd");
        assert_eq!(fetched.tax_id.as_deref(), Some("US-TAX-98214"));

        let all = ContactsService::list_contacts(&db.conn()).expect("Failed to list contacts");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, contact.id);
    }
}
