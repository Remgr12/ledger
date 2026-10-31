use qrcode::{Color, QrCode};
use rusqlite::Connection;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::errors::InventoryError;
use crate::location_service::LocationService;
use crate::models::{
    Gs1BarcodeData, ItemLocationMatch, LabelFormat, ParsedQr, ScanResolution,
};
use crate::stock_service::StockService;

pub struct QrService;

impl QrService {
    /// Parse raw scanned string into a structured ParsedQr,
    /// with full support for erp:// URIs and GS1-128 / GS1 DataMatrix Application Identifiers.
    pub fn parse_qr(raw: &str) -> ParsedQr {
        let trimmed = raw.trim();

        if let Some(rest) = trimmed.strip_prefix("erp://loc/") {
            ParsedQr::Location {
                id_or_code: rest.trim_end_matches('/').to_string(),
            }
        } else if let Some(rest) = trimmed.strip_prefix("erp://serial/").or_else(|| trimmed.strip_prefix("erp://sn/")) {
            ParsedQr::Serial {
                serial_number: rest.trim_end_matches('/').to_string(),
            }
        } else if let Some(rest) = trimmed.strip_prefix("erp://item/").or_else(|| trimmed.strip_prefix("erp://sku/")) {
            // Check for query parameters (?lot=...&sn=...)
            let mut parts = rest.splitn(2, '?');
            let sku = parts.next().unwrap_or_default().to_string();
            let mut lot = None;
            let mut serial = None;

            if let Some(query) = parts.next() {
                for pair in query.split('&') {
                    let mut kv = pair.splitn(2, '=');
                    let key = kv.next().unwrap_or_default();
                    let val = kv.next().unwrap_or_default();
                    if key == "lot" && !val.is_empty() {
                        lot = Some(val.to_string());
                    } else if (key == "sn" || key == "serial") && !val.is_empty() {
                        serial = Some(val.to_string());
                    }
                }
            }

            ParsedQr::Item { sku, lot, serial }
        } else if let Some(gs1) = Self::try_parse_gs1(trimmed) {
            ParsedQr::Gs1(gs1)
        } else {
            ParsedQr::ExternalBarcode {
                raw: trimmed.to_string(),
            }
        }
    }

    /// Parse GS1-128 / DataMatrix Application Identifiers (AI)
    /// Supports parenthesized: `(01)07350012345678(10)LOT-2026A(21)SN1234`
    /// and raw AI: `010735001234567810LOT-2026A`
    pub fn try_parse_gs1(raw: &str) -> Option<Gs1BarcodeData> {
        if raw.contains("(01)") {
            // Parenthesized AI format
            let mut gtin = None;
            let mut lot = None;
            let mut serial = None;
            let mut exp = None;

            let segments: Vec<&str> = raw.split('(').collect();
            for seg in segments {
                if seg.is_empty() {
                    continue;
                }
                let mut parts = seg.splitn(2, ')');
                let ai = parts.next().unwrap_or_default();
                let val = parts.next().unwrap_or_default();

                match ai {
                    "01" => gtin = Some(val.to_string()),
                    "10" => lot = Some(val.to_string()),
                    "21" => serial = Some(val.to_string()),
                    "17" => exp = Some(val.to_string()),
                    _ => {}
                }
            }

            if gtin.is_some() || lot.is_some() || serial.is_some() {
                return Some(Gs1BarcodeData {
                    gtin,
                    lot_number: lot,
                    serial_number: serial,
                    expiration_date: exp,
                    count: None,
                });
            }
        } else if raw.starts_with("01") && raw.len() >= 16 {
            // Raw GS1 AI format: AI 01 is 14 digits
            let gtin = Some(raw[2..16].to_string());
            let mut lot = None;
            let mut serial = None;

            let mut remainder = &raw[16..];
            while !remainder.is_empty() {
                if remainder.starts_with("10") {
                    let next_ai_idx = remainder[2..]
                        .find(|c: char| c.is_ascii_punctuation())
                        .map(|i| i + 2)
                        .unwrap_or(remainder.len());
                    lot = Some(remainder[2..next_ai_idx].to_string());
                    remainder = &remainder[next_ai_idx..];
                } else if remainder.starts_with("21") {
                    let next_ai_idx = remainder[2..]
                        .find(|c: char| c.is_ascii_punctuation())
                        .map(|i| i + 2)
                        .unwrap_or(remainder.len());
                    serial = Some(remainder[2..next_ai_idx].to_string());
                    remainder = &remainder[next_ai_idx..];
                } else {
                    break;
                }
            }

            return Some(Gs1BarcodeData {
                gtin,
                lot_number: lot,
                serial_number: serial,
                expiration_date: None,
                count: None,
            });
        }

        None
    }

