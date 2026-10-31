use chrono::Utc;
use erp_kernel::ModuleContext;
use mod_inventory::{MovementType, StockService};
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::ManufacturingError;
use crate::models::{
    BillOfMaterials, BomComponent, ComponentGenealogyEntry, WorkOrder, WorkOrderGenealogyTree,
    WorkOrderStatus,
};

pub struct ManufacturingService;

impl ManufacturingService {
    /// Create a new Bill of Materials (BOM)
    #[allow(clippy::too_many_arguments)]
    pub fn create_bom(
        ctx: &ModuleContext,
        bom_number: &str,
        item_id: &str,
        name: &str,
        quantity: Decimal,
        components: Vec<(String, Decimal, Decimal, Option<String>)>, // (item_id, qty, scrap, notes)
        routing_notes: Option<&str>,
    ) -> Result<BillOfMaterials, ManufacturingError> {
        if components.is_empty() {
            return Err(ManufacturingError::InvalidConfiguration(
                "A Bill of Materials requires at least one component".into(),
            ));
        }

        if quantity <= Decimal::ZERO {
            return Err(ManufacturingError::InvalidConfiguration(
                "BOM output quantity must be greater than zero".into(),
            ));
        }

        let bom_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let mut bom_components = Vec::new();
        for (comp_item_id, qty, scrap, notes) in components {
            if qty <= Decimal::ZERO {
                return Err(ManufacturingError::InvalidConfiguration(
                    "Component quantity must be greater than zero".into(),
                ));
            }

            bom_components.push(BomComponent {
                id: Uuid::new_v4().to_string(),
                bom_id: bom_id.clone(),
                item_id: comp_item_id,
                quantity: qty,
                scrap_percentage: scrap,
                notes,
            });
        }

        {
            let mut conn = ctx.db.conn();
            let tx = conn.transaction()?;

            tx.execute(
                r#"
                INSERT INTO boms (id, bom_number, item_id, name, quantity, routing_notes, is_active, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)
                "#,
                params![
                    bom_id,
                    bom_number,
                    item_id,
                    name,
                    quantity.to_string(),
                    routing_notes,
                    now.to_rfc3339()
                ],
            )?;

            let mut comp_stmt = tx.prepare(
                r#"
                INSERT INTO bom_components (id, bom_id, item_id, quantity, scrap_percentage, notes)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;

            for comp in &bom_components {
                comp_stmt.execute(params![
                    comp.id,
                    comp.bom_id,
                    comp.item_id,
                    comp.quantity.to_string(),
                    comp.scrap_percentage.to_string(),
                    comp.notes
                ])?;
            }
            drop(comp_stmt);

            tx.commit()?;
        }

        let bom = BillOfMaterials {
            id: bom_id.clone(),
            bom_number: bom_number.to_string(),
            item_id: item_id.to_string(),
            name: name.to_string(),
            quantity,
            routing_notes: routing_notes.map(String::from),
            is_active: true,
            components: bom_components,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "BomCreated",
            "BillOfMaterials",
            &bom_id,
            serde_json::to_value(&bom).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(bom)
    }

    /// Create a new Work Order / Production Run
    pub fn create_work_order(
        ctx: &ModuleContext,
        work_order_number: &str,
        bom_id: &str,
        target_quantity: Decimal,
        source_location_id: &str,
        destination_location_id: &str,
    ) -> Result<WorkOrder, ManufacturingError> {
        if target_quantity <= Decimal::ZERO {
            return Err(ManufacturingError::InvalidConfiguration(
                "Work order target quantity must be greater than zero".into(),
            ));
        }

        let (item_id, bom_exists) = {
            let conn = ctx.db.conn();
            let mut stmt = conn.prepare("SELECT item_id FROM boms WHERE id = ?1")?;
            let i_id: String = stmt
                .query_row(params![bom_id], |r| r.get(0))
                .map_err(|_| ManufacturingError::BomNotFound(bom_id.to_string()))?;
            (i_id, true)
        };

        if !bom_exists {
            return Err(ManufacturingError::BomNotFound(bom_id.to_string()));
        }

        let wo_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO work_orders (
                    id, work_order_number, bom_id, item_id, target_quantity,
                    source_location_id, destination_location_id, status, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    wo_id,
                    work_order_number,
                    bom_id,
                    item_id,
                    target_quantity.to_string(),
                    source_location_id,
                    destination_location_id,
                    WorkOrderStatus::Draft.as_str(),
                    now.to_rfc3339()
                ],
            )?;
        }

        let wo = WorkOrder {
            id: wo_id.clone(),
            work_order_number: work_order_number.to_string(),
            bom_id: bom_id.to_string(),
            item_id,
            target_quantity,
            source_location_id: source_location_id.to_string(),
            destination_location_id: destination_location_id.to_string(),
            status: WorkOrderStatus::Draft,
            created_at: now,
            completed_at: None,
        };

        let _ = ctx.record_and_publish_event(
            "WorkOrderCreated",
            "WorkOrder",
            &wo_id,
            serde_json::to_value(&wo).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(wo)
    }

    /// Execute a production run:
    /// 1. Consumes raw components from source_location_id (double-entry `Pick` stock movement).
    /// 2. Produces finished good to destination_location_id (double-entry `Receive` stock movement) with assigned manufactured lot number.
    /// 3. Records end-to-end component genealogy linking raw component lots to the finished product lot.
    /// 4. Marks work order as Completed.
    pub fn execute_work_order(
        ctx: &ModuleContext,
        work_order_id: &str,
    ) -> Result<WorkOrder, ManufacturingError> {
        let (wo, bom) = {
            let conn = ctx.db.conn();
            let w = Self::get_work_order_by_id(&conn, work_order_id)?;
            let b = Self::get_bom_by_id(&conn, &w.bom_id)?;
            (w, b)
        };

        if wo.status == WorkOrderStatus::Completed {
            return Err(ManufacturingError::InvalidStatus(
                "Work order is already completed".into(),
            ));
        }

        let ref_note = format!("Work Order: {}", wo.work_order_number);
        let finished_lot = format!("LOT-MFG-{}", wo.work_order_number);
        let now = Utc::now();

        // 1. Consume all required raw materials from source_location_id and track genealogy
        for comp in &bom.components {
            let unit_ratio = comp.quantity / bom.quantity;
            let scrap_multiplier = Decimal::ONE + comp.scrap_percentage;
            let req_qty = wo.target_quantity * unit_ratio * scrap_multiplier;

            // Lookup existing component lot from source location if available
            let comp_lot: Option<String> = {
                let conn = ctx.db.conn();
                conn.query_row(
                    "SELECT lot_number FROM stock_quants WHERE item_id = ?1 AND location_id = ?2 AND lot_number IS NOT NULL AND CAST(quantity AS REAL) > 0 LIMIT 1",
                    params![comp.item_id, wo.source_location_id],
                    |r| r.get(0),
                ).ok()
            };

            StockService::record_movement(
                ctx,
                MovementType::Pick,
                &comp.item_id,
                Some(&wo.source_location_id),
                None,
                req_qty,
                comp_lot.as_deref(),
                None,
                Some(&wo.work_order_number),
                Some(&ref_note),
                "manufacturing",
            )?;

            // Record genealogy link
            {
                let conn = ctx.db.conn();
                let gen_id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    r#"
                    INSERT INTO work_order_genealogy (
                        id, work_order_id, finished_item_id, finished_lot_number,
                        component_item_id, component_lot_number, component_quantity, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    "#,
                    params![
                        gen_id,
                        work_order_id,
                        bom.item_id,
                        finished_lot,
                        comp.item_id,
                        comp_lot,
                        req_qty.to_string(),
                        now.to_rfc3339(),
                    ],
                )?;
            }
        }

        // 2. Receive finished goods into destination_location_id with assigned manufactured lot
        StockService::record_movement(
            ctx,
            MovementType::Receive,
            &bom.item_id,
            None,
            Some(&wo.destination_location_id),
            wo.target_quantity,
            Some(&finished_lot),
            None,
            Some(&wo.work_order_number),
            Some(&format!("Manufactured output from {}", wo.work_order_number)),
            "manufacturing",
        )?;

        // Register the new manufactured lot in mod-inventory
        {
            let conn = ctx.db.conn();
            let exp_date = (now + chrono::Duration::days(365 * 2)).format("%Y-%m-%d").to_string();
            let _ = StockService::create_or_update_lot(
                &conn,
                &finished_lot,
                &bom.item_id,
                Some(&exp_date),
                Some("released"),
                Some(&wo.work_order_number),
            );
        }

        // 3. Mark work order completed
        {
            let conn = ctx.db.conn();
            conn.execute(
                "UPDATE work_orders SET status = ?1, completed_at = ?2 WHERE id = ?3",
                params![
                    WorkOrderStatus::Completed.as_str(),
                    now.to_rfc3339(),
                    work_order_id
                ],
            )?;
        }

        let mut completed_wo = wo;
        completed_wo.status = WorkOrderStatus::Completed;
        completed_wo.completed_at = Some(now);

        let _ = ctx.record_and_publish_event(
            "WorkOrderCompleted",
            "WorkOrder",
            work_order_id,
            serde_json::to_value(&completed_wo).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(completed_wo)
    }

    /// Update work order status (e.g. Planned -> In Progress -> Quality Check -> Completed)
    pub fn update_work_order_status(
        ctx: &ModuleContext,
        work_order_id: &str,
        new_status: &str,
    ) -> Result<WorkOrder, ManufacturingError> {
        let status = WorkOrderStatus::from_str(new_status)
            .map_err(|_| ManufacturingError::InvalidStatus(format!("Invalid status: {}", new_status)))?;

        // If advancing to Completed, trigger execute_work_order
        if status == WorkOrderStatus::Completed {
            return Self::execute_work_order(ctx, work_order_id);
        }

        let mut wo = {
            let conn = ctx.db.conn();
            Self::get_work_order_by_id(&conn, work_order_id)?
        };

        {
            let conn = ctx.db.conn();
            conn.execute(
                "UPDATE work_orders SET status = ?1 WHERE id = ?2",
                params![status.as_str(), work_order_id],
            )?;
        }

        wo.status = status;

        let _ = ctx.record_and_publish_event(
            "WorkOrderStatusUpdated",
            "WorkOrder",
            work_order_id,
            serde_json::to_value(&wo).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(wo)
    }

    /// Retrieve component genealogy tree for a finished work order
    pub fn get_work_order_genealogy(
        conn: &Connection,
        work_order_id: &str,
    ) -> Result<Option<WorkOrderGenealogyTree>, ManufacturingError> {
        let wo_result = Self::get_work_order_by_id(conn, work_order_id);
        let wo = match wo_result {
            Ok(w) => w,
            Err(_) => {
                // Try finding by work_order_number or finished_lot_number
                let mut stmt = conn.prepare(
                    "SELECT id FROM work_orders WHERE work_order_number = ?1 OR id IN (SELECT work_order_id FROM work_order_genealogy WHERE finished_lot_number = ?1)"
                )?;
                let mut rows = stmt.query(params![work_order_id])?;
                if let Some(r) = rows.next()? {
                    let id: String = r.get(0)?;
                    Self::get_work_order_by_id(conn, &id)?
                } else {
                    return Ok(None);
                }
            }
        };

        // Query finished item details
        let (finished_sku, finished_name): (String, String) = conn.query_row(
            "SELECT sku, name FROM inventory_items WHERE id = ?1",
            params![wo.item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap_or_else(|_| ("UNKNOWN".into(), "Unknown Item".into()));

        let finished_lot = format!("LOT-MFG-{}", wo.work_order_number);

        // Query components and their lot metadata
        let mut stmt = conn.prepare(
            r#"
            SELECT
                g.component_item_id,
                i.sku,
                i.name,
                g.component_lot_number,
                g.component_quantity,
                lt.expiration_date,
                lt.quarantine_status,
                lt.manufacturer_batch
            FROM work_order_genealogy g
            INNER JOIN inventory_items i ON g.component_item_id = i.id
            LEFT JOIN inventory_lots lt ON g.component_lot_number = lt.lot_number
            WHERE g.work_order_id = ?1
            ORDER BY i.name ASC
            "#,
        )?;

        let rows = stmt.query_map(params![wo.id], |row| {
            let component_item_id: String = row.get(0)?;
            let component_sku: String = row.get(1)?;
            let component_name: String = row.get(2)?;
            let component_lot_number: Option<String> = row.get(3)?;
            let qty_str: String = row.get(4)?;
            let component_quantity = Decimal::from_str(&qty_str).unwrap_or(Decimal::ZERO);
            let expiration_date: Option<String> = row.get(5)?;
            let quarantine_status: Option<String> = row.get(6)?;
            let manufacturer_batch: Option<String> = row.get(7)?;

            Ok(ComponentGenealogyEntry {
                component_item_id,
                component_sku,
                component_name,
                component_lot_number,
                component_quantity,
                expiration_date,
                quarantine_status,
                manufacturer_batch,
            })
        })?;

        let mut components = Vec::new();
        for r in rows {
            components.push(r?);
        }

        // If no genealogy entries recorded yet (e.g. before execution), populate planned components from BOM
        if components.is_empty() {
            if let Ok(bom) = Self::get_bom_by_id(conn, &wo.bom_id) {
                for c in bom.components {
                    let (sku, name): (String, String) = conn.query_row(
                        "SELECT sku, name FROM inventory_items WHERE id = ?1",
                        params![c.item_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    ).unwrap_or_else(|_| ("UNKNOWN".into(), "Unknown Component".into()));

                    components.push(ComponentGenealogyEntry {
                        component_item_id: c.item_id,
                        component_sku: sku,
                        component_name: name,
                        component_lot_number: Some("PLANNED-LOT".into()),
                        component_quantity: c.quantity * wo.target_quantity,
                        expiration_date: Some("2028-12-31".into()),
                        quarantine_status: Some("released".into()),
                        manufacturer_batch: Some("BATCH-STD".into()),
                    });
                }
            }
        }

        Ok(Some(WorkOrderGenealogyTree {
            work_order_id: wo.id,
            work_order_number: wo.work_order_number,
            finished_item_id: wo.item_id,
            finished_sku,
            finished_name,
            finished_lot_number: finished_lot,
            quantity: wo.target_quantity,
            completed_at: wo.completed_at,
            components,
        }))
    }

    pub fn get_bom_by_id(conn: &Connection, id: &str) -> Result<BillOfMaterials, ManufacturingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, bom_number, item_id, name, quantity, routing_notes, is_active, created_at
            FROM boms
            WHERE id = ?1
            "#,
        )?;

        let mut bom = stmt
            .query_row(params![id], Self::map_bom_row)
            .map_err(|_| ManufacturingError::BomNotFound(id.to_string()))?;

        let mut comp_stmt = conn.prepare(
            r#"
            SELECT id, bom_id, item_id, quantity, scrap_percentage, notes
            FROM bom_components
            WHERE bom_id = ?1
            "#,
        )?;

        let comps = comp_stmt
            .query_map(params![id], Self::map_comp_row)?
            .collect::<Result<Vec<_>, _>>()?;

        bom.components = comps;
        Ok(bom)
    }

    pub fn list_boms(conn: &Connection) -> Result<Vec<BillOfMaterials>, ManufacturingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, bom_number, item_id, name, quantity, routing_notes, is_active, created_at
            FROM boms
            WHERE is_active = 1
            ORDER BY bom_number ASC
            "#,
        )?;

        let rows = stmt.query_map([], Self::map_bom_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_work_order_by_id(
        conn: &Connection,
        id: &str,
    ) -> Result<WorkOrder, ManufacturingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, work_order_number, bom_id, item_id, target_quantity,
                   source_location_id, destination_location_id, status, created_at, completed_at
            FROM work_orders
            WHERE id = ?1
            "#,
        )?;

        stmt.query_row(params![id], Self::map_wo_row)
            .map_err(|_| ManufacturingError::WorkOrderNotFound(id.to_string()))
    }

    pub fn list_work_orders(conn: &Connection) -> Result<Vec<WorkOrder>, ManufacturingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, work_order_number, bom_id, item_id, target_quantity,
                   source_location_id, destination_location_id, status, created_at, completed_at
            FROM work_orders
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], Self::map_wo_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    fn map_bom_row(row: &Row<'_>) -> Result<BillOfMaterials, rusqlite::Error> {
        let id: String = row.get(0)?;
        let bom_number: String = row.get(1)?;
        let item_id: String = row.get(2)?;
        let name: String = row.get(3)?;
        let quantity_str: String = row.get(4)?;
        let routing_notes: Option<String> = row.get(5)?;
        let is_active_int: i32 = row.get(6)?;
        let created_at_str: String = row.get(7)?;

        let quantity = Decimal::from_str(&quantity_str).unwrap_or(Decimal::ONE);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(BillOfMaterials {
            id,
            bom_number,
            item_id,
            name,
            quantity,
            routing_notes,
            is_active: is_active_int != 0,
            components: Vec::new(),
            created_at,
        })
    }

    fn map_comp_row(row: &Row<'_>) -> Result<BomComponent, rusqlite::Error> {
        let id: String = row.get(0)?;
        let bom_id: String = row.get(1)?;
        let item_id: String = row.get(2)?;
        let quantity_str: String = row.get(3)?;
        let scrap_percentage_str: String = row.get(4)?;
        let notes: Option<String> = row.get(5)?;

        let quantity = Decimal::from_str(&quantity_str).unwrap_or(Decimal::ZERO);
        let scrap_percentage = Decimal::from_str(&scrap_percentage_str).unwrap_or(Decimal::ZERO);

        Ok(BomComponent {
            id,
            bom_id,
            item_id,
            quantity,
            scrap_percentage,
            notes,
        })
    }

    fn map_wo_row(row: &Row<'_>) -> Result<WorkOrder, rusqlite::Error> {
        let id: String = row.get(0)?;
        let work_order_number: String = row.get(1)?;
        let bom_id: String = row.get(2)?;
        let item_id: String = row.get(3)?;
        let target_quantity_str: String = row.get(4)?;
        let source_location_id: String = row.get(5)?;
        let destination_location_id: String = row.get(6)?;
        let status_str: String = row.get(7)?;
        let created_at_str: String = row.get(8)?;
        let completed_at_str: Option<String> = row.get(9)?;

        let target_quantity = Decimal::from_str(&target_quantity_str).unwrap_or(Decimal::ZERO);
        let status = WorkOrderStatus::from_str(&status_str).unwrap_or(WorkOrderStatus::Draft);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let completed_at = completed_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Ok(WorkOrder {
            id,
            work_order_number,
            bom_id,
            item_id,
            target_quantity,
            source_location_id,
            destination_location_id,
            status,
            created_at,
            completed_at,
        })
    }
}
