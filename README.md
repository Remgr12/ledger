# Ledger ERP

> An easy-to-use, local-first, modular Enterprise Resource Planning system built for sole proprietors, workshops, craftsmen, and small businesses.

Unlike traditional monolithic or cloud-hosted ERPs that require expensive servers, external databases, continuous internet connectivity, and complex IT infrastructure, **Ledger ERP** is designed from the ground up to be **offline-first**, **self-contained**, and **blazingly fast**.

---

## ⚡ Key Highlights & Capabilities

* **Local-First & Zero-Cloud Dependency**: Runs on an embedded **SQLite** engine operating with Write-Ahead Logging (WAL) for microsecond queries and offline durability.
* **Offline Vector PDF Generation with EPC QR GiroCode**:
  * Pure vector PDF-1.4 generation engine in native Rust (`crates/erp-ffi/src/pdf_generator.rs`) without external browser or cloud dependencies.
  * Invoices feature vector European EPC QR payment codes (SEPA QR / GiroCode) for 1-scan mobile banking payments.
  * Direct export for Purchase Orders and Invoices with in-app preview dialogs.
* **Peer-to-Peer Local Network Database Sync (LAN / mDNS)**:
  * Zero-configuration peer discovery over local Wi-Fi / Ethernet via mDNS.
  * Event-driven CRDT convergence engine using Hybrid Logical Clock (HLC) timestamps for deterministic causal conflict resolution.
  * Real-time top bar status indicator and Settings sync management dashboard with manual sync trigger.
* **Automated Low-Stock Alerts & 1-Click PO Reordering**:
  * Continuous tracking against SKU and location-level `min_stock_level` thresholds.
  * Prominent low-stock warning banners and card badges in the Item Locator.
  * 1-click "Generate Reorder PO" action producing populated draft purchase orders with suggested replenishment quantities.
* **Interactive Kanban Workflow Boards**:
  * Work Order Kanban swimlanes (`Planned`, `In Progress`, `Quality Check`, `Completed`) with atomic state transitions recorded in the event log.
  * Project Task Kanban board with status transitions, quick time logging chips (+1h, +2h, +4h, +8h), and unbilled hours summary.
* **Lot Expiration Tracking & Component Genealogy Tree**:
  * Complete lot tracking with expiration timestamps, manufacturer batch codes, and QA quarantine states in `mod-inventory`.
  * Visual expiration warning chips and quarantine putaway protection in Rapid Scan Move HUD.
  * Interactive Component Genealogy Tree modal viewer in Manufacturing, detailing the exact component lot numbers used in BOM assembly.
* **Spatial Inventory & 2D Floor Plan Radar**:
  * Self-referencing storage tree (`Warehouse` → `Room` → `Rack` → `Shelf` → `Bin`).
  * Interactive 2D drag-and-drop floor plan canvas with coordinate persistence in SQLite.
  * Instant "Where Is It?" search with breadcrumb navigation and visual shelf-level indicators.
* **Rapid Scan-to-Move & Cycle Count HUD**:
  * High-velocity 2-tap stock transfer workflow (Scan Bin QR → Scan Item Barcode → Done).
  * Cycle count audit mode for reconciling physical stock with the immutable stock ledger.
  * Hardware camera barcode scanning on Android (CameraX + ML Kit) and iOS (AVFoundation).
  * USB/Bluetooth HID keyboard wedge barcode scanner support on Desktop.
* **Offline Thermal Label Printing Station**:
  * Direct binary ESC/POS and plain-text TSPL/ZPL command generation with native QR code commands (`GS ( k`).
  * Supports standard thermal rolls (Brother 62mm continuous DK-22205 / die-cut DK-11209), standard Avery die-cut sheets (5160 / 24-up), and POS receipt printers (58mm/80mm).
  * Dispatches to system print dialog (OS spooler) or raw serial/USB devices (`/dev/usb/lp0`).
* **Double-Entry General Ledger & Invoicing**:
  * Strict accounting invariants: total debits must equal total credits before commitment.
  * Pre-seeded standard Chart of Accounts (Assets, Liabilities, Equity, Revenue, Expenses).
  * Real-time Trial Balance and Profit & Loss generation.
  * Sales invoicing with fixed-point `rust_decimal::Decimal` precision (zero floating-point errors).
* **Sandboxed WebAssembly (WASM) Plugin Extensibility**:
  * Securely extend functionality using compiled `.wasm` modules (`wasm32-wasip1`) running in an embedded Extism sandbox.
  * In-app plugin installer and management UI in Settings.
  * Declarative dynamic UI node trees rendered natively with Compose Multiplatform.
* **Local Database Backup & Data Portability**:
  * Safe SQLite full database backup with WAL flushing (`PRAGMA wal_checkpoint(TRUNCATE)`).
  * CSV and JSON data export for inventory items, contacts, and GL journal entries.
  * Bulk data ingestion from CSV spreadsheets.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│               Compose Multiplatform UI (Desktop, Android, iOS)   │