    /// Resolve a scanned payload and execute the rapid action state machine.
    pub fn resolve_qr_scan(
        conn: &Connection,
        raw_payload: &str,
        active_context_location_id: Option<&str>,
    ) -> Result<ScanResolution, InventoryError> {
        let parsed = Self::parse_qr(raw_payload);

        match parsed {
            ParsedQr::Location { id_or_code } => {
                let location = LocationService::get_location_by_id(conn, &id_or_code)
                    .or_else(|_| LocationService::get_location_by_code(conn, &id_or_code))?;

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
                        q.serial_number,
                        i.min_stock_level,
                        lt.expiration_date,
                        lt.quarantine_status,
                        lt.manufacturer_batch
                    FROM stock_quants q
                    INNER JOIN inventory_items i ON q.item_id = i.id
                    INNER JOIN storage_locations l ON q.location_id = l.id
                    LEFT JOIN inventory_lots lt ON q.lot_number = lt.lot_number
                    WHERE l.id = ?1 AND CAST(q.quantity AS REAL) > 0
                    ORDER BY i.name ASC
                    "#,
                )?;

                let rows = stmt.query_map(rusqlite::params![location.id], |row| {
                    let quantity_str: String = row.get(11)?;
                    let quantity = Decimal::from_str(&quantity_str).unwrap_or(Decimal::ZERO);
                    let min_stock_str: String = row.get(14)?;
                    let min_stock_level = Decimal::from_str(&min_stock_str).unwrap_or(Decimal::ZERO);

                    Ok(ItemLocationMatch {
                        item_id: row.get(0)?,
                        sku: row.get(1)?,
                        item_name: row.get(2)?,
                        unit: row.get(3)?,
                        location_id: row.get(4)?,
                        location_code: row.get(5)?,
                        location_name: row.get(6)?,
                        location_type: row.get(7)?,
                        breadcrumb_path: row.get(8)?,
                        map_x: row.get(9)?,
                        map_y: row.get(10)?,
                        quantity,
                        lot_number: row.get(12)?,
                        serial_number: row.get(13)?,
                        min_stock_level,
                        lot_expiration_date: row.get(15)?,
                        lot_quarantine_status: row.get(16)?,
                        lot_manufacturer_batch: row.get(17)?,
                    })
                })?;

                let mut items = Vec::new();
                for r in rows {
                    items.push(r?);
                }

                Ok(ScanResolution::LocationContext {
                    prompt_message: format!(
                        "Target location active: [{}]. Scan an item to Putaway or Transfer.",
                        location.name
                    ),
                    location,
                    items,
                })
            }

            ParsedQr::Item { sku, lot, serial } => {
                let item = StockService::get_item_by_sku(conn, &sku)?;
                Self::resolve_item_action(conn, item, lot, serial, active_context_location_id, raw_payload)
            }

            ParsedQr::Gs1(gs1) => {
                // Find item by GTIN barcode or SKU
                let gtin_str = gs1.gtin.clone().unwrap_or_default();
                let item = StockService::get_item_by_sku(conn, &gtin_str).or_else(|_| {
                    let mut stmt = conn.prepare(
                        "SELECT id, sku, name, description, unit, track_serial, track_lot, min_stock_level, qr_code_uri, barcode, created_at FROM inventory_items WHERE barcode = ?1",
                    )?;
                    stmt.query_row(rusqlite::params![gtin_str], |row| {
                        Ok(crate::models::InventoryItem {
                            id: row.get(0)?,
                            sku: row.get(1)?,
                            name: row.get(2)?,
                            description: row.get(3)?,
                            unit: row.get(4)?,
                            track_serial: row.get::<_, i32>(5)? != 0,
                            track_lot: row.get::<_, i32>(6)? != 0,
                            min_stock_level: Decimal::from_str(&row.get::<_, String>(7)?).unwrap_or(Decimal::ZERO),
                            qr_code_uri: row.get(8)?,
                            barcode: row.get(9)?,
                            standard_cost: None,
                            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?).unwrap_or_default().with_timezone(&chrono::Utc),
                        })
                    }).map_err(|_| InventoryError::ItemNotFound(gtin_str.clone()))
                })?;

                Self::resolve_item_action(
                    conn,
                    item,
                    gs1.lot_number,
                    gs1.serial_number,
                    active_context_location_id,
                    raw_payload,
                )
            }

