<div align="center">
  <h1>ledger</h1>

  <p><strong>An easy-to-use, local-first ERP application for craftsmen, workshops, and small manufacturers.</strong></p>

  <p>
    <a href="#credits"><img alt="Lead developer" src="https://img.shields.io/badge/lead%20developer-Zsombor%20Simon-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/ledger"><img alt="Repository" src="https://img.shields.io/badge/forgejo-git.soventa.tech-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/ledger/releases"><img alt="Release" src="https://img.shields.io/badge/release-v0.0.4-58a9ff" /></a>
    <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-LGPL--2.1-9e6bfa" /></a>
  </p>
</div>

ledger is a multi-platform, local-first ERP and warehouse management application built around an embedded SQLite engine and domain modules written in Rust, coupled with a Compose Multiplatform interface. It features a modern and intuitive workflow for spatial inventory, double-entry accounting, purchasing, manufacturing, invoicing, and peer-to-peer sync without cloud lock-in.

## Features

- **Spatial inventory hierarchy** - self-referencing warehouse tree (`Warehouse` → `Room` → `Rack` → `Shelf` → `Bin`) with instant search and shelf-level indicators
- **Floor plan radar** - interactive 2D drag-and-drop canvas for rooms and racks with coordinate persistence in SQLite
- **Rapid scan-to-move** - high-velocity 2-tap stock transfer HUD, cycle count audit mode, and camera/hardware barcode scanning
- **Thermal label printing** - direct print station for Brother 62mm rolls, Avery 5160 / 24-up sheets, ESC/POS, and TSPL printer queues
- **Manufacturing & BOM** - multi-level Bills of Materials, component consumption, workstation capacity planning, and Gantt scheduling
- **Component genealogy** - lot expiration tracking, QA quarantine validation, and end-to-end component genealogy trees
- **Kanban workflow boards** - drag-and-drop status swimlanes for work orders and project tasks with time tracking chips
- **Invoicing & Purchasing** - fixed-point Decimal arithmetic, automated low-stock reorder PO generation, and recurring subscriptions
- **Vector PDF generation** - offline PDF-1.4 generation with European EPC QR (GiroCode) and PDF/A-3 Factur-X / ZUGFeRD 2.2 XML
- **Point of Sale (POS)** - offline touch-friendly cashier register, split tenders, cash drawer kick pulse (`ESC p`), and shift X/Z reports
- **Double-entry general ledger** - Chart of Accounts, trial balance, P&L, multi-currency accounting, and realized FX gain/loss
- **Serialized asset tracking** - individual serial numbers, service histories, maintenance logs, and RMA claim workflows
- **Multi-warehouse transfers** - two-step dispatch and receipt workflows with virtual in-transit inventory tracking
- **Offline OCR intake** - on-device receipt and vendor invoice intake with bounding-box verification
- **Local-first P2P sync** - peer discovery over LAN/mDNS using Hybrid Logical Clock (HLC) and CRDT event convergence with PSK/QR pairing
- **Automated backups** - safe SQLite WAL flush checkpoints, compressed `.db.zst` snapshots, NAS/SFTP sync, and 1-click USB drive sync
- **Plugin system** - sandboxed WebAssembly runtime (Wasmtime) with manifest parsing and dynamic UI slot injection
- **Cross-platform** - native desktop (Linux, macOS, Windows) and mobile (Android, iOS) via Compose Multiplatform

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Kernel | Rust (`crates/erp-kernel`) - SQLite, migrations, event log, CRDT/HLC sync, backup engine |
| FFI Bridge | Rust (`crates/erp-ffi`) + Mozilla UniFFI (C-ABI / JNA) |
| UI Framework | Kotlin + Compose Multiplatform 1.6 (Desktop, Android, iOS) |
| Database | SQLite 3 (`rusqlite`) with WAL mode + ChaCha20 encryption at rest |
| Accounting | `rust_decimal` fixed-point 128-bit arithmetic |
| PDF Engine | Pure Rust vector PDF-1.4 generator + PDF/A-3 Factur-X XML |
| Barcode & QR | `qrcode` + native platform bindings (CameraX, AVFoundation, ESC/POS) |
| P2P & Discovery | LAN / mDNS zero-conf + CRDT event log convergence |
| Plugins | WebAssembly via Wasmtime (`wasm32-wasip1`) |

## Project Structure

