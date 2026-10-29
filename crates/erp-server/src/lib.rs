pub mod web_ui;

use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use erp_ffi::{
    BomComponentDto, ErpEngine, JournalLineInputDto,
    PurchaseOrderLineDto, PurchaseReceiptLineDto,
};
use erp_kernel::{DomainEvent, EventLogStore};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ServerState {
    pub engine: Arc<ErpEngine>,
}

pub fn create_router(engine: Arc<ErpEngine>) -> Router {
    let state = ServerState { engine };

    Router::new()
        // Embedded Web Console SPA
        .route("/", get(index_handler))
        .route("/index.html", get(index_handler))
        // System & Health
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/modules", get(get_modules_handler))
        .route("/api/v1/modules/:id/toggle", post(toggle_module_handler))
        // Spatial Inventory & QR
        .route("/api/v1/inventory/locations", get(get_locations_handler).post(create_location_handler))
        .route("/api/v1/inventory/tree", get(get_location_tree_handler))
        .route("/api/v1/inventory/items", post(create_item_handler))
        .route("/api/v1/inventory/locate", get(locate_item_handler))
        .route("/api/v1/inventory/scan", post(scan_qr_handler))
        .route("/api/v1/inventory/move", post(move_stock_handler))
        .route("/api/v1/inventory/label", get(generate_label_handler))
        // Contacts & Invoicing
        .route("/api/v1/contacts", get(get_contacts_handler).post(create_contact_handler))
        .route("/api/v1/invoices", get(get_invoices_handler))
        .route("/api/v1/invoices/:id/finalize", post(finalize_invoice_handler))
        // Accounting & GL
        .route("/api/v1/accounting/accounts", get(get_accounts_handler))
        .route("/api/v1/accounting/trial-balance", get(get_trial_balance_handler))
        .route("/api/v1/accounting/journal-entries", post(post_journal_entry_handler))
        // Purchasing
        .route("/api/v1/purchasing/orders", get(get_purchase_orders_handler).post(create_purchase_order_handler))
        .route("/api/v1/purchasing/orders/:id/submit", post(submit_purchase_order_handler))
        .route("/api/v1/purchasing/receive", post(receive_goods_handler))
        // Manufacturing
        .route("/api/v1/manufacturing/boms", get(get_boms_handler).post(create_bom_handler))
        .route("/api/v1/manufacturing/work-orders", get(get_work_orders_handler).post(create_work_order_handler))
        .route("/api/v1/manufacturing/work-orders/:id/execute", post(execute_work_order_handler))
        // Projects
        .route("/api/v1/projects", get(get_projects_handler).post(create_project_handler))
        .route("/api/v1/projects/timesheets", post(record_timesheet_handler))
        .route("/api/v1/projects/:id/invoice", post(invoice_project_handler))
        // Sync Relay
        .route("/api/v1/sync/pull", get(sync_pull_handler))
        .route("/api/v1/sync/push", post(sync_push_handler))
        .layer(axum::middleware::map_response(add_cors_headers))
        .with_state(state)
}

async fn add_cors_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
    headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization"));
    response
}

// ----------------------------------------------------------------------------
// Handlers
// ----------------------------------------------------------------------------

async fn index_handler() -> Html<&'static str> {
    Html(web_ui::INDEX_HTML)
}

async fn health_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let hlc_time = state.engine.hlc.now().to_string();
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ledger-headless-server",
        "hlc": hlc_time,
        "utc_time": Utc::now().to_rfc3339(),
        "database": "sqlite_wal_active"
    }))
}

async fn get_modules_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mods = state.engine.get_available_modules().map_err(err_to_http)?;
    Ok(Json(mods))
}

#[derive(Deserialize)]
struct ToggleModuleRequest {
    enabled: bool,
}

async fn toggle_module_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ToggleModuleRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.engine.set_module_enabled(id, payload.enabled).map_err(err_to_http)?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// Inventory Handlers

async fn get_locations_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let locs = state.engine.get_storage_locations().map_err(err_to_http)?;
    Ok(Json(locs))
}

async fn get_location_tree_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tree = state.engine.get_location_tree().map_err(err_to_http)?;
    Ok(Json(tree))
}