            ParsedQr::ExternalBarcode { raw } => {
                let item = StockService::get_item_by_sku(conn, &raw).or_else(|_| {
                    let mut stmt = conn.prepare(
                        "SELECT id, sku, name, description, unit, track_serial, track_lot, min_stock_level, qr_code_uri, barcode, created_at FROM inventory_items WHERE barcode = ?1",
                    )?;
                    stmt.query_row(rusqlite::params![raw], |row| {
                        Ok(crate::models::InventoryItem {
                            id: row.get(0)?,
                            sku: row.get(1)?,
                            name: row.get(2)?,
                            description: row.get(3)?,
                            unit: row.get(4)?,
                            track_serial: row.get::<_, i32>(5)? != 0,
                            track_lot: row.get::<_, i32>(6)? != 0,
                            min_stock_level: Decimal::from_str(&row.get::<_, String>(7)?).unwrap_or(Decimal::ZERO),
                            qr_code_uri: row.get(8)?,
                            barcode: row.get(9)?,
                            standard_cost: None,
                            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?).unwrap_or_default().with_timezone(&chrono::Utc),
                        })
                    }).map_err(|_| InventoryError::ItemNotFound(raw.clone()))
                });

                match item {
                    Ok(item) => Self::resolve_item_action(conn, item, None, None, active_context_location_id, raw_payload),
                    Err(_) => Ok(ScanResolution::Unknown {
                        raw_payload: raw.clone(),
                        message: format!("Barcode '{}' not recognized as location, item, or GS1 matrix.", raw),
                    }),
                }
            }
            ParsedQr::Serial { serial_number } => {
                let asset = StockService::get_serialized_asset(conn, &serial_number)?;
                if let Some(asset) = asset {
                    let item = StockService::get_item_by_id(conn, &asset.item_id)?;
                    Self::resolve_item_action(conn, item, None, Some(serial_number), active_context_location_id, raw_payload)
                } else {
                    Ok(ScanResolution::Unknown {
                        raw_payload: raw_payload.to_string(),
                        message: format!("Serialized asset '{}' not found.", serial_number),
                    })
                }
            }
        }
    }

    fn resolve_item_action(
        conn: &Connection,
        item: crate::models::InventoryItem,
        _lot: Option<String>,
        _serial: Option<String>,
        active_context_location_id: Option<&str>,
        _raw_payload: &str,
    ) -> Result<ScanResolution, InventoryError> {
        if let Some(loc_id) = active_context_location_id {
            let target_location = LocationService::get_location_by_id(conn, loc_id)
                .or_else(|_| LocationService::get_location_by_code(conn, loc_id))?;

            let current_qty_str: Option<String> = conn
                .query_row(
                    "SELECT quantity FROM stock_quants WHERE item_id = ?1 AND location_id = ?2",
                    rusqlite::params![item.id, target_location.id],
                    |r| r.get(0),
                )
                .ok();

            let current_qty = current_qty_str
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or(Decimal::ZERO);

            return Ok(ScanResolution::RapidStockAction {
                item,
                target_location,
                current_quantity_at_location: current_qty,
                suggested_action: "Putaway / Transfer".to_string(),
            });
        }

        let locations = StockService::locate_item(conn, &item.sku)?;
        let total_quantity = StockService::get_total_stock(conn, &item.id)?;

        Ok(ScanResolution::ItemLookup {
            item,
            locations,
            total_quantity,
        })
    }

    /// Generate offline, crisp vector SVG label formatted for standard thermal label printers
    /// or sticker sheets.
    pub fn generate_qr_label_svg(
        target_uri: &str,
        title: &str,
        subtitle: &str,
        details: Option<&str>,
    ) -> Result<String, InventoryError> {
        Self::generate_label_svg(
            LabelFormat::ThermalRoll62mm,
            target_uri,
            title,
            subtitle,
            details,
        )
    }

    /// Multi-format label generator supporting 62mm thermal rolls, 4x6" Zebra labels, and A4 sticker sheets.
    pub fn generate_label_svg(
        format: LabelFormat,
        target_uri: &str,
        title: &str,
        subtitle: &str,
        details: Option<&str>,
    ) -> Result<String, InventoryError> {
        match format {
            LabelFormat::ThermalRoll62mm => {
                Self::render_single_label_svg(380, 180, 4, target_uri, title, subtitle, details)
            }
            LabelFormat::Thermal4x6 => {
                Self::render_4x6_label_svg(target_uri, title, subtitle, details)
            }
            LabelFormat::A4StickerSheet => {
                Self::render_a4_sheet_svg(target_uri, title, subtitle)
            }
        }
    }

    fn render_single_label_svg(
        canvas_width: u32,
        canvas_height: u32,
        module_size: u32,
        target_uri: &str,
        title: &str,
        subtitle: &str,
        details: Option<&str>,
    ) -> Result<String, InventoryError> {
        let code = QrCode::new(target_uri.as_bytes())
            .map_err(|e| InventoryError::QrGeneration(e.to_string()))?;

        let colors = code.to_colors();
        let width = code.width() as u32;
        let qr_pixel_size = width * module_size;

        let mut qr_rects = String::with_capacity(colors.len() * 32);
        for y in 0..width {
            for x in 0..width {
                if colors[(y * width + x) as usize] == Color::Dark {
                    let rx = 16 + (x * module_size);
                    let ry = 20 + (y * module_size);
                    qr_rects.push_str(&format!(
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#111827"/>"##,
                        rx, ry, module_size, module_size
                    ));
                }
            }
        }

        let text_x = 32 + qr_pixel_size;
        let details_line = match details {
            Some(d) => format!(
                r##"<text x="{}" y="115" font-family="sans-serif" font-size="12" fill="#6B7280">{}</text>"##,
                text_x, escape_xml(d)
            ),
            None => String::new(),
        };

        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canvas_width} {canvas_height}" width="{canvas_width}" height="{canvas_height}">
  <!-- Label Background & Border -->
  <rect x="2" y="2" width="{border_w}" height="{border_h}" rx="8" ry="8" fill="#FFFFFF" stroke="#E5E7EB" stroke-width="2"/>
  <rect x="6" y="6" width="{inner_w}" height="{inner_h}" rx="6" ry="6" fill="#F9FAFB"/>

  <!-- QR Code Matrix -->
  {qr_rects}

  <!-- Typography -->
  <text x="{text_x}" y="50" font-family="sans-serif" font-size="20" font-weight="700" fill="#111827">{title}</text>
  <text x="{text_x}" y="80" font-family="sans-serif" font-size="13" font-weight="500" fill="#374151">{subtitle}</text>
  {details_line}

  <!-- URI Reference Badge -->
  <rect x="{text_x}" y="132" width="{badge_w}" height="24" rx="4" fill="#EEF2FF"/>
  <text x="{uri_x}" y="148" font-family="monospace" font-size="11" font-weight="600" fill="#4338CA">{uri}</text>