```
ledger/
├── .githooks/                # Tracked git pre-commit and pre-push hooks
├── Cargo.toml                # Rust workspace configuration
├── crates/
│   ├── erp-kernel/           # Core engine: SQLite migrations, event log, CRDT/HLC sync,
│   │                         #   crypto, backup scheduler, WASM runtime
│   ├── erp-ffi/              # UniFFI interface, DTOs, vector PDF generator, FFI engine
│   ├── erp-server/           # Headless Axum server & management console
│   ├── erp-cli/              # Terminal CLI (ledger-cli) for offline warehouse operations
│   └── modules/              # Modular domain business engines
│       ├── mod-accounting/   # Chart of accounts, double-entry GL, trial balance, P&L, FX
│       ├── mod-contacts/     # Customers, suppliers, vendors, and CRM directory
│       ├── mod-inventory/    # Spatial hierarchy, stock levels, lot tracking, transfers
│       ├── mod-invoicing/    # Invoices, lines, recurring billing, Factur-X XML
│       ├── mod-manufacturing/# BOMs, work orders, component genealogy, workstations
│       ├── mod-projects/     # Projects, timesheets, Kanban boards, unbilled summaries
│       └── mod-purchasing/   # Purchase orders, OCR intake, goods receipts, reorders
├── composeApp/               # Compose Multiplatform UI (Desktop, Android, iOS)
│   ├── build.gradle.kts      # Gradle configuration & compileRustLib automation
│   └── src/
│       ├── commonMain/       # Shared UI features, screens, navigation, ErpBridge
│       │   └── kotlin/
│       │       ├── core/     # ErpBridge UniFFI wrapper & state cache
│       │       ├── features/ # Screens: inventory, accounting, manufacturing, etc.
│       │       └── uniffi/   # Auto-generated UniFFI Kotlin bindings
│       ├── desktopMain/      # Desktop entrypoint, JNA library loader, thermal printing
│       ├── androidMain/      # CameraX & ML Kit mobile barcode scanner
│       ├── iosMain/          # AVFoundation iOS barcode scanner
│       └── desktopTest/      # Desktop feature & integration test suite
└── plugins/                  # WebAssembly extension plugins (sample-supplier-plugin)
```

## Keybinds

| Keys | Action |
|------|--------|
| `Ctrl F` | Item Locator Screen |
| `Ctrl S` | Rapid Scan & Move HUD |
| `Ctrl T` | Storage Location Tree & Floor Plan |
| `Ctrl P` | QR Label Printing Station |
| `Ctrl I` | Invoicing Screen & Vector PDF Export |
| `Ctrl A` | Accounting / General Ledger |
| `Ctrl B` | Purchasing & Purchase Orders |
| `Ctrl O` | Manufacturing & Work Order Kanban |
| `Ctrl J` | Projects & Timesheet Kanban |
| `Ctrl M` | Settings, P2P Sync & Plugins |
| `Esc` | Clear active context location / close dialogs |

All keybinds are globally registered on desktop platforms for rapid warehouse inventory intake, status transitions, and navigation.

## Building

### Prerequisites

- Rust toolchain (`rustup`, 1.80+)
- JDK 17+ (e.g. OpenJDK 21 for Compose Multiplatform Desktop)
- Android SDK (for mobile builds)

### Development

```bash
git clone https://git.soventa.tech/moha/ledger.git
cd ledger

# Configure tracked git hooks
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push

# Launch the desktop app (automatically compiles native Rust library via compileRustLib)
cd composeApp
./gradlew run
```

### Running Tests

```bash
# Run Rust workspace unit and integration tests
cargo test --workspace

# Run Kotlin Compose desktop integration test suite
cd composeApp
./gradlew desktopTest
```

### Production Build

```bash
cd composeApp

# Produces standalone desktop release JAR
./gradlew desktopJar
```

### Building WASM Plugins

```bash
cd plugins/sample-supplier-plugin
cargo build --target wasm32-wasip1 --release
```

Open the **Settings** view in Ledger ERP, provide the compiled `.wasm` file path, and click **Install & Activate .wasm Module** to hot-reload the plugin into the UI.

## Release History

- **v0.0.4** - Batch barcode printing (LEDGER-16), BLE hardware scanners & digital scales (LEDGER-17), multi-currency accounting (LEDGER-18), recurring subscription invoicing (LEDGER-19), ABC Pareto inventory & dead-stock heatmaps (LEDGER-20), database encryption at rest & PSK/QR P2P sync (LEDGER-21), offline OCR receipt intake (LEDGER-22), serialized asset tracking & RMA (LEDGER-23), offline POS cashier register (LEDGER-24), multi-warehouse stock transfer orders (LEDGER-25), workstation capacity planning & Gantt scheduling (LEDGER-26), automated NAS/USB backups (LEDGER-27), European e-invoicing Factur-X / ZUGFeRD 2.2 (LEDGER-28)
- **v0.0.3** - Vector PDF generation for invoices & POs with EPC QR GiroCode (LEDGER-11), LAN/mDNS P2P sync (LEDGER-12), automated low-stock reorder alerts (LEDGER-13), interactive Kanban boards (LEDGER-14), lot expiration tracking & component genealogy tree (LEDGER-15), tracked git hooks
- **v0.0.2** - Automated native UniFFI packaging (`compileRustLib`), drag-and-drop 2D floor plan radar, camera barcode scanning on Android/iOS, thermal label printing (ESC/POS & TSPL), SQLite backup & export/import, in-app WASM plugin manager
- **v0.0.1** - Initial multi-module core architecture, Mozilla UniFFI bridging, basic Compose Multiplatform navigation, double-entry GL, and spatial inventory hierarchy

## Roadmap

- **LEDGER-29** - Mobile biometric authentication integration (FaceID / Fingerprint) via platform Keystores
- **LEDGER-30** - Mesh relay synchronization for air-gapped field vans and offline pop-up workshops
- **LEDGER-32** - Direct CAD / STEP file component visualization in BOM genealogy trees

## License

Licensed under the GNU Lesser General Public License v2.1:

- [LICENSE](LICENSE)

## Links

- [Documentation](https://docs.soventa.tech/books/ledger)
- [Tasks](https://tasks.soventa.tech/share/VujOFE3PZ2LSqZDRAOPwF3HgjHxaL9rVYMmQIq6q/auth?)