│  - Spatial Inventory HUD  - Invoicing & PDF  - Manufacturing    │
│  - Accounting / GL        - Purchasing & PO  - Kanban Boards    │
│  - Interactive Floor Plan - Camera Barcode   - Genealogy Trees  │
│  - Thermal Label Station  - P2P LAN Sync     - WASM Plugins     │
└───────────────────────────────┬─────────────────────────────────┘
                                │ Mozilla UniFFI (JNA / C-ABI)
                                │ Auto-bundled via compileRustLib
┌───────────────────────────────▼─────────────────────────────────┐
│                        ErpEngine (Rust Core)                    │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ Modules: Inventory, Contacts, Invoicing, Accounting,        │ │
│ │          Purchasing, Manufacturing, Projects                │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌───────────────────────┐ ┌───────────────────────────────────┐ │
│ │ Event Bus & HLC Clock │ │ WebAssembly Plugin Host (Extism)  │ │
│ └───────────────────────┘ └───────────────────────────────────┘ │
│ ┌───────────────────────┐ ┌───────────────────────────────────┐ │
│ │ P2P LAN/mDNS Sync     │ │ Pure Vector PDF-1.4 Generator     │ │
│ │ (CRDT Event Exchange) │ │ (Fixed-point Decimal & EPC QR)    │ │
│ └───────────────────────┘ └───────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ SQLite Database Engine (WAL Mode, rusqlite with safe backup)│ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📦 Project Structure

```
ledger/
├── .githooks/                # Tracked git pre-commit and pre-push hooks
├── crates/
│   ├── erp-kernel/           # SQLite connection pool, HLC clock, event log, P2P CRDT sync
│   ├── erp-plugin-host/      # Extism WebAssembly runtime & declarative UI layout engine
│   ├── erp-ffi/              # Mozilla UniFFI bindings, ErpEngine, & pure vector PDF generator
│   ├── erp-server/           # Headless Axum server & web management console
│   ├── erp-cli/              # Terminal CLI (ledger-cli) for offline warehouse operations
│   └── modules/
│       ├── mod-inventory/    # Location tree, stock ledger, QR codes, floor plan, lot tracking
│       ├── mod-contacts/     # Customer, vendor, and supplier directory
│       ├── mod-invoicing/    # Invoices, lines, tax calculations, and status transitions
│       ├── mod-accounting/   # Chart of accounts, double-entry GL, trial balance, P&L
│       ├── mod-purchasing/   # Purchase orders, goods receipts, and low-stock reorder
│       ├── mod-manufacturing/# Bills of materials, work orders, Kanban, component genealogy
│       └── mod-projects/     # Projects, timesheets, Kanban boards, unbilled summaries
├── composeApp/               # Compose Multiplatform UI (Desktop, Android, iOS)
│   ├── src/commonMain/       # Shared UI features, screens, and ErpBridge
│   ├── src/desktopMain/      # Desktop entrypoint, JNA library loader, thermal printing
│   ├── src/androidMain/      # CameraX & ML Kit mobile barcode scanner
│   ├── src/iosMain/          # AVFoundation iOS barcode scanner
│   └── src/desktopTest/      # Desktop unit & native integration tests
└── plugins/
    └── sample-supplier-plugin/ # Reference WASM plugin (supplier GS1 decoding & QA badges)
```

---

## 🚀 Getting Started

### Prerequisites
* **Rust**: 1.80+ (`cargo`, `rustc`, `wasm32-wasip1` target for plugins)
* **Java Development Kit**: OpenJDK 21 (for Compose Multiplatform Desktop)
* **Build Tools**: CMake, Make, GCC/Clang

### 1. Configure Git Hooks
To ensure automatic code quality and test validation before commits and pushes:
```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push
```

### 2. Build Core Rust Crates
```bash
# Build all crates in release mode
cargo build --release

# Run Rust unit and integration test suite across all crates
cargo test --workspace
```

### 3. Run Desktop Application
The Gradle build automatically compiles the Rust FFI library and packages `liberp_ffi.so` directly into the classpath:

```bash
cd composeApp

# Launch desktop app (automatically runs compileRustLib)
./gradlew run
```

### 4. Run Desktop Integration Tests
```bash
cd composeApp

# Runs NativeLoadTest, ThermalPrintingTest, BackupAndPluginTest, and V003FeaturesTest
./gradlew desktopTest
```

### 5. Build Standalone Desktop JAR
```bash
cd composeApp

# Produces standalone desktop release JAR
./gradlew desktopJar
```

---

## ⌨️ Desktop Keyboard Shortcuts

| Shortcut | Screen / Action |
| :--- | :--- |
| `Ctrl / Cmd + F` | Item Locator Screen |
| `Ctrl / Cmd + S` | Rapid Scan & Move HUD |
| `Ctrl / Cmd + T` | Storage Location Tree & Floor Plan |
| `Ctrl / Cmd + P` | QR Label Printing Station |
| `Ctrl / Cmd + I` | Invoicing Screen & Vector PDF Export |
| `Ctrl / Cmd + A` | Accounting / General Ledger |
| `Ctrl / Cmd + B` | Purchasing & POs |
| `Ctrl / Cmd + O` | Manufacturing & Work Order Kanban |
| `Ctrl / Cmd + J` | Projects & Timesheet Kanban |
| `Ctrl / Cmd + M` | Settings, P2P Sync & Plugins |
| `Escape` | Clear active context location |

