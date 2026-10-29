use chrono::Utc;
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::InventoryError;
use crate::models::{
    InventoryItem, ItemLocationMatch, MovementType, StockMovement,
};
use erp_kernel::ModuleContext;

pub struct StockService;

impl StockService {
    /// Create a new inventory item with SKU, tracking policies, and QR code URI.
    #[allow(clippy::too_many_arguments)]
    pub fn create_item(
        ctx: &ModuleContext,
        sku: &str,
        name: &str,
        description: Option<&str>,
        unit: &str,
        track_serial: bool,
        track_lot: bool,
        min_stock_level: Decimal,
        barcode: Option<&str>,
    ) -> Result<InventoryItem, InventoryError> {
        let item_id = Uuid::new_v4().to_string();
        let qr_code_uri = format!("erp://item/{}", sku);
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO inventory_items (
                    id, sku, name, description, unit, track_serial, track_lot,
                    min_stock_level, qr_code_uri, barcode, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    item_id,
                    sku,
                    name,
                    description,
                    unit,
                    if track_serial { 1 } else { 0 },
                    if track_lot { 1 } else { 0 },
                    min_stock_level.to_string(),
                    qr_code_uri,
                    barcode,
                    now.to_rfc3339(),
                ],
            )?;
        }

        let item = InventoryItem {
            id: item_id.clone(),
            sku: sku.to_string(),
            name: name.to_string(),
            description: description.map(String::from),
            unit: unit.to_string(),
            track_serial,
            track_lot,
            min_stock_level,
            qr_code_uri,
            barcode: barcode.map(String::from),
            standard_cost: None,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "ItemCreated",
            "InventoryItem",
            &item_id,
            serde_json::to_value(&item).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(item)
    }

    pub fn get_item_by_id(conn: &Connection, id: &str) -> Result<InventoryItem, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, sku, name, description, unit, track_serial, track_lot,
                   min_stock_level, qr_code_uri, barcode, created_at
            FROM inventory_items
            WHERE id = ?1
            "#,
        )?;
        stmt.query_row(params![id], Self::map_item_row)
            .map_err(|_| InventoryError::ItemNotFound(id.to_string()))
    }

    pub fn get_item_by_sku(conn: &Connection, sku: &str) -> Result<InventoryItem, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, sku, name, description, unit, track_serial, track_lot,
                   min_stock_level, qr_code_uri, barcode, created_at
            FROM inventory_items
            WHERE sku = ?1
            "#,
        )?;
        stmt.query_row(params![sku], Self::map_item_row)
            .map_err(|_| InventoryError::ItemNotFound(sku.to_string()))
    }

    pub fn list_all_items(conn: &Connection) -> Result<Vec<InventoryItem>, InventoryError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, sku, name, description, unit, track_serial, track_lot,
                   min_stock_level, qr_code_uri, barcode, created_at
            FROM inventory_items
            ORDER BY sku ASC
            "#,
        )?;
        let rows = stmt.query_map([], Self::map_item_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Record an immutable double-entry stock movement and update the materialized stock quants.
    /// Never overwrite quantities directly—all changes flow through this ledger entry.
    #[allow(clippy::too_many_arguments)]
    pub fn record_movement(
        ctx: &ModuleContext,
        movement_type: MovementType,
        item_id: &str,
        from_location_id: Option<&str>,
        to_location_id: Option<&str>,
        quantity: Decimal,
        lot_number: Option<&str>,
        serial_number: Option<&str>,
        reference_id: Option<&str>,
        notes: Option<&str>,
        actor_id: &str,
    ) -> Result<StockMovement, InventoryError> {
        if quantity <= Decimal::ZERO {
            return Err(InventoryError::InvalidMovement(
                "Stock movement quantity must be strictly greater than zero".into(),
            ));
        }

        if from_location_id.is_none() && to_location_id.is_none() {
            return Err(InventoryError::InvalidMovement(
                "Stock movement requires at least a from_location or a to_location".into(),
            ));
        }

        if let (Some(f), Some(t)) = (from_location_id, to_location_id) {
            if f == t {
                return Err(InventoryError::InvalidMovement(
                    "Source and destination locations cannot be identical".into(),
                ));
            }
        }

        let movement_id = Uuid::new_v4().to_string();
        let hlc_ts = ctx.hlc.now().to_string();
        let now = Utc::now();

        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        // 1. Verify item exists
        let item_exists: bool = tx
            .query_row(
                "SELECT 1 FROM inventory_items WHERE id = ?1",
                params![item_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !item_exists {
            return Err(InventoryError::ItemNotFound(item_id.to_string()));
        }

        // 2. Deduct from source location if specified
        if let Some(from_loc) = from_location_id {
            let current_qty_str: Option<String> = tx
                .query_row(
                    r#"
                    SELECT quantity FROM stock_quants
                    WHERE item_id = ?1 AND location_id = ?2
                      AND (lot_number = ?3 OR (lot_number IS NULL AND ?3 IS NULL))
                      AND (serial_number = ?4 OR (serial_number IS NULL AND ?4 IS NULL))
                    "#,
                    params![item_id, from_loc, lot_number, serial_number],
                    |r| r.get(0),
                )
                .ok();

            let available = match current_qty_str {
                Some(s) => Decimal::from_str(&s).unwrap_or(Decimal::ZERO),
                None => Decimal::ZERO,
            };

            if available < quantity && movement_type != MovementType::Adjust {
                return Err(InventoryError::InsufficientStock {
                    item_id: item_id.to_string(),
                    location_id: from_loc.to_string(),
                    available,
                    requested: quantity,
                });
            }

            let new_available = available - quantity;
            if new_available == Decimal::ZERO {
                tx.execute(
                    r#"
                    DELETE FROM stock_quants
                    WHERE item_id = ?1 AND location_id = ?2
                      AND (lot_number = ?3 OR (lot_number IS NULL AND ?3 IS NULL))
                      AND (serial_number = ?4 OR (serial_number IS NULL AND ?4 IS NULL))
                    "#,
                    params![item_id, from_loc, lot_number, serial_number],
                )?;
            } else {
                tx.execute(
                    r#"
                    UPDATE stock_quants
                    SET quantity = ?1, updated_at = ?2
                    WHERE item_id = ?3 AND location_id = ?4
                      AND (lot_number = ?5 OR (lot_number IS NULL AND ?5 IS NULL))
                      AND (serial_number = ?6 OR (serial_number IS NULL AND ?6 IS NULL))
                    "#,
                    params![
                        new_available.to_string(),
                        now.to_rfc3339(),
                        item_id,
                        from_loc,
                        lot_number,
                        serial_number
                    ],
                )?;
            }
        }

        // 3. Add to destination location if specified
        if let Some(to_loc) = to_location_id {
            let current_qty_str: Option<String> = tx
                .query_row(
                    r#"
                    SELECT quantity FROM stock_quants
                    WHERE item_id = ?1 AND location_id = ?2
                      AND (lot_number = ?3 OR (lot_number IS NULL AND ?3 IS NULL))
                      AND (serial_number = ?4 OR (serial_number IS NULL AND ?4 IS NULL))
                    "#,
                    params![item_id, to_loc, lot_number, serial_number],
                    |r| r.get(0),
                )
                .ok();

            match current_qty_str {
                Some(s) => {
                    let existing_qty = Decimal::from_str(&s).unwrap_or(Decimal::ZERO);
                    let new_qty = existing_qty + quantity;
                    tx.execute(
                        r#"
                        UPDATE stock_quants
                        SET quantity = ?1, updated_at = ?2
                        WHERE item_id = ?3 AND location_id = ?4
                          AND (lot_number = ?5 OR (lot_number IS NULL AND ?5 IS NULL))
                          AND (serial_number = ?6 OR (serial_number IS NULL AND ?6 IS NULL))
                        "#,
                        params![
                            new_qty.to_string(),
                            now.to_rfc3339(),
                            item_id,
                            to_loc,
                            lot_number,
                            serial_number
                        ],
                    )?;
                }
                None => {
                    let quant_id = Uuid::new_v4().to_string();
                    tx.execute(
                        r#"
                        INSERT INTO stock_quants (
                            id, item_id, location_id, quantity, lot_number, serial_number, updated_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                        "#,
                        params![
                            quant_id,
                            item_id,
                            to_loc,
                            quantity.to_string(),
                            lot_number,
                            serial_number,
                            now.to_rfc3339(),
                        ],
                    )?;
                }
            }
        }

        // 4. Insert immutable movement ledger record
        tx.execute(
            r#"
            INSERT INTO stock_movements (
                id, movement_type, item_id, from_location_id, to_location_id,
                quantity, lot_number, serial_number, reference_id, notes,
                hlc_timestamp, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                movement_id,
                movement_type.as_str(),
                item_id,
                from_location_id,
                to_location_id,
                quantity.to_string(),
                lot_number,
                serial_number,
                reference_id,
                notes,
                hlc_ts,
                now.to_rfc3339(),
            ],
        )?;

        tx.commit()?;
        drop(conn);

        let movement = StockMovement {
            id: movement_id.clone(),
            movement_type,
            item_id: item_id.to_string(),
            from_location_id: from_location_id.map(String::from),
            to_location_id: to_location_id.map(String::from),
            quantity,
            lot_number: lot_number.map(String::from),
            serial_number: serial_number.map(String::from),
            unit_cost: None,
            reference_id: reference_id.map(String::from),
            notes: notes.map(String::from),
            hlc_timestamp: hlc_ts,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "StockMovementRecorded",
            "StockMovement",
            &movement_id,
            serde_json::to_value(&movement).unwrap_or(serde_json::Value::Null),
            actor_id,
        )?;

        Ok(movement)
    }

    /// Reconcile a physical cycle count audit against system on-hand balance.
    /// Automatically records an immutable double-entry `Adjust` movement for any discrepancy.
    pub fn reconcile_audit(
        ctx: &ModuleContext,
        location_id: &str,
        item_id: &str,
        physical_count: Decimal,
        lot_number: Option<&str>,
        serial_number: Option<&str>,
        actor_id: &str,
    ) -> Result<Option<StockMovement>, InventoryError> {
        let current_qty_str: Option<String> = {
            let conn = ctx.db.conn();
            conn.query_row(
                r#"
                SELECT quantity FROM stock_quants
                WHERE item_id = ?1 AND location_id = ?2
                  AND (lot_number = ?3 OR (lot_number IS NULL AND ?3 IS NULL))
                  AND (serial_number = ?4 OR (serial_number IS NULL AND ?4 IS NULL))
                "#,
                params![item_id, location_id, lot_number, serial_number],
                |r| r.get(0),
            )
            .ok()
        };

        let system_qty = current_qty_str
            .and_then(|s| Decimal::from_str(&s).ok())
            .unwrap_or(Decimal::ZERO);

        let delta = physical_count - system_qty;
        if delta == Decimal::ZERO {
            return Ok(None);
        }

        if delta > Decimal::ZERO {
            let movement = Self::record_movement(
                ctx,
                MovementType::Adjust,
                item_id,
                None,
                Some(location_id),
                delta,
                lot_number,
                serial_number,
                None,
                Some("Cycle count audit reconciliation: surplus added"),
                actor_id,
            )?;
            Ok(Some(movement))
        } else {
            let abs_delta = -delta;
            let movement = Self::record_movement(
                ctx,
                MovementType::Adjust,
                item_id,
                Some(location_id),
                None,
                abs_delta,
                lot_number,
                serial_number,
                None,
                Some("Cycle count audit reconciliation: shrinkage deducted"),
                actor_id,
            )?;
            Ok(Some(movement))
        }
    }

    /// Calculate total inventory units/valuation using rust_decimal.
    pub fn calculate_inventory_valuation(
        conn: &Connection,
        location_id_filter: Option<&str>,
    ) -> Result<Decimal, InventoryError> {
        let mut total = Decimal::ZERO;
        if let Some(loc) = location_id_filter {
            let mut stmt = conn.prepare("SELECT quantity FROM stock_quants WHERE location_id = ?1")?;
            let rows = stmt.query_map(params![loc], |r| r.get::<_, String>(0))?;
            for r in rows {
                let q_str = r?;
                if let Ok(d) = Decimal::from_str(&q_str) {
                    total += d;
                }
            }
        } else {
            let mut stmt = conn.prepare("SELECT quantity FROM stock_quants")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for r in rows {
                let q_str = r?;
                if let Ok(d) = Decimal::from_str(&q_str) {
                    total += d;
                }
            }
        }
        Ok(total)
    }

    /// Instant "Where Is It?" item locator returning exact quantities,
    /// human-readable breadcrumb paths (`Workshop > North Wall > Rack 2 > Shelf C > Blue Bin #4`),
    /// and floor-plan coordinates (x, y) for 2D highlighting.
    pub fn locate_item(conn: &Connection, search_query: &str) -> Result<Vec<ItemLocationMatch>, InventoryError> {
        let pattern = format!("%{}%", search_query.trim());
        let mut stmt = conn.prepare(
            r#"
            SELECT
                i.id AS item_id,
                i.sku,
                i.name AS item_name,
                i.unit,
                l.id AS location_id,
                l.code AS location_code,
                l.name AS location_name,
                l.location_type,
                l.path_breadcrumb,
                l.map_x,
                l.map_y,
                q.quantity,
                q.lot_number,
                q.serial_number
            FROM stock_quants q
            INNER JOIN inventory_items i ON q.item_id = i.id
            INNER JOIN storage_locations l ON q.location_id = l.id
            WHERE (i.sku LIKE ?1 OR i.name LIKE ?1 OR i.barcode LIKE ?1 OR q.lot_number LIKE ?1 OR q.serial_number LIKE ?1)
              AND CAST(q.quantity AS REAL) > 0
            ORDER BY l.path_breadcrumb ASC, CAST(q.quantity AS REAL) DESC
            "#,
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            let item_id: String = row.get(0)?;
            let sku: String = row.get(1)?;
            let item_name: String = row.get(2)?;
            let unit: String = row.get(3)?;
            let location_id: String = row.get(4)?;
            let location_code: String = row.get(5)?;
            let location_name: String = row.get(6)?;
            let location_type: String = row.get(7)?;
            let breadcrumb_path: String = row.get(8)?;
            let map_x: Option<f64> = row.get(9)?;
            let map_y: Option<f64> = row.get(10)?;
            let quantity_str: String = row.get(11)?;
            let lot_number: Option<String> = row.get(12)?;
            let serial_number: Option<String> = row.get(13)?;

            let quantity = Decimal::from_str(&quantity_str).unwrap_or(Decimal::ZERO);

            Ok(ItemLocationMatch {
                item_id,
                sku,
                item_name,
                unit,
                location_id,
                location_code,
                location_name,
                location_type,
                breadcrumb_path,
                map_x,
                map_y,
                quantity,
                lot_number,
                serial_number,
            })
        })?;

        let mut matches = Vec::new();
        for r in rows {
            matches.push(r?);
        }

        Ok(matches)
    }

    /// Get total on-hand stock across all locations for an item.
    pub fn get_total_stock(conn: &Connection, item_id: &str) -> Result<Decimal, InventoryError> {
        let mut stmt = conn.prepare("SELECT quantity FROM stock_quants WHERE item_id = ?1")?;
        let rows = stmt.query_map(params![item_id], |r| r.get::<_, String>(0))?;
        let mut total = Decimal::ZERO;
        for r in rows {
            let q_str = r?;
            if let Ok(d) = Decimal::from_str(&q_str) {
                total += d;
            }
        }
        Ok(total)
    }

    fn map_item_row(row: &Row<'_>) -> Result<InventoryItem, rusqlite::Error> {
        let id: String = row.get(0)?;
        let sku: String = row.get(1)?;
        let name: String = row.get(2)?;
        let description: Option<String> = row.get(3)?;
        let unit: String = row.get(4)?;
        let track_serial_int: i32 = row.get(5)?;
        let track_lot_int: i32 = row.get(6)?;
        let min_stock_str: String = row.get(7)?;
        let qr_code_uri: String = row.get(8)?;
        let barcode: Option<String> = row.get(9)?;
        let created_at_str: String = row.get(10)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let min_stock_level = Decimal::from_str(&min_stock_str).unwrap_or(Decimal::ZERO);

        Ok(InventoryItem {
            id,
            sku,
            name,
            description,
            unit,
            track_serial: track_serial_int != 0,
            track_lot: track_lot_int != 0,
            min_stock_level,
            qr_code_uri,
            barcode,
            standard_cost: None,
            created_at,
        })
    }
}
