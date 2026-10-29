use rusqlite::{params, Connection};
use std::str::FromStr;

use crate::errors::ErpKernelError;
use crate::event_bus::DomainEvent;
use crate::event_log::EventLogStore;
use crate::hlc::{HlcTimestamp, HybridLogicalClock};

pub struct SyncEngine;

impl SyncEngine {
    /// Export un-synced local events ready to be pushed to cloud or peer device.
    pub fn export_unsynced_events(
        conn: &Connection,
        limit: usize,
    ) -> Result<Vec<DomainEvent>, ErpKernelError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, hlc_timestamp, aggregate_type, aggregate_id, event_type,
                   payload_json, actor_id, created_at
            FROM event_log
            WHERE synced = 0
            ORDER BY hlc_timestamp ASC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id: String = row.get(0)?;
            let hlc_timestamp: String = row.get(1)?;
            let aggregate_type: String = row.get(2)?;
            let aggregate_id: String = row.get(3)?;
            let event_type: String = row.get(4)?;
            let payload_json: String = row.get(5)?;
            let actor_id: String = row.get(6)?;
            let created_at_str: String = row.get(7)?;

            let payload: serde_json::Value =
                serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
            let timestamp = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            Ok(DomainEvent {
                event_id: id,
                event_type,
                aggregate_type,
                aggregate_id,
                payload,
                hlc_timestamp,
                actor_id,
                timestamp,
            })
        })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r?);
        }
        Ok(events)
    }

    /// Ingest a batch of remote events received from another node or server.
    /// Updates local HLC clock monotonically and preserves causal order.
    pub fn import_remote_events(
        conn: &Connection,
        hlc: &HybridLogicalClock,
        events: &[DomainEvent],
    ) -> Result<usize, ErpKernelError> {
        let mut count = 0;
        let mut check_stmt = conn.prepare("SELECT 1 FROM event_log WHERE id = ?1")?;
        let mut insert_stmt = conn.prepare(
            r#"
            INSERT INTO event_log (
                id, hlc_timestamp, aggregate_type, aggregate_id, event_type,
                payload_json, actor_id, synced, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)
            "#,
        )?;

        for event in events {
            if let Ok(remote_hlc) = HlcTimestamp::from_str(&event.hlc_timestamp) {
                hlc.update(&remote_hlc);
            }

            let exists: bool = check_stmt
                .query_row(params![event.event_id], |_| Ok(true))
                .unwrap_or(false);

            if !exists {
                insert_stmt.execute(params![
                    event.event_id,
                    event.hlc_timestamp,
                    event.aggregate_type,
                    event.aggregate_id,
                    event.event_type,
                    event.payload.to_string(),
                    event.actor_id,
                    event.timestamp.to_rfc3339(),
                ])?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Mark an array of event IDs as synced.
    pub fn mark_events_synced(
        conn: &Connection,
        event_ids: &[String],
    ) -> Result<(), ErpKernelError> {
        EventLogStore::mark_synced(conn, event_ids)
    }
}