#[derive(Deserialize)]
struct CreateLocationRequest {
    code: String,
    name: String,
    parent_id: Option<String>,
    location_type: String,
    map_x: Option<f64>,
    map_y: Option<f64>,
}

async fn create_location_handler(
    State(state): State<ServerState>,
    Json(payload): Json<CreateLocationRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let loc = state.engine.create_location(
        payload.code,
        payload.name,
        payload.parent_id,
        payload.location_type,
        payload.map_x,
        payload.map_y,
    ).map_err(err_to_http)?;
    Ok(Json(loc))
}

#[derive(Deserialize)]
struct CreateItemRequest {
    sku: String,
    name: String,
    description: Option<String>,
    unit: String,
    track_serial: Option<bool>,
    track_lot: Option<bool>,
    min_stock_level: Option<String>,
    barcode: Option<String>,
}

async fn create_item_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreateItemRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let item = state.engine.create_item(
        p.sku,
        p.name,
        p.description,
        p.unit,
        p.track_serial.unwrap_or(false),
        p.track_lot.unwrap_or(false),
        p.min_stock_level.unwrap_or_else(|| "0".into()),
        p.barcode,
    ).map_err(err_to_http)?;
    Ok(Json(item))
}

#[derive(Deserialize)]
struct LocateQuery {
    q: Option<String>,
}

async fn locate_item_handler(
    State(state): State<ServerState>,
    Query(q): Query<LocateQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query_str = q.q.unwrap_or_default();
    let matches = state.engine.search_item_locations(query_str).map_err(err_to_http)?;
    Ok(Json(matches))
}

#[derive(Deserialize)]
struct ScanRequest {
    payload: String,
    context_location_id: Option<String>,
}

async fn scan_qr_handler(
    State(state): State<ServerState>,
    Json(p): Json<ScanRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let resolution = state.engine.resolve_qr_scan(p.payload, p.context_location_id).map_err(err_to_http)?;
    Ok(Json(resolution))
}

#[derive(Deserialize)]
struct MoveStockRequest {
    item_id: String,
    from_loc: Option<String>,
    to_loc: Option<String>,
    qty: String,
    movement_type: Option<String>,
    lot_or_sn: Option<String>,
    notes: Option<String>,
}

async fn move_stock_handler(
    State(state): State<ServerState>,
    Json(p): Json<MoveStockRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let m_type = p.movement_type.unwrap_or_else(|| "Transfer".into());
    state.engine.move_stock(
        p.item_id,
        p.from_loc,
        p.to_loc,
        p.qty,
        m_type,
        p.lot_or_sn,
        p.notes,
    ).map_err(err_to_http)?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
struct LabelQuery {
    sku: String,
    title: Option<String>,
    subtitle: Option<String>,
}

async fn generate_label_handler(
    State(state): State<ServerState>,
    Query(q): Query<LabelQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let uri = format!("erp://item/{}", q.sku);
    let title = q.title.unwrap_or_else(|| q.sku.clone());
    let subtitle = q.subtitle.unwrap_or_else(|| "Warehouse Storage Unit".into());
    let svg = state.engine.generate_qr_label_svg(uri, title, subtitle).map_err(err_to_http)?;
    Ok(Json(serde_json::json!({ "sku": q.sku, "svg": svg })))
}

// Contacts Handlers

async fn get_contacts_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let contacts = state.engine.list_contacts().map_err(err_to_http)?;
    Ok(Json(contacts))
}

#[derive(Deserialize)]
struct CreateContactRequest {
    contact_type: Option<String>,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    tax_id: Option<String>,
    address: Option<String>,
}

async fn create_contact_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreateContactRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let c = state.engine.create_contact(
        p.contact_type.unwrap_or_else(|| "Customer".into()),
        p.name,
        p.email,
        p.phone,
        p.tax_id,
        p.address,
    ).map_err(err_to_http)?;
    Ok(Json(c))
}

// Invoices Handlers

async fn get_invoices_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let invoices = state.engine.list_invoices().map_err(err_to_http)?;
    Ok(Json(invoices))
}

