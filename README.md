# Ledger ERP

> An easy-to-use, local-first, modular Enterprise Resource Planning system built for sole proprietors, workshops, craftsmen, and small businesses.

Unlike traditional monolithic or cloud-hosted ERPs that require expensive servers, external databases, continuous internet connectivity, and complex IT infrastructure, **Ledger ERP** is designed from the ground up to be **offline-first**, **self-contained**, and **blazingly fast**.

---

## ⚡ Key Highlights & Capabilities

* **Local-First & Zero-Cloud Dependency**: Runs on an embedded **SQLite** engine operating with Write-Ahead Logging (WAL) for microsecond queries and offline durability.
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
  * Direct binary ESC/POS and plain-text TSPL/ZPL command generation.
  * Native QR code commands (`GS ( k`) and bold typography formatting.
  * Supports standard thermal rolls (Brother 62mm continuous DK-22205 / die-cut DK-11209), standard Avery die-cut sheets (5160 / 24-up), and POS receipt printers (58mm/80mm).
  * Dispatches to system print dialog (OS spooler) or raw serial/USB devices (`/dev/usb/lp0`).
* **Double-Entry General Ledger & Invoicing**:
  * Strict accounting invariants: total debits must equal total credits before commitment.
  * Pre-seeded standard Chart of Accounts (Assets, Liabilities, Equity, Revenue, Expenses).
  * Real-time Trial Balance and Profit & Loss generation.
  * Sales invoicing with fixed-point `rust_decimal::Decimal` precision (zero floating-point errors).
* **Manufacturing (BOMs) & Time Tracking**:
  * Multi-level Bills of Materials (BOM) with automatic component stock deduction upon work order execution.
  * Project timesheet tracking with one-click conversion of unbilled hours into sales invoices.
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
│  - Spatial Inventory HUD  - Invoicing  - Manufacturing (BOM)    │
│  - Accounting / GL        - Purchasing - Dynamic WASM Plugins   │
│  - Interactive Floor Plan - Camera Barcode Scanner (CameraX/AV) │
│  - Thermal Label Station  - Local Backup & CSV/JSON Portability │
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
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ SQLite Database Engine (WAL Mode, rusqlite with safe backup)│ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📦 Project Structure

```
ledger/
├── crates/
│   ├── erp-kernel/           # SQLite connection pool, HLC clock, event log, module lifecycle
│   ├── erp-plugin-host/      # Extism WebAssembly runtime & declarative UI layout engine
│   ├── erp-ffi/              # Mozilla UniFFI bindings & ErpEngine high-level facade
│   ├── erp-server/           # Headless Axum server & web management console
│   ├── erp-cli/              # Terminal CLI (ledger-cli) for offline warehouse operations
│   └── modules/
│       ├── mod-inventory/    # Location tree, stock ledger, QR codes, spatial floor plan
│       ├── mod-contacts/     # Customer, vendor, and supplier directory
│       ├── mod-invoicing/    # Invoices, lines, tax calculations, and status transitions
│       ├── mod-accounting/   # Chart of accounts, double-entry GL, trial balance, P&L
│       ├── mod-purchasing/   # Purchase orders and goods receipts
│       ├── mod-manufacturing/# Bills of materials (BOM) and work order execution
│       └── mod-projects/     # Projects, timesheet logs, and automated invoicing
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

### 1. Build Core Rust Crates
```bash
# Build all crates in release mode
cargo build --release

# Run Rust unit and integration test suite (19 tests)
cargo test --workspace
```

### 2. Run Desktop Application
The Gradle build automatically compiles the Rust FFI library and packages `liberp_ffi.so` directly into the classpath:

```bash
cd composeApp

# Launch desktop app (automatically runs compileRustLib)
./gradlew run
```

### 3. Run Desktop Integration Tests
```bash
cd composeApp

# Runs NativeLoadTest, ThermalPrintingTest, and BackupAndPluginTest
./gradlew desktopTest
```

### 4. Build Standalone Desktop JAR
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
| `Ctrl / Cmd + I` | Invoicing Screen |
| `Ctrl / Cmd + A` | Accounting / General Ledger |
| `Ctrl / Cmd + B` | Purchasing & POs |
| `Ctrl / Cmd + O` | Manufacturing & BOMs |
| `Ctrl / Cmd + J` | Projects & Timesheets |
| `Ctrl / Cmd + M` | Settings, Backup & Plugins |
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

* **v0.0.2**:
  * Automated native UniFFI packaging (`compileRustLib`) for Desktop & Mobile.
  * Interactive drag-and-drop floor plan canvas with persistent 2D coordinate updates in SQLite.
  * Native camera barcode scanning on Android (CameraX/ML Kit) and iOS (AVFoundation).
  * Direct thermal printing to ESC/POS, TSPL, and OS print spooler (Brother 62mm rolls & Avery sheets).
  * SQLite database backup with safe WAL flush, and CSV/JSON export/import.
  * In-app WASM plugin installer and dynamic slot UI manager.
* **v0.0.1**: Initial multi-module core architecture, Mozilla UniFFI bridging, basic Compose Multiplatform navigation, double-entry GL, and spatial inventory hierarchy.
