use qrcode::{Color, QrCode};
use rust_decimal::Decimal;

/// Pure-vector PDF-1.4 generator operating completely offline without external C/cloud libraries.
pub struct VectorPdfBuilder {
    content_stream: String,
}

impl VectorPdfBuilder {
    pub fn new() -> Self {
        Self {
            content_stream: String::new(),
        }
    }

    /// Set stroke RGB color (0.0 to 1.0)
    pub fn set_stroke_color(&mut self, r: f64, g: f64, b: f64) {
        self.content_stream.push_str(&format!("{:.3} {:.3} {:.3} RG\n", r, g, b));
    }

    /// Set fill RGB color (0.0 to 1.0)
    pub fn set_fill_color(&mut self, r: f64, g: f64, b: f64) {
        self.content_stream.push_str(&format!("{:.3} {:.3} {:.3} rg\n", r, g, b));
    }

    /// Set line stroke width in points
    pub fn set_line_width(&mut self, width: f64) {
        self.content_stream.push_str(&format!("{:.2} w\n", width));
    }

    /// Draw a stroked line from (x1, y1) to (x2, y2)
    pub fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.content_stream.push_str(&format!("{:.2} {:.2} m {:.2} {:.2} l S\n", x1, y1, x2, y2));
    }

    /// Draw a rectangle (stroked)
    pub fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.content_stream.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re S\n", x, y, w, h));
    }

    /// Fill a rectangle
    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.content_stream.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re f\n", x, y, w, h));
    }

    /// Draw text at position (x, y) with specified font ("F1" = Regular, "F2" = Bold, "F3" = Mono) and size
    pub fn draw_text(&mut self, font: &str, size: f64, x: f64, y: f64, text: &str) {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        self.content_stream.push_str(&format!(
            "BT\n/{} {:.2} Tf\n{:.2} {:.2} Td\n({}) Tj\nET\n",
            font, size, x, y, escaped
        ));
    }

    /// Render a vector QR code into the PDF stream
    pub fn draw_qr_code(&mut self, payload: &str, x: f64, y: f64, size: f64) {
        if let Ok(qr) = QrCode::new(payload.as_bytes()) {
            let width = qr.width();
            let module_size = size / (width as f64);
            self.set_fill_color(0.0, 0.0, 0.0);

            for row in 0..width {
                for col in 0..width {
                    if qr[(col, row)] == Color::Dark {
                        let mx = x + (col as f64) * module_size;
                        let my = y + ((width - 1 - row) as f64) * module_size;
                        self.fill_rect(mx, my, module_size, module_size);
                    }
                }
            }
        }
    }

    /// Assemble full PDF-1.4 file bytes with xref table
    pub fn build(self) -> Vec<u8> {
        let stream_bytes = self.content_stream.as_bytes();
        let stream_len = stream_bytes.len();

        let mut body = Vec::new();
        let mut offsets = Vec::new();

        let header = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n";
        body.extend_from_slice(header);

        // Object 1: Catalog
        offsets.push(body.len());
        body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        offsets.push(body.len());
        body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // Object 3: Page (A4: 595.28 x 841.89 pt)
        offsets.push(body.len());
        body.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] /Contents 4 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R /F3 7 0 R >> >> >>\nendobj\n",
        );

        // Object 4: Contents Stream
        offsets.push(body.len());
        let stream_obj_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_len);
        body.extend_from_slice(stream_obj_header.as_bytes());
        body.extend_from_slice(stream_bytes);
        body.extend_from_slice(b"\nendstream\nendobj\n");

        // Object 5: Helvetica
        offsets.push(body.len());
        body.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

        // Object 6: Helvetica-Bold
        offsets.push(body.len());
        body.extend_from_slice(b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n");

        // Object 7: Courier
        offsets.push(body.len());
        body.extend_from_slice(b"7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n");

        // Cross-reference table
        let xref_start = body.len();
        let mut xref = format!("xref\n0 8\n0000000000 65535 f \n");
        for off in offsets {
            xref.push_str(&format!("{:010} 00000 n \n", off));
        }
        body.extend_from_slice(xref.as_bytes());

        let trailer = format!(
            "trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_start
        );
        body.extend_from_slice(trailer.as_bytes());

        body
    }
}

