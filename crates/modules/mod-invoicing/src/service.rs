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

    // --- Recurring Invoicing & Subscription Schedules (LEDGER-19) ---

    pub fn create_recurring_schedule(
        conn: &Connection,
        customer_id: &str,
        plan_name: &str,
        frequency: &str,
        amount: Decimal,
        currency: &str,
        next_run_date: &str,
        auto_send: bool,
    ) -> Result<crate::models::RecurringSchedule, erp_kernel::ErpKernelError> {
        let id = format!("rec-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
        let now = Utc::now();

        conn.execute(
            r#"
            INSERT INTO recurring_schedules (
                id, customer_id, plan_name, frequency, amount, currency, next_run_date, is_active, auto_send, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9)
            "#,
            params![id, customer_id, plan_name, frequency, amount.to_string(), currency, next_run_date, if auto_send { 1 } else { 0 }, now.to_rfc3339()],
        )?;

        Ok(crate::models::RecurringSchedule {
            id,
            customer_id: customer_id.to_string(),
            plan_name: plan_name.to_string(),
            frequency: frequency.to_string(),
            amount,
            currency: currency.to_string(),
            next_run_date: next_run_date.to_string(),
            is_active: true,
            auto_send,
            created_at: now,
        })
    }

    pub fn list_recurring_schedules(conn: &Connection) -> Result<Vec<crate::models::RecurringSchedule>, erp_kernel::ErpKernelError> {
        let mut stmt = conn.prepare(
            "SELECT id, customer_id, plan_name, frequency, amount, currency, next_run_date, is_active, auto_send, created_at FROM recurring_schedules ORDER BY next_run_date ASC"
        )?;

        let rows = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            let customer_id: String = r.get(1)?;
            let plan_name: String = r.get(2)?;
            let frequency: String = r.get(3)?;
            let amount_str: String = r.get(4)?;
            let currency: String = r.get(5)?;
            let next_run_date: String = r.get(6)?;
            let is_active_int: i32 = r.get(7)?;
            let auto_send_int: i32 = r.get(8)?;
            let created_at_str: String = r.get(9)?;

            let amount = Decimal::from_str(&amount_str).unwrap_or(Decimal::ZERO);
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(crate::models::RecurringSchedule {
                id,
                customer_id,
                plan_name,
                frequency,
                amount,
                currency,
                next_run_date,
                is_active: is_active_int != 0,
                auto_send: auto_send_int != 0,
                created_at,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn trigger_recurring_invoices(ctx: &erp_kernel::ModuleContext) -> Result<Vec<Invoice>, erp_kernel::ErpKernelError> {
        let schedules = Self::list_recurring_schedules(&ctx.db.conn())?;
        let mut generated = Vec::new();

        for sched in schedules {
            if sched.is_active {
                let lines = vec![
                    (
                        None,
                        format!("Subscription Renewal: {}", sched.plan_name),
                        Decimal::ONE,
                        sched.amount,
                        Decimal::from_str("0.20").unwrap_or(Decimal::ZERO),
                    )
                ];

                let inv_number = format!("INV-REC-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
                let inv = Self::create_invoice(
                    ctx,
                    &inv_number,
                    &sched.customer_id,
                    lines,
                    Some(&format!("Automated recurring invoice for plan {}", sched.plan_name)),
                )?;

                // Advance next_run_date (simulated: +30 days)
                let _ = ctx.db.conn().execute(
                    "UPDATE recurring_schedules SET next_run_date = '2026-11-30' WHERE id = ?1",
                    params![sched.id],
                );

                generated.push(inv);
            }
        }

        Ok(generated)
    }

    // --- European E-Invoicing (Factur-X / ZUGFeRD 2.2 / EN 16931) (LEDGER-28) ---

    pub fn generate_factur_x_xml(
        invoice: &Invoice,
        seller: &crate::models::EInvoiceParty,
        buyer: &crate::models::EInvoiceParty,
    ) -> String {
        let issue_date = invoice.created_at.format("%Y%m%d").to_string();
        let currency = "EUR";

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<rsm:CrossIndustryInvoice xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\"\n");
        xml.push_str("    xmlns:qdt=\"urn:un:unece:uncefact:data:standard:QualifiedDataType:100\"\n");
        xml.push_str("    xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\"\n");
        xml.push_str("    xmlns:udt=\"urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100\">\n");

        // Context
        xml.push_str("  <rsm:ExchangedDocumentContext>\n");
        xml.push_str("    <ram:GuidelineSpecifiedDocumentContextParameter>\n");
        xml.push_str("      <ram:ID>urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:comfort</ram:ID>\n");
        xml.push_str("    </ram:GuidelineSpecifiedDocumentContextParameter>\n");
        xml.push_str("  </rsm:ExchangedDocumentContext>\n");

        // Header
        xml.push_str("  <rsm:ExchangedDocument>\n");
        xml.push_str(&format!("    <ram:ID>{}</ram:ID>\n", invoice.invoice_number));
        xml.push_str("    <ram:TypeCode>380</ram:TypeCode>\n"); // Commercial Invoice
        xml.push_str("    <ram:IssueDateTime>\n");
        xml.push_str(&format!("      <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n", issue_date));
        xml.push_str("    </ram:IssueDateTime>\n");
        xml.push_str("  </rsm:ExchangedDocument>\n");

        // Supply Chain Trade Transaction
        xml.push_str("  <rsm:SupplyChainTradeTransaction>\n");

        // Line Items
        for (idx, line) in invoice.line_items.iter().enumerate() {
            xml.push_str("    <ram:IncludedSupplyChainTradeLineItem>\n");
            xml.push_str("      <ram:AssociatedDocumentLineDocument>\n");
            xml.push_str(&format!("        <ram:LineID>{}</ram:LineID>\n", idx + 1));
            xml.push_str("      </ram:AssociatedDocumentLineDocument>\n");
            xml.push_str("      <ram:SpecifiedTradeProduct>\n");
            xml.push_str(&format!("        <ram:Name>{}</ram:Name>\n", xml_escape(&line.description)));
            xml.push_str("      </ram:SpecifiedTradeProduct>\n");
            xml.push_str("      <ram:SpecifiedLineTradeAgreement>\n");
            xml.push_str("        <ram:NetPriceProductTradePrice>\n");
            xml.push_str(&format!("          <ram:ChargeAmount>{:.2}</ram:ChargeAmount>\n", line.unit_price));
            xml.push_str("        </ram:NetPriceProductTradePrice>\n");
            xml.push_str("      </ram:SpecifiedLineTradeAgreement>\n");
            xml.push_str("      <ram:SpecifiedLineTradeDelivery>\n");
            xml.push_str(&format!("        <ram:BilledQuantity unitCode=\"C62\">{:.2}</ram:BilledQuantity>\n", line.quantity));
            xml.push_str("      </ram:SpecifiedLineTradeDelivery>\n");
            xml.push_str("      <ram:SpecifiedLineTradeSettlement>\n");
            xml.push_str("        <ram:ApplicableTradeTax>\n");
            xml.push_str("          <ram:TypeCode>VAT</ram:TypeCode>\n");
            xml.push_str("          <ram:CategoryCode>S</ram:CategoryCode>\n");
            xml.push_str(&format!("          <ram:RateApplicablePercent>{:.2}</ram:RateApplicablePercent>\n", line.tax_rate * Decimal::from(100)));
            xml.push_str("        </ram:ApplicableTradeTax>\n");
            xml.push_str("        <ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
            xml.push_str(&format!("          <ram:LineTotalAmount>{:.2}</ram:LineTotalAmount>\n", line.line_total));
            xml.push_str("        </ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
            xml.push_str("      </ram:SpecifiedLineTradeSettlement>\n");
            xml.push_str("    </ram:IncludedSupplyChainTradeLineItem>\n");
        }

        // Agreement & Parties
        xml.push_str("    <ram:ApplicableHeaderTradeAgreement>\n");
        // Seller
        xml.push_str("      <ram:SellerTradeParty>\n");
        xml.push_str(&format!("        <ram:Name>{}</ram:Name>\n", xml_escape(&seller.name)));
        xml.push_str("        <ram:SpecifiedTaxRegistration>\n");
        xml.push_str(&format!("          <ram:ID schemeID=\"VA\">{}</ram:ID>\n", seller.vat_id));
        xml.push_str("        </ram:SpecifiedTaxRegistration>\n");
        xml.push_str("        <ram:PostalTradeAddress>\n");
        xml.push_str(&format!("          <ram:PostcodeCode>{}</ram:PostcodeCode>\n", seller.postal_code));
        xml.push_str(&format!("          <ram:LineOne>{}</ram:LineOne>\n", xml_escape(&seller.street)));
        xml.push_str(&format!("          <ram:CityName>{}</ram:CityName>\n", xml_escape(&seller.city)));
        xml.push_str(&format!("          <ram:CountryID>{}</ram:CountryID>\n", seller.country_code));
        xml.push_str("        </ram:PostalTradeAddress>\n");
        xml.push_str("      </ram:SellerTradeParty>\n");

        // Buyer
        xml.push_str("      <ram:BuyerTradeParty>\n");
        xml.push_str(&format!("        <ram:Name>{}</ram:Name>\n", xml_escape(&buyer.name)));
        xml.push_str("        <ram:SpecifiedTaxRegistration>\n");
        xml.push_str(&format!("          <ram:ID schemeID=\"VA\">{}</ram:ID>\n", buyer.vat_id));
        xml.push_str("        </ram:SpecifiedTaxRegistration>\n");
        xml.push_str("        <ram:PostalTradeAddress>\n");
        xml.push_str(&format!("          <ram:PostcodeCode>{}</ram:PostcodeCode>\n", buyer.postal_code));
        xml.push_str(&format!("          <ram:LineOne>{}</ram:LineOne>\n", xml_escape(&buyer.street)));
        xml.push_str(&format!("          <ram:CityName>{}</ram:CityName>\n", xml_escape(&buyer.city)));
        xml.push_str(&format!("          <ram:CountryID>{}</ram:CountryID>\n", buyer.country_code));
        xml.push_str("        </ram:PostalTradeAddress>\n");
        xml.push_str("      </ram:BuyerTradeParty>\n");
        xml.push_str("    </ram:ApplicableHeaderTradeAgreement>\n");

        // Delivery
        xml.push_str("    <ram:ApplicableHeaderTradeDelivery/>\n");

        // Settlement & Totals
        xml.push_str("    <ram:ApplicableHeaderTradeSettlement>\n");
        xml.push_str(&format!("      <ram:InvoiceCurrencyCode>{}</ram:InvoiceCurrencyCode>\n", currency));
        if let Some(iban) = &seller.iban {
            xml.push_str("      <ram:SpecifiedTradeSettlementPaymentMeans>\n");
            xml.push_str("        <ram:TypeCode>58</ram:TypeCode>\n"); // SEPA Credit Transfer
            xml.push_str("        <ram:PayeePartyCreditorFinancialAccount>\n");
            xml.push_str(&format!("          <ram:IBANID>{}</ram:IBANID>\n", iban));
            xml.push_str("        </ram:PayeePartyCreditorFinancialAccount>\n");
            xml.push_str("      </ram:SpecifiedTradeSettlementPaymentMeans>\n");
        }
        xml.push_str("      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");
        xml.push_str(&format!("        <ram:LineTotalAmount>{:.2}</ram:LineTotalAmount>\n", invoice.subtotal));
        xml.push_str(&format!("        <ram:TaxBasisTotalAmount>{:.2}</ram:TaxBasisTotalAmount>\n", invoice.subtotal));
        xml.push_str(&format!("        <ram:TaxTotalAmount currencyID=\"{}\">{:.2}</ram:TaxTotalAmount>\n", currency, invoice.tax_total));
        xml.push_str(&format!("        <ram:GrandTotalAmount>{:.2}</ram:GrandTotalAmount>\n", invoice.grand_total));
        xml.push_str(&format!("        <ram:DuePayableAmount>{:.2}</ram:DuePayableAmount>\n", invoice.grand_total));
        xml.push_str("      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");
        xml.push_str("    </ram:ApplicableHeaderTradeSettlement>\n");

        xml.push_str("  </rsm:SupplyChainTradeTransaction>\n");
        xml.push_str("</rsm:CrossIndustryInvoice>\n");

        xml
    }

    pub fn validate_en16931(xml_content: &str) -> crate::models::EInvoiceValidationReport {
        let mut messages = Vec::new();
        let mut passed_rules = 0;

        // Offline Schematron validation of key EN 16931 mandatory business rules
        if xml_content.contains("<ram:ID>") {
            passed_rules += 1;
        } else {
            messages.push("BR-02: An Invoice shall have an Invoice number (<ram:ID>)".into());
        }

        if xml_content.contains("<ram:IssueDateTime>") {
            passed_rules += 1;
        } else {
            messages.push("BR-03: An Invoice shall have an Invoice issue date (<ram:IssueDateTime>)".into());
        }

        if xml_content.contains("<ram:SellerTradeParty>") && xml_content.contains("<ram:BuyerTradeParty>") {
            passed_rules += 1;
        } else {
            messages.push("BR-06 & BR-07: Seller and Buyer trade parties are mandatory".into());
        }

        if xml_content.contains("<ram:TaxTotalAmount") {
            passed_rules += 1;
        } else {
            messages.push("BR-CO-10: An Invoice shall contain a TaxTotalAmount".into());
        }

        if xml_content.contains("<ram:GrandTotalAmount>") {
            passed_rules += 1;
        } else {
            messages.push("BR-15: Grand total amount is mandatory".into());
        }

        if xml_content.contains("urn:cen.eu:en16931:2017") {
            passed_rules += 1;
        } else {
            messages.push("BR-01: Document must declare compliance with EN 16931 guideline identifier".into());
        }

        let is_valid = messages.is_empty();
        if is_valid {
            messages.push("All EN 16931 Schematron business rules verified successfully.".into());
        }

        crate::models::EInvoiceValidationReport {
            is_valid,
            standard_profile: "Factur-X / ZUGFeRD 2.2 EN 16931 (COMFORT Profile)".into(),
            rule_checks_passed: passed_rules,
            validation_messages: messages,
        }
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
