use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::errors::ErpKernelError;
use crate::event_bus::{DomainEvent, EventBus};
use crate::event_log::EventLogStore;
use crate::hlc::HybridLogicalClock;

/// A single isolated database migration for an ERP module.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Dynamic navigation route exposed by a module for the Kotlin Compose UI shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRoute {
    pub route_id: String,
    pub title: String,
    pub icon: String,
    pub display_order: u32,
}

/// Context provided to modules during event dispatch and business operations.
#[derive(Clone)]
pub struct ModuleContext {
    pub db: Database,
    pub hlc: Arc<HybridLogicalClock>,
    pub event_bus: Arc<EventBus>,
}

impl ModuleContext {
    pub fn new(db: Database, hlc: Arc<HybridLogicalClock>, event_bus: Arc<EventBus>) -> Self {
        Self { db, hlc, event_bus }
    }

    /// Atomically records a domain event into the HLC event log and publishes it on the event bus.
    pub fn record_and_publish_event(
        &self,
        event_type: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: serde_json::Value,
        actor_id: impl Into<String>,
    ) -> Result<DomainEvent, ErpKernelError> {
        let hlc_ts = self.hlc.now().to_string();
        let event = DomainEvent::new(
            event_type,
            aggregate_type,
            aggregate_id,
            payload,
            hlc_ts,
            actor_id,
        );

        {
            let conn = self.db.conn();
            EventLogStore::append(&conn, &event)?;
        }

        self.event_bus.publish(&event)?;
        Ok(event)
    }
}

/// The ErpModule trait that every functional module (inventory, invoicing, contacts, etc.) must implement.
pub trait ErpModule: Send + Sync {
    /// Unique identifier for the module (e.g. "mod-inventory", "mod-invoicing")
    fn id(&self) -> &'static str;

    /// Human-readable module name
    fn name(&self) -> &'static str;

    /// Detailed description of what this module provides
    fn description(&self) -> &'static str;

    /// Module IDs that this module strictly depends upon
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// List of ordered SQLite schema migrations
    fn migrations(&self) -> Vec<Migration> {
        Vec::new()
    }

    /// Inter-module event handler invoked when an active module emits a domain event
    fn on_event(&self, _event: &DomainEvent, _ctx: &ModuleContext) -> Result<(), ErpKernelError> {
        Ok(())
    }

    /// Navigation routes that this module contributes to the UI shell
    fn routes(&self) -> Vec<ModuleRoute> {
        Vec::new()
    }
}
