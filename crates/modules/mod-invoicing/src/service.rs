use chrono::Utc;
use erp_kernel::ModuleContext;
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::models::{Invoice, InvoiceLineItem, InvoiceStatus};

pub struct InvoicingService;

impl InvoicingService {
    pub fn create_invoice(
        ctx: &ModuleContext,
        invoice_number: &str,
        customer_id: &str,
        line_inputs: Vec<(Option<String>, String, Decimal, Decimal, Decimal)>, // (item_id, desc, qty, unit_price, tax_rate)
        notes: Option<&str>,
    ) -> Result<Invoice, erp_kernel::ErpKernelError> {
        let invoice_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let mut subtotal = Decimal::ZERO;
        let mut tax_total = Decimal::ZERO;
        let mut line_items = Vec::new();

        for (item_id, desc, qty, unit_price, tax_rate) in line_inputs {
            let line_subtotal = qty * unit_price;
            let line_tax = line_subtotal * tax_rate;
            let line_total = line_subtotal + line_tax;

            subtotal += line_subtotal;
            tax_total += line_tax;

            line_items.push(InvoiceLineItem {
                id: Uuid::new_v4().to_string(),
                item_id,
                description: desc,
                quantity: qty,
                unit_price,
                tax_rate,
                line_total,
            });
        }

        let grand_total = subtotal + tax_total;

        let mut conn = ctx.db.conn();
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO invoices (
                id, invoice_number, customer_id, status, subtotal, tax_total, grand_total, notes, created_at
            ) VALUES (?1, ?2, ?3, 'Draft', ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                invoice_id,
                invoice_number,
                customer_id,
                subtotal.to_string(),
                tax_total.to_string(),
                grand_total.to_string(),
                notes,
                now.to_rfc3339()
            ],
        )?;

        for line in &line_items {
            tx.execute(
                r#"
                INSERT INTO invoice_lines (
                    id, invoice_id, item_id, description, quantity, unit_price, tax_rate, line_total
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    line.id,
                    invoice_id,
                    line.item_id,
                    line.description,
                    line.quantity.to_string(),
                    line.unit_price.to_string(),
                    line.tax_rate.to_string(),
                    line.line_total.to_string()
                ],
            )?;
        }

        tx.commit()?;
        drop(conn);

        let invoice = Invoice {
            id: invoice_id.clone(),
            invoice_number: invoice_number.to_string(),
            customer_id: customer_id.to_string(),
            status: InvoiceStatus::Draft,
            subtotal,
            tax_total,
            grand_total,
            notes: notes.map(String::from),
            created_at: now,
            line_items,
        };

        let _ = ctx.record_and_publish_event(
            "InvoiceCreated",
            "Invoice",
            &invoice_id,
            serde_json::to_value(&invoice).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(invoice)
    }

    /// Finalize an invoice and broadcast the `InvoiceFinalized` event,
    /// triggering other modules (such as `mod-inventory`) to react and deduct stock.
    pub fn finalize_invoice(
        ctx: &ModuleContext,
        invoice_id: &str,
        actor_id: &str,
    ) -> Result<Invoice, erp_kernel::ErpKernelError> {
        let mut invoice = Self::get_invoice(&ctx.db.conn(), invoice_id)?;

        if invoice.status != InvoiceStatus::Draft {
            return Err(erp_kernel::ErpKernelError::Internal(format!(
                "Cannot finalize invoice in status '{:?}'",
                invoice.status
            )));
        }

        {
            let conn = ctx.db.conn();
            conn.execute(
                "UPDATE invoices SET status = 'Finalized' WHERE id = ?1",
                params![invoice_id],
            )?;
        }

        invoice.status = InvoiceStatus::Finalized;

        // Broadcast InvoiceFinalized domain event with line items
        let _ = ctx.record_and_publish_event(
            "InvoiceFinalized",
            "Invoice",
            invoice_id,
            serde_json::json!({
                "id": invoice_id,
                "invoice_id": invoice_id,
                "invoice_number": invoice.invoice_number,
                "customer_id": invoice.customer_id,
                "subtotal": invoice.subtotal.to_string(),
                "tax_total": invoice.tax_total.to_string(),
                "grand_total": invoice.grand_total.to_string(),
                "line_items": invoice.line_items.iter().map(|li| {
                    serde_json::json!({
                        "item_id": li.item_id,
                        "description": li.description,
                        "quantity": li.quantity.to_string(),
                        "unit_price": li.unit_price.to_string(),
                    })
                }).collect::<Vec<_>>()
            }),
            actor_id,
        )?;

        Ok(invoice)
    }