async fn finalize_invoice_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let inv = state.engine.finalize_invoice(id).map_err(err_to_http)?;
    Ok(Json(inv))
}

// Accounting Handlers

async fn get_accounts_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let accounts = state.engine.get_chart_of_accounts().map_err(err_to_http)?;
    Ok(Json(accounts))
}

async fn get_trial_balance_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tb = state.engine.get_trial_balance().map_err(err_to_http)?;
    Ok(Json(tb))
}

#[derive(Deserialize)]
struct PostJournalEntryRequest {
    posting_date: String,
    lines: Vec<JournalLineInputDto>,
    voucher_type: Option<String>,
    voucher_id: Option<String>,
    remarks: Option<String>,
}

async fn post_journal_entry_handler(
    State(state): State<ServerState>,
    Json(p): Json<PostJournalEntryRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let voucher_type = p.voucher_type.unwrap_or_else(|| "Journal Entry".into());
    let voucher_id = p.voucher_id.unwrap_or_else(|| format!("JV-{}", Utc::now().timestamp_millis()));
    let entries = state.engine.post_journal_entry(
        p.posting_date,
        p.lines,
        voucher_type,
        voucher_id,
        p.remarks,
    ).map_err(err_to_http)?;
    Ok(Json(entries))
}

// Purchasing Handlers

async fn get_purchase_orders_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pos = state.engine.list_purchase_orders().map_err(err_to_http)?;
    Ok(Json(pos))
}

#[derive(Deserialize)]
struct CreatePurchaseOrderRequest {
    po_number: String,
    supplier_id: String,
    expected_delivery: Option<String>,
    lines: Vec<PurchaseOrderLineDto>,
    notes: Option<String>,
}

async fn create_purchase_order_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreatePurchaseOrderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let po = state.engine.create_purchase_order(
        p.po_number,
        p.supplier_id,
        p.expected_delivery,
        p.lines,
        p.notes,
    ).map_err(err_to_http)?;
    Ok(Json(po))
}

async fn submit_purchase_order_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let po = state.engine.submit_purchase_order(id).map_err(err_to_http)?;
    Ok(Json(po))
}

#[derive(Deserialize)]
struct ReceiveGoodsRequest {
    po_id: Option<String>,
    supplier_id: String,
    destination_location_id: String,
    items: Vec<PurchaseReceiptLineDto>,
    notes: Option<String>,
}

async fn receive_goods_handler(
    State(state): State<ServerState>,
    Json(p): Json<ReceiveGoodsRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let receipt = state.engine.receive_goods(
        p.po_id,
        p.supplier_id,
        p.destination_location_id,
        p.items,
        p.notes,
    ).map_err(err_to_http)?;
    Ok(Json(receipt))
}

// Manufacturing Handlers

async fn get_boms_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let boms = state.engine.list_boms().map_err(err_to_http)?;
    Ok(Json(boms))
}

#[derive(Deserialize)]
struct CreateBomRequest {
    bom_number: String,
    item_id: String,
    name: String,
    quantity: String,
    components: Vec<BomComponentDto>,
    routing_notes: Option<String>,
}

async fn create_bom_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreateBomRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let bom = state.engine.create_bom(
        p.bom_number,
        p.item_id,
        p.name,
        p.quantity,
        p.components,
        p.routing_notes,
    ).map_err(err_to_http)?;
    Ok(Json(bom))
}

async fn get_work_orders_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wos = state.engine.list_work_orders().map_err(err_to_http)?;
    Ok(Json(wos))
}

#[derive(Deserialize)]
struct CreateWorkOrderRequest {
    work_order_number: String,
    bom_id: String,
    target_quantity: String,
    source_location_id: String,
    destination_location_id: String,
}

async fn create_work_order_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreateWorkOrderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wo = state.engine.create_work_order(
        p.work_order_number,
        p.bom_id,
        p.target_quantity,
        p.source_location_id,
        p.destination_location_id,
    ).map_err(err_to_http)?;
    Ok(Json(wo))
}

async fn execute_work_order_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wo = state.engine.execute_work_order(id).map_err(err_to_http)?;
    Ok(Json(wo))
}

// Projects Handlers

