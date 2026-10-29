use chrono::Utc;
use rusqlite::{params, Connection};

use crate::errors::ErpKernelError;
use crate::event_bus::DomainEvent;

pub struct EventLogStore;

impl EventLogStore {
    /// Initialize the append-only event log schema.
    pub fn init_schema(conn: &Connection) -> Result<(), ErpKernelError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS event_log (
                id TEXT PRIMARY KEY,
                hlc_timestamp TEXT NOT NULL UNIQUE,
                aggregate_type TEXT NOT NULL,
                aggregate_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                synced INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_event_log_hlc ON event_log(hlc_timestamp ASC);
            CREATE INDEX IF NOT EXISTS idx_event_log_aggregate ON event_log(aggregate_type, aggregate_id);
            CREATE INDEX IF NOT EXISTS idx_event_log_synced ON event_log(synced, hlc_timestamp ASC);
            "#,
        )?;
        Ok(())
    }

    /// Append a domain event atomically to the event log.
    pub fn append(conn: &Connection, event: &DomainEvent) -> Result<(), ErpKernelError> {
        conn.execute(
            r#"
            INSERT INTO event_log (
                id, hlc_timestamp, aggregate_type, aggregate_id, event_type,
                payload_json, actor_id, synced, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
            "#,
            params![
                event.event_id,
                event.hlc_timestamp,
                event.aggregate_type,
                event.aggregate_id,
                event.event_type,
                event.payload.to_string(),
                event.actor_id,
                event.timestamp.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Retrieve events since a specific HLC timestamp for sync or projection replays.
    pub fn get_events_since(
        conn: &Connection,
        since_hlc: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DomainEvent>, ErpKernelError> {
        let sql = match since_hlc {
            Some(_) => {
                r#"
                SELECT id, hlc_timestamp, aggregate_type, aggregate_id, event_type,
                       payload_json, actor_id, created_at
                FROM event_log
                WHERE hlc_timestamp > ?1
                ORDER BY hlc_timestamp ASC
                LIMIT ?2
                "#
            }
            None => {
                r#"
                SELECT id, hlc_timestamp, aggregate_type, aggregate_id, event_type,
                       payload_json, actor_id, created_at
                FROM event_log
                ORDER BY hlc_timestamp ASC
                LIMIT ?1
                "#
            }
        };

        let mut stmt = conn.prepare(sql)?;
        let rows = match since_hlc {
            Some(hlc) => stmt.query_map(params![hlc, limit as i64], Self::map_row)?,
            None => stmt.query_map(params![limit as i64], Self::map_row)?,
        };

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Mark an array of event IDs as synced to cloud/relay server.
    pub fn mark_synced(conn: &Connection, event_ids: &[String]) -> Result<(), ErpKernelError> {
        if event_ids.is_empty() {
            return Ok(());
        }

        let mut stmt = conn.prepare("UPDATE event_log SET synced = 1 WHERE id = ?1")?;
        for id in event_ids {
            stmt.execute(params![id])?;
        }
        Ok(())
    }

    fn map_row(row: &rusqlite::Row<'_>) -> Result<DomainEvent, rusqlite::Error> {
        let id: String = row.get(0)?;
        let hlc_timestamp: String = row.get(1)?;
        let aggregate_type: String = row.get(2)?;
        let aggregate_id: String = row.get(3)?;
        let event_type: String = row.get(4)?;
        let payload_json: String = row.get(5)?;
        let actor_id: String = row.get(6)?;
        let created_at_str: String = row.get(7)?;

        let payload: serde_json::Value = serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
        let timestamp = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

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
    }
}
