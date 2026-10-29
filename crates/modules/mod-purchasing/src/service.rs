use chrono::Utc;
use erp_kernel::ModuleContext;
use mod_inventory::{MovementType, StockService};
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::PurchasingError;
use crate::models::{
    PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus, PurchaseReceipt, PurchaseReceiptLine,
};

pub struct PurchasingService;

impl PurchasingService {
    /// Create a new Purchase Order in Draft status
    #[allow(clippy::too_many_arguments)]
    pub fn create_purchase_order(
        ctx: &ModuleContext,
        po_number: &str,
        supplier_id: &str,
        expected_delivery: Option<&str>,
        lines: Vec<(String, String, Decimal, Decimal, Decimal)>, // (item_id, description, qty, price, tax_rate)
        notes: Option<&str>,
    ) -> Result<PurchaseOrder, PurchasingError> {
        if lines.is_empty() {
            return Err(PurchasingError::InvalidLine(
                "Purchase order must contain at least one line item".into(),
            ));
        }

        let po_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let order_date = now.format("%Y-%m-%d").to_string();

        let mut subtotal = Decimal::ZERO;
        let mut tax_total = Decimal::ZERO;
        let mut order_lines = Vec::new();

        for (item_id, desc, qty, price, tax_rate) in lines {
            if qty <= Decimal::ZERO {
                return Err(PurchasingError::InvalidLine(
                    "Order quantity must be greater than zero".into(),
                ));
            }

            let line_total = qty * price;
            let line_tax = line_total * tax_rate;

            subtotal += line_total;
            tax_total += line_tax;

            order_lines.push(PurchaseOrderLine {
                id: Uuid::new_v4().to_string(),
                purchase_order_id: po_id.clone(),
                item_id,
                description: desc,
                quantity: qty,
                received_quantity: Decimal::ZERO,
                unit_price: price,
                tax_rate,
                line_total,
            });
        }

        let grand_total = subtotal + tax_total;

        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO purchase_orders (
                id, po_number, supplier_id, order_date, expected_delivery, status,
                subtotal, tax_total, grand_total, notes, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                po_id,
                po_number,
                supplier_id,
                order_date,
                expected_delivery,
                PurchaseOrderStatus::Draft.as_str(),
                subtotal.to_string(),
                tax_total.to_string(),
                grand_total.to_string(),
                notes,
                now.to_rfc3339()
            ],
        )?;