async fn get_projects_handler(State(state): State<ServerState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let prjs = state.engine.list_projects().map_err(err_to_http)?;
    Ok(Json(prjs))
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    code: String,
    customer_id: Option<String>,
    billing_rate_hourly: String,
    estimated_hours: String,
}

async fn create_project_handler(
    State(state): State<ServerState>,
    Json(p): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let prj = state.engine.create_project(
        p.name,
        p.code,
        p.customer_id,
        p.billing_rate_hourly,
        p.estimated_hours,
    ).map_err(err_to_http)?;
    Ok(Json(prj))
}

#[derive(Deserialize)]
struct RecordTimesheetRequest {
    project_id: String,
    task_id: Option<String>,
    employee_or_user: Option<String>,
    activity_date: Option<String>,
    hours: String,
    hourly_rate: Option<String>,
    is_billable: Option<bool>,
    description: String,
}

async fn record_timesheet_handler(
    State(state): State<ServerState>,
    Json(p): Json<RecordTimesheetRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let ts = state.engine.record_timesheet(
        p.project_id,
        p.task_id,
        p.employee_or_user.unwrap_or_else(|| "admin".into()),
        p.activity_date.unwrap_or_else(|| Utc::now().date_naive().to_string()),
        p.hours,
        p.hourly_rate,
        p.is_billable.unwrap_or(true),
        p.description,
    ).map_err(err_to_http)?;
    Ok(Json(ts))
}

async fn invoice_project_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let invoice_no = format!("INV-PRJ-{}", uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    let inv = state.engine.invoice_unbilled_timesheets(id, invoice_no, "0.0".into()).map_err(err_to_http)?;
    Ok(Json(inv))
}

// Sync Relay Handlers