/// Helper to render professional Invoice vector PDF with fixed-point Decimal math & EPC QR code
pub fn generate_invoice_pdf(
    invoice_number: &str,
    status: &str,
    customer_name: &str,
    created_at_str: &str,
    subtotal: Decimal,
    tax_total: Decimal,
    grand_total: Decimal,
    notes: Option<&str>,
    line_items: &[(String, Decimal, Decimal, Decimal, Decimal)], // (desc, qty, unit_price, tax_rate, total)
) -> Vec<u8> {
    let mut pdf = VectorPdfBuilder::new();

    // Top primary header banner
    pdf.set_fill_color(0.09, 0.16, 0.32); // Deep Navy #172952
    pdf.fill_rect(36.0, 770.0, 523.0, 48.0);

    pdf.set_fill_color(1.0, 1.0, 1.0);
    pdf.draw_text("F2", 18.0, 50.0, 788.0, "LEDGER ERP");
    pdf.draw_text("F1", 10.0, 180.0, 790.0, "Local-First Enterprise Management");
    pdf.draw_text("F2", 14.0, 430.0, 788.0, "INVOICE");

    // Company & invoice details
    pdf.set_fill_color(0.15, 0.2, 0.25);
    pdf.draw_text("F2", 12.0, 50.0, 740.0, "Billed To:");
    pdf.draw_text("F1", 12.0, 50.0, 722.0, customer_name);
    pdf.draw_text("F1", 10.0, 50.0, 706.0, "Enterprise Account • Net 30");

    pdf.draw_text("F2", 11.0, 360.0, 740.0, "Invoice Number:");
    pdf.draw_text("F3", 11.0, 460.0, 740.0, invoice_number);

    pdf.draw_text("F2", 11.0, 360.0, 722.0, "Date Issued:");
    let date_clean = created_at_str.split('T').next().unwrap_or(created_at_str);
    pdf.draw_text("F1", 11.0, 460.0, 722.0, date_clean);

    pdf.draw_text("F2", 11.0, 360.0, 704.0, "Status:");
    let is_final = status.eq_ignore_ascii_case("finalized") || status.eq_ignore_ascii_case("paid");
    if is_final {
        pdf.set_fill_color(0.02, 0.5, 0.25);
    } else {
        pdf.set_fill_color(0.7, 0.4, 0.05);
    }
    pdf.draw_text("F2", 11.0, 460.0, 704.0, &status.to_uppercase());

    // Line items table header
    pdf.set_fill_color(0.92, 0.94, 0.97);
    pdf.fill_rect(36.0, 665.0, 523.0, 22.0);
    pdf.set_stroke_color(0.8, 0.85, 0.9);
    pdf.set_line_width(0.75);
    pdf.draw_rect(36.0, 665.0, 523.0, 22.0);

    pdf.set_fill_color(0.2, 0.25, 0.35);
    pdf.draw_text("F2", 9.0, 46.0, 672.0, "#");
    pdf.draw_text("F2", 9.0, 70.0, 672.0, "DESCRIPTION");
    pdf.draw_text("F2", 9.0, 290.0, 672.0, "QTY");
    pdf.draw_text("F2", 9.0, 350.0, 672.0, "UNIT PRICE");
    pdf.draw_text("F2", 9.0, 440.0, 672.0, "TAX (VAT)");
    pdf.draw_text("F2", 9.0, 500.0, 672.0, "TOTAL");

    // Line items rows
    let mut current_y = 645.0;
    pdf.set_fill_color(0.1, 0.1, 0.1);

    for (idx, (desc, qty, unit_price, tax_rate, total)) in line_items.iter().enumerate() {
        if current_y < 280.0 {
            break;
        }

        // alternating row shading
        if idx % 2 == 1 {
            pdf.set_fill_color(0.98, 0.98, 0.99);
            pdf.fill_rect(36.0, current_y - 4.0, 523.0, 18.0);
        }

        pdf.set_fill_color(0.15, 0.2, 0.25);
        pdf.draw_text("F1", 9.0, 46.0, current_y, &format!("{}", idx + 1));
        pdf.draw_text("F2", 9.0, 70.0, current_y, desc);
        pdf.draw_text("F1", 9.0, 290.0, current_y, &format!("{}", qty));
        pdf.draw_text("F1", 9.0, 350.0, current_y, &format!("${}", unit_price));
        let tax_pct = (tax_rate * Decimal::from(100)).round_dp(0);
        pdf.draw_text("F1", 9.0, 440.0, current_y, &format!("{}%", tax_pct));
        pdf.draw_text("F2", 9.0, 500.0, current_y, &format!("${}", total));

        pdf.set_stroke_color(0.9, 0.9, 0.9);
        pdf.draw_line(36.0, current_y - 6.0, 559.0, current_y - 6.0);

        current_y -= 20.0;
    }

    // Totals Box
    let summary_y = 350.0;
    pdf.set_stroke_color(0.8, 0.85, 0.9);
    pdf.set_fill_color(0.97, 0.98, 1.0);
    pdf.fill_rect(340.0, summary_y, 219.0, 75.0);
    pdf.draw_rect(340.0, summary_y, 219.0, 75.0);

    pdf.set_fill_color(0.2, 0.25, 0.3);
    pdf.draw_text("F1", 10.0, 355.0, summary_y + 54.0, "Subtotal:");
    pdf.draw_text("F2", 10.0, 470.0, summary_y + 54.0, &format!("${}", subtotal));

    pdf.draw_text("F1", 10.0, 355.0, summary_y + 34.0, "Tax Breakdown (VAT):");
    pdf.draw_text("F2", 10.0, 470.0, summary_y + 34.0, &format!("${}", tax_total));

    pdf.set_stroke_color(0.7, 0.75, 0.85);
    pdf.draw_line(355.0, summary_y + 26.0, 545.0, summary_y + 26.0);

    pdf.set_fill_color(0.09, 0.16, 0.32);
    pdf.draw_text("F2", 12.0, 355.0, summary_y + 10.0, "Grand Total:");
    pdf.draw_text("F2", 12.0, 470.0, summary_y + 10.0, &format!("${}", grand_total));

    // Notes
    if let Some(n) = notes {
        pdf.set_fill_color(0.4, 0.45, 0.5);
        pdf.draw_text("F2", 9.0, 50.0, summary_y + 60.0, "Payment Terms & Notes:");
        pdf.draw_text("F1", 9.0, 50.0, summary_y + 44.0, n);
    }

    // EPC QR Code (GiroCode / SEPA QR)
    let epc_y = 120.0;
    pdf.set_stroke_color(0.85, 0.88, 0.92);
    pdf.set_fill_color(0.98, 0.99, 1.0);
    pdf.fill_rect(36.0, epc_y, 523.0, 170.0);
    pdf.draw_rect(36.0, epc_y, 523.0, 170.0);

    // Standard EPC QR payload
    let epc_payload = format!(
        "BCD\n002\n1\nSCT\n\nLedger ERP Workshop Inc\nDE89370400440532013000\nEUR{:.2}\n\n\n{}\n",
        grand_total, invoice_number
    );

    // Draw Vector QR Code (100pt x 100pt)
    pdf.draw_qr_code(&epc_payload, 50.0, epc_y + 35.0, 100.0);

    pdf.set_fill_color(0.1, 0.15, 0.25);
    pdf.draw_text("F2", 12.0, 165.0, epc_y + 135.0, "SEPA EPC QR Payment (GiroCode)");
    pdf.set_fill_color(0.35, 0.4, 0.45);
    pdf.draw_text("F1", 9.5, 165.0, epc_y + 118.0, "Scan with any mobile banking app to initiate zero-fee instant SEPA payment.");

    pdf.draw_text("F2", 9.0, 165.0, epc_y + 95.0, "Beneficiary:");
    pdf.draw_text("F1", 9.0, 240.0, epc_y + 95.0, "Ledger ERP Workshop Inc");

    pdf.draw_text("F2", 9.0, 165.0, epc_y + 78.0, "IBAN:");
    pdf.draw_text("F3", 9.5, 240.0, epc_y + 78.0, "DE89 3704 0044 0532 0130 00");

    pdf.draw_text("F2", 9.0, 165.0, epc_y + 61.0, "BIC:");
    pdf.draw_text("F3", 9.5, 240.0, epc_y + 61.0, "COBADEFFXXX");

    pdf.draw_text("F2", 9.0, 165.0, epc_y + 44.0, "Reference:");
    pdf.draw_text("F3", 9.5, 240.0, epc_y + 44.0, invoice_number);

    // Footer
    pdf.set_stroke_color(0.85, 0.85, 0.85);
    pdf.draw_line(36.0, 55.0, 559.0, 55.0);
    pdf.set_fill_color(0.5, 0.55, 0.6);
    pdf.draw_text(
        "F1",
        8.0,
        130.0,
        40.0,
        "Offline Vector PDF Generated by Ledger ERP • Immutable WAL-Backed Accounting • No Cloud Required",
    );

    pdf.build()
}

