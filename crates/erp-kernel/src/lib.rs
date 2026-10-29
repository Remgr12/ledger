pub mod db;
pub mod errors;
pub mod event_bus;
pub mod event_log;
pub mod hlc;
pub mod module;
pub mod module_manager;
pub mod sync;

pub use db::Database;
pub use errors::ErpKernelError;
pub use event_bus::{DomainEvent, EventBus, EventHandler};
pub use event_log::EventLogStore;
pub use hlc::{HlcTimestamp, HybridLogicalClock};
pub use module::{ErpModule, Migration, ModuleContext, ModuleRoute};
pub use module_manager::{ModuleManager, ModuleStatus};
pub use sync::SyncEngine;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct DummyModuleA;
    impl ErpModule for DummyModuleA {
        fn id(&self) -> &'static str {
            "mod-a"
        }
        fn name(&self) -> &'static str {
            "Module A"
        }
        fn description(&self) -> &'static str {
            "Test Module A"
        }
        fn migrations(&self) -> Vec<Migration> {
            vec![Migration {
                version: 1,
                name: "create_table_a",
                sql: "CREATE TABLE table_a (id TEXT PRIMARY KEY, val TEXT);",
            }]
        }
    }

    struct DummyModuleB;
    impl ErpModule for DummyModuleB {
        fn id(&self) -> &'static str {
            "mod-b"
        }
        fn name(&self) -> &'static str {
            "Module B"
        }
        fn description(&self) -> &'static str {
            "Test Module B depending on A"
        }
        fn dependencies(&self) -> Vec<&'static str> {
            vec!["mod-a"]
        }
        fn migrations(&self) -> Vec<Migration> {
            vec![Migration {
                version: 1,
                name: "create_table_b",
                sql: "CREATE TABLE table_b (id TEXT PRIMARY KEY, a_id TEXT REFERENCES table_a(id));",
            }]
        }
    }

    #[test]
    fn test_hlc_monotonicity() {
        let clock = HybridLogicalClock::new("node-1".into());
        let t1 = clock.now();
        let t2 = clock.now();
        assert!(t2 > t1);

        let s1 = t1.to_string();
        let parsed: HlcTimestamp = s1.parse().expect("Valid parse");
        assert_eq!(t1, parsed);
    }

    #[test]
    fn test_module_manager_dependency_enforcement() {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("test-node".into()));
        let event_bus = Arc::new(EventBus::new());

        let mm = ModuleManager::new(db.clone(), hlc, event_bus);
        mm.register(Arc::new(DummyModuleA));
        mm.register(Arc::new(DummyModuleB));
        mm.initialize().expect("initialize");

        // Enabling B before A must fail
        assert!(mm.enable_module("mod-b").is_err());

        // Enabling A succeeds
        assert!(mm.enable_module("mod-a").is_ok());

        // Now B succeeds
        assert!(mm.enable_module("mod-b").is_ok());

        // Disabling A while B is enabled must fail
        assert!(mm.disable_module("mod-a").is_err());

        // Disable B first
        assert!(mm.disable_module("mod-b").is_ok());
        // Now A can be disabled
        assert!(mm.disable_module("mod-a").is_ok());
    }

    #[test]
    fn test_event_bus_and_log() {
        let db = Database::open_in_memory().expect("open memory db");
        let hlc = Arc::new(HybridLogicalClock::new("test-node".into()));
        let event_bus = Arc::new(EventBus::new());

        let mm = ModuleManager::new(db.clone(), hlc.clone(), event_bus.clone());
        mm.initialize().expect("initialize");

        let counter = Arc::new(AtomicUsize::new(0));
        let c_clone = counter.clone();
        event_bus.subscribe("TestEvent", Arc::new(move |_| {
            c_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        let ctx = mm.context();
        ctx.record_and_publish_event(
            "TestEvent",
            "TestAggregate",
            "agg-123",
            serde_json::json!({ "foo": "bar" }),
            "user-1",
        ).expect("record event");

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        let events = EventLogStore::get_events_since(&db.conn(), None, 10).expect("get events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "TestEvent");
    }

    #[test]
    fn test_sync_engine_peer_convergence() {
        // Node 1 (Mobile device offline)
        let db1 = Database::open_in_memory().expect("db1");
        let hlc1 = Arc::new(HybridLogicalClock::new("node-mobile-1".into()));
        let bus1 = Arc::new(EventBus::new());
        let mm1 = ModuleManager::new(db1.clone(), hlc1.clone(), bus1.clone());
        mm1.initialize().expect("init mm1");

        // Node 2 (Desktop warehouse terminal)
        let db2 = Database::open_in_memory().expect("db2");
        let hlc2 = Arc::new(HybridLogicalClock::new("node-desktop-2".into()));
        let bus2 = Arc::new(EventBus::new());
        let mm2 = ModuleManager::new(db2.clone(), hlc2.clone(), bus2.clone());
        mm2.initialize().expect("init mm2");

        // Mobile device records 2 events offline
        let ctx1 = mm1.context();
        let ev1 = ctx1.record_and_publish_event(
            "ScanEvent", "Item", "item-101", serde_json::json!({"action": "pick"}), "user-1"
        ).expect("ev1");

        let ev2 = ctx1.record_and_publish_event(
            "ScanEvent", "Item", "item-102", serde_json::json!({"action": "pick"}), "user-1"
        ).expect("ev2");

        // Export un-synced events from mobile
        let unsynced = SyncEngine::export_unsynced_events(&db1.conn(), 10).expect("export");
        assert_eq!(unsynced.len(), 2);

        // Import into desktop warehouse terminal
        let imported = SyncEngine::import_remote_events(&db2.conn(), &hlc2, &unsynced).expect("import");
        assert_eq!(imported, 2);

        // Verify desktop clock advanced past the imported mobile clock
        let ev2_ts: HlcTimestamp = ev2.hlc_timestamp.parse().expect("parse ev2 ts");
        assert!(hlc2.now() >= ev2_ts);

        // Mark synced on mobile
        SyncEngine::mark_events_synced(&db1.conn(), &[ev1.event_id, ev2.event_id]).expect("mark synced");
        let remaining_unsynced = SyncEngine::export_unsynced_events(&db1.conn(), 10).expect("export remaining");
        assert_eq!(remaining_unsynced.len(), 0);
    }
}