</svg>"##,
            canvas_width = canvas_width,
            canvas_height = canvas_height,
            border_w = canvas_width - 4,
            border_h = canvas_height - 4,
            inner_w = canvas_width - 12,
            inner_h = canvas_height - 12,
            qr_rects = qr_rects,
            text_x = text_x,
            title = escape_xml(title),
            subtitle = escape_xml(subtitle),
            details_line = details_line,
            badge_w = canvas_width - text_x - 16,
            uri_x = text_x + 8,
            uri = escape_xml(target_uri)
        );

        Ok(svg)
    }

    /// High-Density 4x6" Industrial Label with both 2D QR and simulated 1D Barcode bars
    fn render_4x6_label_svg(
        target_uri: &str,
        title: &str,
        subtitle: &str,
        details: Option<&str>,
    ) -> Result<String, InventoryError> {
        let code = QrCode::new(target_uri.as_bytes())
            .map_err(|e| InventoryError::QrGeneration(e.to_string()))?;

        let colors = code.to_colors();
        let width = code.width() as u32;
        let module_size = 5;

        let mut qr_rects = String::with_capacity(colors.len() * 32);
        for y in 0..width {
            for x in 0..width {
                if colors[(y * width + x) as usize] == Color::Dark {
                    let rx = 30 + (x * module_size);
                    let ry = 80 + (y * module_size);
                    qr_rects.push_str(&format!(
                        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#000000"/>"##,
                        rx, ry, module_size, module_size
                    ));
                }
            }
        }

        // Simulated Code-128 1D vector barcode bars for title
        let mut barcode_1d = String::new();
        let bar_x_start = 30;
        let bar_y = 260;
        let mut curr_x = bar_x_start;
        for b in title.bytes() {
            let pattern = (b % 4) + 1;
            barcode_1d.push_str(&format!(
                r##"<rect x="{}" y="{}" width="{}" height="50" fill="#000000"/>"##,
                curr_x, bar_y, pattern * 2
            ));
            curr_x += (pattern * 2) + 4;
        }

        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 350" width="500" height="350">
  <rect x="0" y="0" width="500" height="350" fill="#FFFFFF"/>
  <rect x="10" y="10" width="480" height="330" fill="none" stroke="#000000" stroke-width="3"/>

  <!-- Top Title Banner -->
  <line x1="10" y1="65" x2="490" y2="65" stroke="#000000" stroke-width="2"/>
  <text x="30" y="48" font-family="sans-serif" font-size="28" font-weight="900" fill="#000000">{title}</text>

  <!-- 2D QR Code -->
  {qr_rects}

  <!-- Information Section -->
  <text x="210" y="110" font-family="sans-serif" font-size="16" font-weight="700" fill="#000000">NAME / DESC:</text>
  <text x="210" y="135" font-family="sans-serif" font-size="15" fill="#333333">{subtitle}</text>

  <text x="210" y="175" font-family="sans-serif" font-size="16" font-weight="700" fill="#000000">DETAILS / LOT:</text>
  <text x="210" y="200" font-family="sans-serif" font-size="14" fill="#333333">{details}</text>

  <line x1="10" y1="240" x2="490" y2="240" stroke="#000000" stroke-width="2"/>

  <!-- 1D Vector Barcode -->
  {barcode_1d}
  <text x="30" y="325" font-family="monospace" font-size="14" font-weight="700" fill="#000000">{title}</text>
  <text x="280" y="325" font-family="monospace" font-size="12" fill="#666666">{uri}</text>
</svg>"##,
            title = escape_xml(title),
            subtitle = escape_xml(subtitle),
            details = escape_xml(details.unwrap_or("Standard Item")),
            qr_rects = qr_rects,
            barcode_1d = barcode_1d,
            uri = escape_xml(target_uri)
        );

        Ok(svg)
    }

    /// 24-Up A4 Sticker Sheet (3 columns x 8 rows)
    fn render_a4_sheet_svg(
        target_uri: &str,
        title: &str,
        subtitle: &str,
    ) -> Result<String, InventoryError> {
        let mut labels = String::new();
        let cols = 3;
        let rows = 8;
        let label_w = 230;
        let label_h = 110;

        for r in 0..rows {
            for c in 0..cols {
                let x = 40 + (c * (label_w + 15));
                let y = 40 + (r * (label_h + 12));
                labels.push_str(&format!(
                    r##"<rect x="{x}" y="{y}" width="{label_w}" height="{label_h}" rx="4" fill="#FFFFFF" stroke="#CCCCCC"/>
<text x="{tx}" y="{ty1}" font-family="sans-serif" font-size="13" font-weight="700" fill="#111827">{title}</text>
<text x="{tx}" y="{ty2}" font-family="sans-serif" font-size="10" fill="#4B5563">{subtitle}</text>
<text x="{tx}" y="{ty3}" font-family="monospace" font-size="9" fill="#4338CA">{uri}</text>"##,
                    x = x,
                    y = y,
                    label_w = label_w,
                    label_h = label_h,
                    tx = x + 10,
                    ty1 = y + 25,
                    ty2 = y + 45,
                    ty3 = y + 85,
                    title = escape_xml(title),
                    subtitle = escape_xml(subtitle),
                    uri = escape_xml(target_uri)
                ));
            }
        }

        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 1100" width="800" height="1100">
  <rect width="800" height="1100" fill="#F3F4F6"/>
  {labels}
</svg>"##,
            labels = labels
        );

        Ok(svg)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