/// Helper to render professional Purchase Order vector PDF for vendor communication
pub fn generate_purchase_order_pdf(
    po_number: &str,
    status: &str,
    supplier_name: &str,
    created_at_str: &str,
    destination_location: &str,
    grand_total: Decimal,
    notes: Option<&str>,
    line_items: &[(String, String, Decimal, Decimal, Decimal)], // (sku, desc, qty, unit_cost, line_total)
) -> Vec<u8> {
    let mut pdf = VectorPdfBuilder::new();

    // Top primary header banner
    pdf.set_fill_color(0.12, 0.35, 0.28); // Forest / Emerald green for Procurement #1E5947
    pdf.fill_rect(36.0, 770.0, 523.0, 48.0);

    pdf.set_fill_color(1.0, 1.0, 1.0);
    pdf.draw_text("F2", 18.0, 50.0, 788.0, "LEDGER ERP");
    pdf.draw_text("F1", 10.0, 180.0, 790.0, "Procurement & Supply Chain Management");
    pdf.draw_text("F2", 14.0, 400.0, 788.0, "PURCHASE ORDER");

    // Vendor & Order details
    pdf.set_fill_color(0.15, 0.2, 0.25);
    pdf.draw_text("F2", 12.0, 50.0, 740.0, "Vendor / Supplier:");
    pdf.draw_text("F1", 12.0, 50.0, 722.0, supplier_name);
    pdf.draw_text("F1", 10.0, 50.0, 706.0, "Approved Component Supplier");

    pdf.draw_text("F2", 11.0, 360.0, 740.0, "PO Number:");
    pdf.draw_text("F3", 11.0, 460.0, 740.0, po_number);

    pdf.draw_text("F2", 11.0, 360.0, 722.0, "Order Date:");
    let date_clean = created_at_str.split('T').next().unwrap_or(created_at_str);
    pdf.draw_text("F1", 11.0, 460.0, 722.0, date_clean);

    pdf.draw_text("F2", 11.0, 360.0, 704.0, "Status:");
    pdf.set_fill_color(0.12, 0.4, 0.3);
    pdf.draw_text("F2", 11.0, 460.0, 704.0, &status.to_uppercase());

    // Delivery destination box
    pdf.set_fill_color(0.95, 0.97, 0.96);
    pdf.fill_rect(36.0, 650.0, 523.0, 32.0);
    pdf.set_stroke_color(0.8, 0.88, 0.84);
    pdf.draw_rect(36.0, 650.0, 523.0, 32.0);

    pdf.set_fill_color(0.15, 0.25, 0.2);
    pdf.draw_text("F2", 10.0, 46.0, 668.0, "Deliver To Warehouse Bin:");
    pdf.draw_text("F3", 10.0, 190.0, 668.0, destination_location);

    // Line items table header
    pdf.set_fill_color(0.9, 0.94, 0.92);
    pdf.fill_rect(36.0, 615.0, 523.0, 22.0);
    pdf.set_stroke_color(0.8, 0.88, 0.84);
    pdf.set_line_width(0.75);
    pdf.draw_rect(36.0, 615.0, 523.0, 22.0);

    pdf.set_fill_color(0.15, 0.25, 0.2);
    pdf.draw_text("F2", 9.0, 46.0, 622.0, "#");
    pdf.draw_text("F2", 9.0, 70.0, 622.0, "SKU / PART NUMBER");
    pdf.draw_text("F2", 9.0, 230.0, 622.0, "DESCRIPTION");
    pdf.draw_text("F2", 9.0, 370.0, 622.0, "ORDER QTY");
    pdf.draw_text("F2", 9.0, 440.0, 622.0, "UNIT COST");
    pdf.draw_text("F2", 9.0, 500.0, 622.0, "LINE TOTAL");

    // Line items rows
    let mut current_y = 595.0;
    for (idx, (sku, desc, qty, unit_cost, total)) in line_items.iter().enumerate() {
        if current_y < 240.0 {
            break;
        }

        if idx % 2 == 1 {
            pdf.set_fill_color(0.98, 0.99, 0.98);
            pdf.fill_rect(36.0, current_y - 4.0, 523.0, 18.0);
        }

        pdf.set_fill_color(0.15, 0.2, 0.25);
        pdf.draw_text("F1", 9.0, 46.0, current_y, &format!("{}", idx + 1));
        pdf.draw_text("F3", 9.0, 70.0, current_y, sku);
        pdf.draw_text("F1", 9.0, 230.0, current_y, desc);
        pdf.draw_text("F2", 9.0, 370.0, current_y, &format!("{}", qty));
        pdf.draw_text("F1", 9.0, 440.0, current_y, &format!("${}", unit_cost));
        pdf.draw_text("F2", 9.0, 500.0, current_y, &format!("${}", total));

        pdf.set_stroke_color(0.9, 0.92, 0.9);
        pdf.draw_line(36.0, current_y - 6.0, 559.0, current_y - 6.0);

        current_y -= 20.0;
    }

    // Totals Box
    let summary_y = 200.0;
    pdf.set_stroke_color(0.8, 0.88, 0.84);
    pdf.set_fill_color(0.95, 0.98, 0.96);
    pdf.fill_rect(340.0, summary_y, 219.0, 45.0);
    pdf.draw_rect(340.0, summary_y, 219.0, 45.0);

    pdf.set_fill_color(0.12, 0.35, 0.28);
    pdf.draw_text("F2", 12.0, 355.0, summary_y + 16.0, "Total Order Value:");
    pdf.draw_text("F2", 12.0, 470.0, summary_y + 16.0, &format!("${}", grand_total));

    if let Some(n) = notes {
        pdf.set_fill_color(0.4, 0.45, 0.42);
        pdf.draw_text("F2", 9.0, 50.0, summary_y + 35.0, "Purchasing Notes & Instructions:");
        pdf.draw_text("F1", 9.0, 50.0, summary_y + 18.0, n);
    }

    // Authorization sign-off block
    let sign_y = 100.0;
    pdf.set_stroke_color(0.8, 0.8, 0.8);
    pdf.draw_line(50.0, sign_y + 20.0, 240.0, sign_y + 20.0);
    pdf.draw_line(350.0, sign_y + 20.0, 530.0, sign_y + 20.0);

    pdf.set_fill_color(0.4, 0.45, 0.5);
    pdf.draw_text("F1", 9.0, 50.0, sign_y + 5.0, "Authorized Procurement Signature");
    pdf.draw_text("F1", 9.0, 350.0, sign_y + 5.0, "Vendor Acknowledgment & Date");

    // Footer
    pdf.set_stroke_color(0.85, 0.85, 0.85);
    pdf.draw_line(36.0, 55.0, 559.0, 55.0);
    pdf.set_fill_color(0.5, 0.55, 0.6);
    pdf.draw_text(
        "F1",
        8.0,
        140.0,
        40.0,
        "Generated offline via Ledger ERP v0.0.3 • Direct Vendor Communication • Zero Cloud Dependencies",
    );

    pdf.build()
}
