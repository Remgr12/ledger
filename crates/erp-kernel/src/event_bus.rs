use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::errors::ErpKernelError;

/// Core domain event representing any state transition in the ERP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub hlc_timestamp: String,
    pub actor_id: String,
    pub timestamp: DateTime<Utc>,
}

impl DomainEvent {
    pub fn new(
        event_type: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: serde_json::Value,
        hlc_timestamp: impl Into<String>,
        actor_id: impl Into<String>,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            payload,
            hlc_timestamp: hlc_timestamp.into(),
            actor_id: actor_id.into(),
            timestamp: Utc::now(),
        }
    }
}

pub type EventHandler = Arc<dyn Fn(&DomainEvent) -> Result<(), ErpKernelError> + Send + Sync>;

/// In-process event bus for synchronous and reactive inter-module communication.
#[derive(Default)]
pub struct EventBus {
    subscribers: RwLock<HashMap<String, Vec<EventHandler>>>,
    wildcard_subscribers: RwLock<Vec<EventHandler>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            wildcard_subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Subscribe to a specific event type (e.g., "InvoiceFinalized")
    pub fn subscribe(&self, event_type: impl Into<String>, handler: EventHandler) {
        let mut subs = self.subscribers.write();
        subs.entry(event_type.into()).or_default().push(handler);
    }

    /// Subscribe to all events passing through the bus
    pub fn subscribe_all(&self, handler: EventHandler) {
        let mut subs = self.wildcard_subscribers.write();
        subs.push(handler);
    }

    /// Publish an event to all matching subscribers
    pub fn publish(&self, event: &DomainEvent) -> Result<(), ErpKernelError> {
        // Dispatch to wildcard subscribers
        let wildcards = self.wildcard_subscribers.read().clone();
        for handler in wildcards {
            handler(event)?;
        }

        // Dispatch to typed subscribers
        let handlers = {
            let subs = self.subscribers.read();
            subs.get(&event.event_type).cloned()
        };

        if let Some(handlers) = handlers {
            for handler in handlers {
                handler(event)?;
            }
        }

        Ok(())
    }
}