    pub fn get_invoice(conn: &Connection, invoice_id: &str) -> Result<Invoice, erp_kernel::ErpKernelError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, invoice_number, customer_id, status, subtotal, tax_total, grand_total, notes, created_at
            FROM invoices WHERE id = ?1
            "#,
        )?;

        let mut invoice = stmt
            .query_row(params![invoice_id], Self::map_invoice_row)
            .map_err(|e| erp_kernel::ErpKernelError::Internal(format!("Invoice not found: {}", e)))?;

        let mut lines_stmt = conn.prepare(
            r#"
            SELECT id, item_id, description, quantity, unit_price, tax_rate, line_total
            FROM invoice_lines WHERE invoice_id = ?1
            "#,
        )?;

        let line_rows = lines_stmt.query_map(params![invoice_id], |r| {
            let id: String = r.get(0)?;
            let item_id: Option<String> = r.get(1)?;
            let description: String = r.get(2)?;
            let q_str: String = r.get(3)?;
            let up_str: String = r.get(4)?;
            let tr_str: String = r.get(5)?;
            let lt_str: String = r.get(6)?;

            Ok(InvoiceLineItem {
                id,
                item_id,
                description,
                quantity: Decimal::from_str(&q_str).unwrap_or(Decimal::ZERO),
                unit_price: Decimal::from_str(&up_str).unwrap_or(Decimal::ZERO),
                tax_rate: Decimal::from_str(&tr_str).unwrap_or(Decimal::ZERO),
                line_total: Decimal::from_str(&lt_str).unwrap_or(Decimal::ZERO),
            })
        })?;

        let mut lines = Vec::new();
        for lr in line_rows {
            lines.push(lr?);
        }

        invoice.line_items = lines;
        Ok(invoice)
    }

    pub fn list_invoices(conn: &Connection) -> Result<Vec<Invoice>, erp_kernel::ErpKernelError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, invoice_number, customer_id, status, subtotal, tax_total, grand_total, notes, created_at
            FROM invoices ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map([], Self::map_invoice_row)?;
        let mut invoices = Vec::new();
        for r in rows {
            let mut inv = r?;
            let mut lines_stmt = conn.prepare(
                r#"
                SELECT id, item_id, description, quantity, unit_price, tax_rate, line_total
                FROM invoice_lines WHERE invoice_id = ?1
                "#,
            )?;
            let line_rows = lines_stmt.query_map(params![&inv.id], |lr| {
                let id: String = lr.get(0)?;
                let item_id: Option<String> = lr.get(1)?;
                let description: String = lr.get(2)?;
                let q_str: String = lr.get(3)?;
                let up_str: String = lr.get(4)?;
                let tr_str: String = lr.get(5)?;
                let lt_str: String = lr.get(6)?;

                Ok(InvoiceLineItem {
                    id,
                    item_id,
                    description,
                    quantity: Decimal::from_str(&q_str).unwrap_or(Decimal::ZERO),
                    unit_price: Decimal::from_str(&up_str).unwrap_or(Decimal::ZERO),
                    tax_rate: Decimal::from_str(&tr_str).unwrap_or(Decimal::ZERO),
                    line_total: Decimal::from_str(&lt_str).unwrap_or(Decimal::ZERO),
                })
            })?;
            for l in line_rows {
                inv.line_items.push(l?);
            }
            invoices.push(inv);
        }
        Ok(invoices)
    }

    fn map_invoice_row(row: &Row<'_>) -> Result<Invoice, rusqlite::Error> {
        let id: String = row.get(0)?;
        let invoice_number: String = row.get(1)?;
        let customer_id: String = row.get(2)?;
        let status_str: String = row.get(3)?;
        let subtotal_str: String = row.get(4)?;
        let tax_total_str: String = row.get(5)?;
        let grand_total_str: String = row.get(6)?;
        let notes: Option<String> = row.get(7)?;
        let created_at_str: String = row.get(8)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Invoice {
            id,
            invoice_number,
            customer_id,
            status: InvoiceStatus::from_str(&status_str).unwrap_or(InvoiceStatus::Draft),
            subtotal: Decimal::from_str(&subtotal_str).unwrap_or(Decimal::ZERO),
            tax_total: Decimal::from_str(&tax_total_str).unwrap_or(Decimal::ZERO),
            grand_total: Decimal::from_str(&grand_total_str).unwrap_or(Decimal::ZERO),
            notes,
            created_at,
            line_items: Vec::new(),
        })
    }
}