---

## 🔌 Building WASM Plugins

```bash
cd plugins/sample-supplier-plugin

# Compile to WebAssembly targeting WASI
cargo build --target wasm32-wasip1 --release

# Copy to runtime directory
cp target/wasm32-wasip1/release/sample_supplier_plugin.wasm ../sample_supplier_plugin.wasm
```

Open the **Settings** view in Ledger ERP, provide the file path, and click **Install & Activate .wasm Module** to hot-reload the plugin into the UI.

---

## 🏷️ Release History

* **v0.0.4**:
  * **LEDGER-16**: Batch barcode/label printing and multi-up sheet PDF layouts (Avery 5160 30-up Letter, Avery 24-up A4, and thermal printer streaming).
  * **LEDGER-17**: Hardware BLE barcode scanner & digital weight scale integration (SPP/HID over GATT, live tare/gross streaming, piece counting).
  * **LEDGER-18**: Multi-currency accounting & offline FX rate tables (dual-currency GL entries, automated realized FX gain/loss journal adjustments).
  * **LEDGER-19**: Recurring subscription invoicing & client retainer deposit tracking (automated billing schedules, deferred revenue drawdown against timesheets).
  * **LEDGER-20**: Advanced inventory Pareto ABC analysis, aging dead-stock heatmaps, and dynamic Wilson EOQ & safety stock calculation.
  * **LEDGER-21**: Database encryption at rest (transparent ChaCha20 / SQLCipher) & authenticated P2P LAN sync via Pre-Shared Key (PSK) and QR code pairing (`erp://pair?...`).
  * **LEDGER-22**: On-device offline OCR receipt and vendor invoice intake (heuristic parser with bounding-box extraction, auto-draft purchase orders).
  * **LEDGER-23**: Individual serialized asset tracking, post-sale warranty validation, calibration tracking, and RMA claim workflows (`erp://serial/<sn>`).
  * **LEDGER-24**: Offline POS cashier mode with touch-friendly register, cash drawer solenoid kick (`ESC p 0 25 250`), receipt printing, and X/Z shift reconciliation.
  * **LEDGER-25**: Multi-warehouse stock transfer orders, in-transit virtual holding locations, and dispatch/arrival mobile barcode scanning with variance tracking.
  * **LEDGER-26**: Workstation capacity planning, machine operational calendar hours, and interactive Gantt / timeline production scheduling in Manufacturing.
  * **LEDGER-27**: Automated local network NAS/SMB/SFTP backup scheduler with WAL checkpointing (`PRAGMA wal_checkpoint(TRUNCATE)`), zstandard compression, and 1-click external USB drive sync.
  * **LEDGER-28**: European B2B e-invoicing compliance with hybrid PDF/A-3 Factur-X / ZUGFeRD 2.2 XML embedding (`factur-x.xml`) and offline EN 16931 Schematron validation.
* **v0.0.3**:
  * **LEDGER-11**: Offline pure vector PDF-1.4 generation for Invoices and Purchase Orders with European EPC QR GiroCode (SEPA QR) and fixed-point Decimal arithmetic.
  * **LEDGER-12**: Peer-to-peer local network database synchronization over LAN/mDNS using Hybrid Logical Clock (HLC) and CRDT event log convergence.
  * **LEDGER-13**: Automated low-stock monitoring against `min_stock_level` thresholds with 1-click draft PO generation and replenishment.
  * **LEDGER-14**: Interactive drag-and-drop Kanban workflow boards for Projects and Work Orders with status swimlanes and quick timesheet logging chips.
  * **LEDGER-15**: Lot expiration date tracking, QA quarantine states, putaway validation, and multi-level component genealogy tree visualization.
  * **Git Hooks**: Integrated tracked pre-commit (`cargo check`) and pre-push (`cargo test`) hooks in `.githooks/`.
* **v0.0.2**:
  * Automated native UniFFI packaging (`compileRustLib`) for Desktop & Mobile.
  * Interactive drag-and-drop floor plan canvas with persistent 2D coordinate updates in SQLite.
  * Native camera barcode scanning on Android (CameraX/ML Kit) and iOS (AVFoundation).
  * Direct thermal printing to ESC/POS, TSPL, and OS print spooler (Brother 62mm rolls & Avery sheets).
  * SQLite database backup with safe WAL flush, and CSV/JSON export/import.
  * In-app WASM plugin installer and dynamic slot UI manager.
* **v0.0.1**: Initial multi-module core architecture, Mozilla UniFFI bridging, basic Compose Multiplatform navigation, double-entry GL, and spatial inventory hierarchy.

---

## 🗺️ Roadmap & Upcoming Features (v0.0.5)

* Mobile biometric authentication integration (FaceID / Fingerprint) via platform Keystores.
* Mesh relay synchronization for air-gapped field vans and offline pop-up workshops.
* AI-assisted supplier price fluctuation detection and procurement forecasting.
* Direct CAD / STEP file component visualization in BOM genealogy trees.