        let mut line_stmt = tx.prepare(
            r#"
            INSERT INTO purchase_order_lines (
                id, purchase_order_id, item_id, description, quantity,
                received_quantity, unit_price, tax_rate, line_total
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )?;

        for line in &order_lines {
            line_stmt.execute(params![
                line.id,
                line.purchase_order_id,
                line.item_id,
                line.description,
                line.quantity.to_string(),
                line.received_quantity.to_string(),
                line.unit_price.to_string(),
                line.tax_rate.to_string(),
                line.line_total.to_string()
            ])?;
        }
        drop(line_stmt);

        tx.commit()?;
        drop(conn);

        let po = PurchaseOrder {
            id: po_id.clone(),
            po_number: po_number.to_string(),
            supplier_id: supplier_id.to_string(),
            order_date,
            expected_delivery: expected_delivery.map(String::from),
            status: PurchaseOrderStatus::Draft,
            subtotal,
            tax_total,
            grand_total,
            notes: notes.map(String::from),
            lines: order_lines,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "PurchaseOrderCreated",
            "PurchaseOrder",
            &po_id,
            serde_json::to_value(&po).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(po)
    }

    /// Submit a purchase order, moving it from Draft to Submitted
    pub fn submit_purchase_order(
        ctx: &ModuleContext,
        po_id: &str,
    ) -> Result<PurchaseOrder, PurchasingError> {
        let po = {
            let conn = ctx.db.conn();
            let p = Self::get_purchase_order_by_id(&conn, po_id)?;

            if p.status != PurchaseOrderStatus::Draft {
                return Err(PurchasingError::InvalidStatus(format!(
                    "Cannot submit purchase order in {:?} status",
                    p.status
                )));
            }

            conn.execute(
                "UPDATE purchase_orders SET status = ?1 WHERE id = ?2",
                params![PurchaseOrderStatus::Submitted.as_str(), po_id],
            )?;
            p
        };

        let mut updated = po;
        updated.status = PurchaseOrderStatus::Submitted;

        let _ = ctx.record_and_publish_event(
            "PurchaseOrderSubmitted",
            "PurchaseOrder",
            po_id,
            serde_json::to_value(&updated).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(updated)
    }

    /// Receive goods against a purchase order and put them directly into inventory locations.
    /// Records an immutable double-entry stock movement (`Receive`) for each received line item.
    pub fn receive_goods(
        ctx: &ModuleContext,
        po_id: Option<&str>,
        supplier_id: &str,
        destination_location_id: &str,
        items: Vec<(String, Decimal, Option<String>, Option<String>)>, // (item_id, qty, lot, serial)
        notes: Option<&str>,
    ) -> Result<PurchaseReceipt, PurchasingError> {
        if items.is_empty() {
            return Err(PurchasingError::InvalidLine(
                "Purchase receipt must contain at least one item".into(),
            ));
        }

        let receipt_id = Uuid::new_v4().to_string();
        let receipt_number = format!("REC-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
        let now = Utc::now();

        let mut receipt_lines = Vec::new();

        // 1. Record stock movements into destination warehouse location
        for (item_id, qty, ref lot, ref serial) in &items {
            StockService::record_movement(
                ctx,
                MovementType::Receive,
                item_id,
                None,
                Some(destination_location_id),
                *qty,
                lot.as_deref(),
                serial.as_deref(),
                Some(&receipt_number),
                notes,
                "purchasing",
            )?;

            receipt_lines.push(PurchaseReceiptLine {
                id: Uuid::new_v4().to_string(),
                receipt_id: receipt_id.clone(),
                item_id: item_id.clone(),
                quantity: *qty,
                lot_number: lot.clone(),
                serial_number: serial.clone(),
            });
        }

        // 2. Persist receipt records and update PO status if linked
        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO purchase_receipts (
                id, receipt_number, purchase_order_id, supplier_id,
                destination_location_id, received_at, notes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                receipt_id,
                receipt_number,
                po_id,
                supplier_id,
                destination_location_id,
                now.to_rfc3339(),
                notes
            ],
        )?;

        let mut line_stmt = tx.prepare(
            r#"
            INSERT INTO purchase_receipt_lines (
                id, receipt_id, item_id, quantity, lot_number, serial_number
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;

        for rl in &receipt_lines {
            line_stmt.execute(params![
                rl.id,
                rl.receipt_id,
                rl.item_id,
                rl.quantity.to_string(),
                rl.lot_number,
                rl.serial_number
            ])?;
        }
        drop(line_stmt);

        // Update purchase order line quantities and status
        if let Some(poid) = po_id {
            for (item_id, qty, _, _) in &items {
                tx.execute(
                    r#"
                    UPDATE purchase_order_lines
                    SET received_quantity = CAST((CAST(received_quantity AS REAL) + ?1) AS TEXT)
                    WHERE purchase_order_id = ?2 AND item_id = ?3
                    "#,
                    params![qty.to_string(), poid, item_id],
                )?;
            }

            // Check if PO is completely received
            let unreceived_count: i64 = tx.query_row(
                r#"
                SELECT COUNT(*) FROM purchase_order_lines
                WHERE purchase_order_id = ?1
                  AND CAST(received_quantity AS REAL) < CAST(quantity AS REAL)
                "#,
                params![poid],
                |r| r.get(0),
            )?;

            let new_status = if unreceived_count == 0 {
                PurchaseOrderStatus::Received
            } else {
                PurchaseOrderStatus::PartiallyReceived
            };

            tx.execute(
                "UPDATE purchase_orders SET status = ?1 WHERE id = ?2",
                params![new_status.as_str(), poid],
            )?;
        }

        tx.commit()?;
        drop(conn);

        let receipt = PurchaseReceipt {
            id: receipt_id.clone(),
            receipt_number,
            purchase_order_id: po_id.map(String::from),
            supplier_id: supplier_id.to_string(),
            destination_location_id: destination_location_id.to_string(),
            received_at: now,
            notes: notes.map(String::from),
            lines: receipt_lines,
        };

        let _ = ctx.record_and_publish_event(
            "GoodsReceived",
            "PurchaseReceipt",
            &receipt_id,
            serde_json::to_value(&receipt).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(receipt)
    }

    pub fn get_purchase_order_by_id(
        conn: &Connection,
        id: &str,
    ) -> Result<PurchaseOrder, PurchasingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, po_number, supplier_id, order_date, expected_delivery,
                   status, subtotal, tax_total, grand_total, notes, created_at
            FROM purchase_orders
            WHERE id = ?1
            "#,
        )?;

        let mut po = stmt
            .query_row(params![id], Self::map_po_row)
            .map_err(|_| PurchasingError::PurchaseOrderNotFound(id.to_string()))?;

        let mut line_stmt = conn.prepare(
            r#"
            SELECT id, purchase_order_id, item_id, description, quantity,
                   received_quantity, unit_price, tax_rate, line_total
            FROM purchase_order_lines
            WHERE purchase_order_id = ?1
            "#,
        )?;

        let lines = line_stmt
            .query_map(params![id], Self::map_line_row)?
            .collect::<Result<Vec<_>, _>>()?;

        po.lines = lines;
        Ok(po)
    }

    pub fn list_purchase_orders(conn: &Connection) -> Result<Vec<PurchaseOrder>, PurchasingError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, po_number, supplier_id, order_date, expected_delivery,
                   status, subtotal, tax_total, grand_total, notes, created_at
            FROM purchase_orders
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], Self::map_po_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    fn map_po_row(row: &Row<'_>) -> Result<PurchaseOrder, rusqlite::Error> {
        let id: String = row.get(0)?;
        let po_number: String = row.get(1)?;
        let supplier_id: String = row.get(2)?;
        let order_date: String = row.get(3)?;
        let expected_delivery: Option<String> = row.get(4)?;
        let status_str: String = row.get(5)?;
        let subtotal_str: String = row.get(6)?;
        let tax_total_str: String = row.get(7)?;
        let grand_total_str: String = row.get(8)?;
        let notes: Option<String> = row.get(9)?;
        let created_at_str: String = row.get(10)?;

        let status = PurchaseOrderStatus::from_str(&status_str).unwrap_or(PurchaseOrderStatus::Draft);
        let subtotal = Decimal::from_str(&subtotal_str).unwrap_or(Decimal::ZERO);
        let tax_total = Decimal::from_str(&tax_total_str).unwrap_or(Decimal::ZERO);
        let grand_total = Decimal::from_str(&grand_total_str).unwrap_or(Decimal::ZERO);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(PurchaseOrder {
            id,
            po_number,
            supplier_id,
            order_date,
            expected_delivery,
            status,
            subtotal,
            tax_total,
            grand_total,
            notes,
            lines: Vec::new(),
            created_at,
        })
    }

    fn map_line_row(row: &Row<'_>) -> Result<PurchaseOrderLine, rusqlite::Error> {
        let id: String = row.get(0)?;
        let purchase_order_id: String = row.get(1)?;
        let item_id: String = row.get(2)?;
        let description: String = row.get(3)?;
        let quantity_str: String = row.get(4)?;
        let received_quantity_str: String = row.get(5)?;
        let unit_price_str: String = row.get(6)?;
        let tax_rate_str: String = row.get(7)?;
        let line_total_str: String = row.get(8)?;

        let quantity = Decimal::from_str(&quantity_str).unwrap_or(Decimal::ZERO);
        let received_quantity = Decimal::from_str(&received_quantity_str).unwrap_or(Decimal::ZERO);
        let unit_price = Decimal::from_str(&unit_price_str).unwrap_or(Decimal::ZERO);
        let tax_rate = Decimal::from_str(&tax_rate_str).unwrap_or(Decimal::ZERO);
        let line_total = Decimal::from_str(&line_total_str).unwrap_or(Decimal::ZERO);

        Ok(PurchaseOrderLine {
            id,
            purchase_order_id,
            item_id,
            description,
            quantity,
            received_quantity,
            unit_price,
            tax_rate,
            line_total,
        })
    }
}
