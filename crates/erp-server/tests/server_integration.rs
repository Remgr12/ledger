use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_ffi::init_erp;
use erp_server::{create_router, seed_demo_data};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn send_request(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (StatusCode, String) {
    let req = match body {
        Some(b) => Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    };

    let resp = router.clone().oneshot(req).await.expect("execute request");
    let status = resp.status();
    let body_bytes = resp.into_body().collect().await.expect("collect body").to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    (status, body_str)
}

#[tokio::test]
async fn test_headless_server_web_ui_and_rest_api() {
    let engine = init_erp(":memory:".to_string()).expect("init in-memory db");
    for m in &[
        "mod-inventory",
        "mod-contacts",
        "mod-invoicing",
        "mod-accounting",
        "mod-purchasing",
        "mod-manufacturing",
        "mod-projects",
    ] {
        engine.set_module_enabled(m.to_string(), true).expect("enable module");
    }

    seed_demo_data(&engine).expect("seed demo data");
    let router = create_router(engine);

    // 1. Web Console SPA (GET /)
    let (code, body) = send_request(&router, "GET", "/", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("⚡ LEDGER ERP"));
    assert!(body.contains("HEADLESS SERVER"));
    assert!(body.contains("Spatial Inventory Search"));

    // 2. Health Check (GET /api/v1/health)
    let (code, body) = send_request(&router, "GET", "/api/v1/health", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("\"status\":\"healthy\""));
    assert!(body.contains("\"service\":\"ledger-headless-server\""));
    assert!(body.contains("\"database\":\"sqlite_wal_active\""));

    // 3. Modules List (GET /api/v1/modules)
    let (code, body) = send_request(&router, "GET", "/api/v1/modules", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("mod-inventory"));
    assert!(body.contains("mod-accounting"));
    assert!(body.contains("mod-purchasing"));

    // 4. "Where Is It?" Spatial Locator Search (GET /api/v1/inventory/locate?q=M8)
    let (code, body) = send_request(&router, "GET", "/api/v1/inventory/locate?q=M8", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("M8-HEX-BOLT-316"));
    assert!(body.contains("Blue Bin #4"));
    assert!(body.contains("142.0"));

    // 5. Storage Hierarchy Tree (GET /api/v1/inventory/tree)
    let (code, body) = send_request(&router, "GET", "/api/v1/inventory/tree", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("WH-MAIN"));
    assert!(body.contains("WS-01"));
    assert!(body.contains("BIN-04"));

    // 6. QR Code Resolution (POST /api/v1/inventory/scan)
    let scan_payload = serde_json::json!({
        "payload": "erp://loc/BIN-04",
        "context_location_id": null
    }).to_string();
    let (code, body) = send_request(&router, "POST", "/api/v1/inventory/scan", Some(&scan_payload)).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("LocationContext"));
    assert!(body.contains("Blue Bin #4"));

    // 7. General Ledger & Trial Balance (GET /api/v1/accounting/trial-balance)
    let (code, body) = send_request(&router, "GET", "/api/v1/accounting/trial-balance", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("rows"));

    // 8. Purchasing (GET /api/v1/purchasing/orders)
    let (code, body) = send_request(&router, "GET", "/api/v1/purchasing/orders", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("PO-2026-081"));
    assert!(body.contains("1450.00"));

    // 9. Manufacturing (GET /api/v1/manufacturing/boms and work orders)
    let (code, body) = send_request(&router, "GET", "/api/v1/manufacturing/boms", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("BOM-ROBOT-01"));

    let (code, body) = send_request(&router, "GET", "/api/v1/manufacturing/work-orders", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("WO-2026-004"));

    // 10. Projects (GET /api/v1/projects)
    let (code, body) = send_request(&router, "GET", "/api/v1/projects", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("PRJ-AUTO-01"));

    // 11. Offline SVG Thermal Label Generation (GET /api/v1/inventory/label)
    let (code, body) = send_request(&router, "GET", "/api/v1/inventory/label?sku=M8-HEX-BOLT-316", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("<svg"));
    assert!(body.contains("M8-HEX-BOLT-316"));

    // 12. HLC Sync Stream Pull (GET /api/v1/sync/pull)
    let (code, body) = send_request(&router, "GET", "/api/v1/sync/pull?limit=10", None).await;
    assert_eq!(code, StatusCode::OK);
    assert!(body.contains("events"));
}
