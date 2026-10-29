use chrono::Utc;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::errors::InventoryError;
use crate::models::{LocationTreeNode, LocationType, StorageLocation};
use erp_kernel::ModuleContext;

pub struct LocationService;

impl LocationService {
    /// Create a new storage location with automatic breadcrumb calculation and closure table indexing.
    pub fn create_location(
        ctx: &ModuleContext,
        code: &str,
        name: &str,
        parent_id: Option<&str>,
        location_type: LocationType,
        map_x: Option<f64>,
        map_y: Option<f64>,
    ) -> Result<StorageLocation, InventoryError> {
        let location_id = Uuid::new_v4().to_string();
        let qr_code_uri = format!("erp://loc/{}", location_id);
        let now = Utc::now();

        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        // Calculate path breadcrumb from parent
        let path_breadcrumb = match parent_id {
            Some(pid) => {
                let parent_path: String = tx
                    .query_row(
                        "SELECT path_breadcrumb FROM storage_locations WHERE id = ?1",
                        params![pid],
                        |r| r.get(0),
                    )
                    .map_err(|_| InventoryError::LocationNotFound(pid.to_string()))?;
                format!("{} > {}", parent_path, name)
            }
            None => name.to_string(),
        };

        // Insert into storage_locations
        tx.execute(
            r#"
            INSERT INTO storage_locations (
                id, code, name, parent_id, location_type, path_breadcrumb,
                qr_code_uri, map_x, map_y, is_active, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)
            "#,
            params![
                location_id,
                code,
                name,
                parent_id,
                location_type.as_str(),
                path_breadcrumb,
                qr_code_uri,
                map_x,
                map_y,
                now.to_rfc3339(),
            ],
        )?;

        // Populate location_closure:
        // 1. Self reference at depth 0
        tx.execute(
            "INSERT INTO location_closure (ancestor_id, descendant_id, depth) VALUES (?1, ?1, 0)",
            params![location_id],
        )?;

        // 2. Transitive ancestors from parent
        if let Some(pid) = parent_id {
            tx.execute(
                r#"
                INSERT INTO location_closure (ancestor_id, descendant_id, depth)
                SELECT ancestor_id, ?1, depth + 1
                FROM location_closure
                WHERE descendant_id = ?2
                "#,
                params![location_id, pid],
            )?;
        }

        tx.commit()?;
        drop(conn);

        let location = StorageLocation {
            id: location_id.clone(),
            code: code.to_string(),
            name: name.to_string(),
            parent_id: parent_id.map(String::from),
            location_type,
            path_breadcrumb,
            qr_code_uri,
            map_x,
            map_y,
            is_active: true,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "LocationCreated",
            "StorageLocation",
            &location_id,
            serde_json::to_value(&location).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(location)
    }

    /// Move a location to a new parent in the hierarchy, maintaining the closure table
    /// and recursively updating all descendant breadcrumb paths.
    pub fn move_location(
        ctx: &ModuleContext,
        location_id: &str,
        new_parent_id: Option<&str>,
    ) -> Result<StorageLocation, InventoryError> {
        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        // 1. Verify location exists
        let current_loc = Self::get_location_by_id(&tx, location_id)?;

        // If parent unchanged, return current
        if current_loc.parent_id.as_deref() == new_parent_id {
            return Ok(current_loc);
        }

        // 2. Prevent cycles: new parent cannot be a descendant of location_id
        if let Some(new_pid) = new_parent_id {
            if new_pid == location_id {
                return Err(InventoryError::InvalidMovement(
                    "Cannot set a location as its own parent".into(),
                ));
            }

            let is_descendant: bool = tx
                .query_row(
                    r#"
                    SELECT 1 FROM location_closure
                    WHERE ancestor_id = ?1 AND descendant_id = ?2
                    "#,
                    params![location_id, new_pid],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if is_descendant {
                return Err(InventoryError::InvalidMovement(format!(
                    "Cannot move location '{}' under its own descendant '{}'",
                    location_id, new_pid
                )));
            }
        }

        // 3. Disconnect location_id and all its descendants from current ancestors
        // Delete rows where descendant is a descendant of location_id,
        // and ancestor is an ancestor of location_id (excluding descendants of location_id)
        tx.execute(
            r#"
            DELETE FROM location_closure
            WHERE descendant_id IN (
                SELECT descendant_id FROM location_closure WHERE ancestor_id = ?1
            )
            AND ancestor_id IN (
                SELECT ancestor_id FROM location_closure
                WHERE descendant_id = ?1 AND ancestor_id != ?1
            )
            "#,
            params![location_id],
        )?;

        // 4. Connect location_id and its descendants to new parent's ancestors
        if let Some(new_pid) = new_parent_id {
            tx.execute(
                r#"
                INSERT INTO location_closure (ancestor_id, descendant_id, depth)
                SELECT supertree.ancestor_id, subtree.descendant_id, supertree.depth + subtree.depth + 1
                FROM location_closure AS supertree
                CROSS JOIN location_closure AS subtree
                WHERE supertree.descendant_id = ?1 AND subtree.ancestor_id = ?2
                "#,
                params![new_pid, location_id],
            )?;
        }

        // 5. Update parent_id in storage_locations
        tx.execute(
            "UPDATE storage_locations SET parent_id = ?1 WHERE id = ?2",
            params![new_parent_id, location_id],
        )?;

        // 6. Recursively update path_breadcrumb for location_id and all its descendants
        Self::rebuild_descendant_breadcrumbs(&tx, location_id)?;

        let updated_loc = Self::get_location_by_id(&tx, location_id)?;
        tx.commit()?;
        drop(conn);

        let _ = ctx.record_and_publish_event(
            "LocationMoved",
            "StorageLocation",
            location_id,
            serde_json::to_value(&updated_loc).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(updated_loc)
    }

    /// Recursively update breadcrumbs for a node and all its subtree children
    fn rebuild_descendant_breadcrumbs(conn: &Connection, node_id: &str) -> Result<(), InventoryError> {
        let parent_id: Option<String> = conn
            .query_row(
                "SELECT parent_id FROM storage_locations WHERE id = ?1",
                params![node_id],
                |r| r.get(0),
            )
            .ok()
            .flatten();

        let name: String = conn.query_row(
            "SELECT name FROM storage_locations WHERE id = ?1",
            params![node_id],
            |r| r.get(0),
        )?;

        let new_breadcrumb = match parent_id {
            Some(pid) => {
                let parent_path: String = conn.query_row(
                    "SELECT path_breadcrumb FROM storage_locations WHERE id = ?1",
                    params![pid],
                    |r| r.get(0),
                )?;
                format!("{} > {}", parent_path, name)
            }
            None => name,
        };

        conn.execute(
            "UPDATE storage_locations SET path_breadcrumb = ?1 WHERE id = ?2",
            params![new_breadcrumb, node_id],
        )?;

        // Fetch direct children and recurse
        let mut stmt = conn.prepare("SELECT id FROM storage_locations WHERE parent_id = ?1")?;
        let child_ids = stmt
            .query_map(params![node_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for cid in child_ids {
            Self::rebuild_descendant_breadcrumbs(conn, &cid)?;
        }

        Ok(())
    }

    pub fn get_location_by_id(conn: &Connection, id: &str) -> Result<StorageLocation, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, parent_id, location_type, path_breadcrumb,
                   qr_code_uri, map_x, map_y, is_active, created_at
            FROM storage_locations
            WHERE id = ?1
            "#,
        )?;
        stmt.query_row(params![id], Self::map_location_row)
            .map_err(|_| InventoryError::LocationNotFound(id.to_string()))
    }

    pub fn get_location_by_code(conn: &Connection, code: &str) -> Result<StorageLocation, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, parent_id, location_type, path_breadcrumb,
                   qr_code_uri, map_x, map_y, is_active, created_at
            FROM storage_locations
            WHERE code = ?1
            "#,
        )?;
        stmt.query_row(params![code], Self::map_location_row)
            .map_err(|_| InventoryError::LocationNotFound(code.to_string()))
    }

    pub fn list_all_locations(conn: &Connection) -> Result<Vec<StorageLocation>, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, code, name, parent_id, location_type, path_breadcrumb,
                   qr_code_uri, map_x, map_y, is_active, created_at
            FROM storage_locations
            WHERE is_active = 1
            ORDER BY path_breadcrumb ASC
            "#,
        )?;
        let rows = stmt.query_map([], Self::map_location_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Retrieve the entire storage hierarchy as a tree for the UI
    pub fn get_location_tree(conn: &Connection) -> Result<Vec<LocationTreeNode>, InventoryError> {
        let all_locations = Self::list_all_locations(conn)?;
        Ok(Self::build_tree(None, &all_locations))
    }

    fn build_tree(parent_id: Option<&str>, all: &[StorageLocation]) -> Vec<LocationTreeNode> {
        all.iter()
            .filter(|loc| loc.parent_id.as_deref() == parent_id)
            .map(|loc| {
                let children = Self::build_tree(Some(&loc.id), all);
                LocationTreeNode {
                    location: loc.clone(),
                    children,
                    total_item_count: 0,
                }
            })
            .collect()
    }

    fn map_location_row(row: &Row<'_>) -> Result<StorageLocation, rusqlite::Error> {
        let id: String = row.get(0)?;
        let code: String = row.get(1)?;
        let name: String = row.get(2)?;
        let parent_id: Option<String> = row.get(3)?;
        let location_type_str: String = row.get(4)?;
        let path_breadcrumb: String = row.get(5)?;
        let qr_code_uri: String = row.get(6)?;
        let map_x: Option<f64> = row.get(7)?;
        let map_y: Option<f64> = row.get(8)?;
        let is_active_int: i32 = row.get(9)?;
        let created_at_str: String = row.get(10)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        use std::str::FromStr;
        Ok(StorageLocation {
            id,
            code,
            name,
            parent_id,
            location_type: LocationType::from_str(&location_type_str).unwrap_or(LocationType::Bin),
            path_breadcrumb,
            qr_code_uri,
            map_x,
            map_y,
            is_active: is_active_int != 0,
            created_at,
        })
    }
}