#[derive(Deserialize)]
struct SyncPullQuery {
    since: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SyncPullResponse {
    events: Vec<DomainEvent>,
    latest_hlc: Option<String>,
}

async fn sync_pull_handler(
    State(state): State<ServerState>,
    Query(q): Query<SyncPullQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).min(1000);
    let events = EventLogStore::get_events_since(
        &state.engine.db.conn(),
        q.since.as_deref(),
        limit,
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let latest_hlc = events.last().map(|e| e.hlc_timestamp.clone());
    Ok(Json(SyncPullResponse { events, latest_hlc }))
}

#[derive(Deserialize)]
struct SyncPushRequest {
    events: Vec<DomainEvent>,
}

async fn sync_push_handler(
    State(state): State<ServerState>,
    Json(p): Json<SyncPushRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut accepted = 0;
    for ev in p.events {
        if let Ok(ts) = ev.hlc_timestamp.parse() {
            state.engine.hlc.update(&ts);
        }
        if EventLogStore::append(&state.engine.db.conn(), &ev).is_ok() {
            accepted += 1;
        }
    }
    Ok(Json(serde_json::json!({
        "accepted_count": accepted,
        "current_hlc": state.engine.hlc.now().to_string()
    })))
}

fn err_to_http<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// ----------------------------------------------------------------------------
// Demo Data Seeder
// ----------------------------------------------------------------------------

pub fn seed_demo_data(engine: &Arc<ErpEngine>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Storage Locations
    let wh = engine.create_location(
        "WH-MAIN".into(),
        "Main Facility".into(),
        None,
        "Warehouse".into(),
        Some(100.0),
        Some(80.0),
    )?;

    let room = engine.create_location(
        "WS-01".into(),
        "Workshop".into(),
        Some(wh.id.clone()),
        "Room".into(),
        Some(140.0),
        Some(110.0),
    )?;

    let rack = engine.create_location(
        "RCK-02".into(),
        "Rack 2 (North Wall)".into(),
        Some(room.id.clone()),
        "Rack".into(),
        Some(160.0),
        Some(120.0),
    )?;

    let bin = engine.create_location(
        "BIN-04".into(),
        "Blue Bin #4".into(),
        Some(rack.id.clone()),
        "Bin".into(),
        Some(165.0),
        Some(125.0),
    )?;

    // 2. Items
    let item1 = engine.create_item(
        "M8-HEX-BOLT-316".into(),
        "M8x40mm Hex Bolt Stainless 316".into(),
        Some("Marine Grade 316 Stainless Fastener".into()),
        "pcs".into(),
        false,
        true,
        "50".into(),
        None,
    )?;

    let item2 = engine.create_item(
        "BEARING-608-2RS".into(),
        "Deep Groove Ball Bearing 608-2RS".into(),
        Some("Precision Sealed Ball Bearing".into()),
        "pcs".into(),
        false,
        true,
        "20".into(),
        None,
    )?;

    let item_robot = engine.create_item(
        "ROBOT-ARM-4AXIS".into(),
        "Desktop Robotic Arm 4-Axis".into(),
        Some("Assembled 4-axis stepper robotic mechanism".into()),
        "unit".into(),
        true,
        false,
        "2".into(),
        None,
    )?;

    // 3. Initial Stock Ledger Move into Blue Bin #4
    let _ = engine.move_stock(
        item1.id.clone(),
        None,
        Some(bin.id.clone()),
        "142.0".into(),
        "Receive".into(),
        Some("LOT-2026-Q1".into()),
        Some("Initial stock balance putaway".into()),
    );

    let _ = engine.move_stock(
        item2.id.clone(),
        None,
        Some(bin.id.clone()),
        "38.0".into(),
        "Receive".into(),
        Some("LOT-NSK-9".into()),
        Some("Initial stock balance putaway".into()),
    );

    // 4. Contacts
    let cust = engine.create_contact(
        "Customer".into(),
        "Acme Robotics Corp".into(),
        Some("procurement@acmerobotics.com".into()),
        Some("+1-555-0199".into()),
        None,
        None,
    )?;

    let supp = engine.create_contact(
        "Supplier".into(),
        "Fastener & Hardware Supply Co".into(),
        Some("orders@fastenerhardware.com".into()),
        Some("+1-555-0144".into()),
        None,
        None,
    )?;

    // 5. Purchase Order
    let po = engine.create_purchase_order(
        "PO-2026-081".into(),
        supp.id.clone(),
        Some("2026-11-15".into()),
        vec![PurchaseOrderLineDto {
            id: "pol-1".into(),
            item_id: item1.id.clone(),
            description: "M8x40mm Hex Bolt Stainless 316 (Box of 500)".into(),
            quantity: "500".into(),
            received_quantity: "0".into(),
            unit_price: "2.90".into(),
            tax_rate: "19.0".into(),
            line_total: "1450.00".into(),
        }],
        Some("Q4 Fastener restocking".into()),
    )?;
    let _ = engine.submit_purchase_order(po.id);

    // 6. BOM & Work Order
    let bom = engine.create_bom(
        "BOM-ROBOT-01".into(),
        item_robot.id.clone(),
        "Desktop Robotic Arm 4-Axis Assembly".into(),
        "1".into(),
        vec![
            BomComponentDto {
                id: "comp-1".into(),
                item_id: item1.id.clone(),
                quantity: "8".into(),
                scrap_percentage: "0.0".into(),
                notes: Some("M8 Structural Fasteners".into()),
            },
            BomComponentDto {
                id: "comp-2".into(),
                item_id: item2.id.clone(),
                quantity: "4".into(),
                scrap_percentage: "0.0".into(),
                notes: Some("Rotational Bearings".into()),
            },
        ],
        Some("CNC base mount -> bearing press fit -> calibration".into()),
    )?;

    let _ = engine.create_work_order(
        "WO-2026-004".into(),
        bom.id,
        "2".into(),
        bin.id.clone(),
        wh.id.clone(),
    );

    // 7. Project & Timesheet
    let prj = engine.create_project(
        "Warehouse Robotics Automation".into(),
        "PRJ-AUTO-01".into(),
        Some(cust.id.clone()),
        "150.00".into(),
        "60.0".into(),
    )?;

    let _ = engine.record_timesheet(
        prj.id.clone(),
        None,
        "lead_architect".into(),
        "2026-10-28".into(),
        "8.5".into(),
        Some("150.00".into()),
        true,
        "Kinematic calibration and sensor bus integration".into(),
    );

    Ok(())
}
