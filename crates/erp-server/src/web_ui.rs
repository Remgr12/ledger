pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Ledger ERP — Headless Server & Web Console</title>
  <style>
    :root {
      --bg-primary: #0f172a;
      --bg-secondary: #1e293b;
      --bg-surface: #334155;
      --accent: #38bdf8;
      --accent-hover: #0ea5e9;
      --text-main: #f8fafc;
      --text-muted: #94a3b8;
      --success: #22c55e;
      --warning: #f59e0b;
      --danger: #ef4444;
      --border: #475569;
      --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: var(--bg-primary);
      color: var(--text-main);
      font-family: var(--font-sans);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }
    header {
      background-color: var(--bg-secondary);
      border-bottom: 1px solid var(--border);
      padding: 0.75rem 1.5rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
      position: sticky;
      top: 0;
      z-index: 50;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }
    .brand h1 {
      font-size: 1.25rem;
      font-weight: 800;
      letter-spacing: 0.05em;
      color: var(--accent);
    }
    .badge {
      font-size: 0.7rem;
      font-weight: 700;
      text-transform: uppercase;
      padding: 0.2rem 0.5rem;
      border-radius: 9999px;
      background: var(--bg-surface);
      color: var(--text-muted);
    }
    .badge.green { background: rgba(34, 197, 94, 0.15); color: var(--success); border: 1px solid var(--success); }
    .badge.blue { background: rgba(56, 189, 248, 0.15); color: var(--accent); border: 1px solid var(--accent); }
    .badge.amber { background: rgba(245, 158, 11, 0.15); color: var(--warning); border: 1px solid var(--warning); }
    
    .nav-tabs {
      display: flex;
      gap: 0.5rem;
      background-color: var(--bg-secondary);
      padding: 0.5rem 1.5rem;
      border-bottom: 1px solid var(--border);
      overflow-x: auto;
    }
    .tab-btn {
      background: transparent;
      border: none;
      color: var(--text-muted);
      padding: 0.5rem 0.9rem;
      border-radius: 6px;
      cursor: pointer;
      font-size: 0.875rem;
      font-weight: 600;
      white-space: nowrap;
      transition: all 0.15s ease;
    }
    .tab-btn:hover {
      background: var(--bg-surface);
      color: var(--text-main);
    }
    .tab-btn.active {
      background: var(--accent);
      color: #0f172a;
    }

    main {
      flex: 1;
      padding: 1.5rem;
      max-width: 1400px;
      margin: 0 auto;
      width: 100%;
    }
    .tab-content { display: none; }
    .tab-content.active { display: block; }

    .card {
      background: var(--bg-secondary);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1.25rem;
      margin-bottom: 1.25rem;
    }
    .card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1rem;
      border-bottom: 1px solid var(--border);
      padding-bottom: 0.75rem;
    }
    .card-title {
      font-size: 1.1rem;
      font-weight: 700;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .card-desc {
      font-size: 0.8rem;
      color: var(--text-muted);
      margin-top: 0.2rem;
    }

    .grid-2 { display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 1rem; }
    .grid-3 { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1rem; }

    .search-box {
      display: flex;
      gap: 0.75rem;
      margin-bottom: 1.25rem;
    }
    input, select, textarea {
      background: var(--bg-primary);
      border: 1px solid var(--border);
      color: var(--text-main);
      padding: 0.6rem 0.9rem;
      border-radius: 6px;
      font-size: 0.9rem;
      width: 100%;
      outline: none;
    }
    input:focus, select:focus, textarea:focus {
      border-color: var(--accent);
      box-shadow: 0 0 0 1px var(--accent);
    }
    button.btn {
      background: var(--accent);
      color: #0f172a;
      border: none;
      padding: 0.6rem 1.2rem;
      border-radius: 6px;
      font-weight: 600;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      transition: background 0.15s ease;
      white-space: nowrap;
    }
    button.btn:hover { background: var(--accent-hover); }
    button.btn-outline {
      background: transparent;
      color: var(--accent);
      border: 1px solid var(--accent);
    }
    button.btn-outline:hover {
      background: rgba(56, 189, 248, 0.1);
    }
    button.btn-sm {
      padding: 0.35rem 0.65rem;
      font-size: 0.8rem;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;
    }
    th, td {
      padding: 0.75rem 0.9rem;
      text-align: left;
      border-bottom: 1px solid var(--border);
    }
    th {
      background: var(--bg-primary);
      color: var(--text-muted);
      font-weight: 600;
      text-transform: uppercase;
      font-size: 0.75rem;
      letter-spacing: 0.05em;
    }
    tr:hover { background: rgba(255, 255, 255, 0.02); }
    .mono { font-family: var(--font-mono); font-size: 0.85rem; }

    .breadcrumb {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      color: var(--accent);
      font-weight: 600;
      font-size: 0.85rem;
      background: rgba(56, 189, 248, 0.1);
      padding: 0.25rem 0.6rem;
      border-radius: 4px;
    }
    .hud-box {
      border: 2px dashed var(--accent);
      background: rgba(56, 189, 248, 0.05);
      border-radius: 8px;
      padding: 1.25rem;
      text-align: center;
      margin-bottom: 1rem;
    }

    .modal-overlay {
      display: none;
      position: fixed;
      top: 0; left: 0; right: 0; bottom: 0;
      background: rgba(0, 0, 0, 0.75);
      z-index: 100;
      align-items: center;
      justify-content: center;
      padding: 1rem;
    }
    .modal-overlay.active { display: flex; }
    .modal {
      background: var(--bg-secondary);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1.5rem;
      max-width: 550px;
      width: 100%;
    }
    .modal-actions {
      display: flex;
      justify-content: flex-end;
      gap: 0.75rem;
      margin-top: 1.25rem;
    }
    .status-toast {
      position: fixed;
      bottom: 1.5rem;
      right: 1.5rem;
      background: var(--bg-secondary);
      border: 1px solid var(--success);
      color: var(--text-main);
      padding: 0.75rem 1.25rem;
      border-radius: 6px;
      display: none;
      box-shadow: 0 10px 25px rgba(0,0,0,0.5);
      z-index: 1000;
    }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <h1>⚡ LEDGER ERP</h1>
      <span class="badge blue">HEADLESS SERVER</span>
      <span class="badge green" id="server-status">ONLINE (HLC ACTIVE)</span>
    </div>
    <div style="display: flex; align-items: center; gap: 1rem;">
      <span class="mono" style="font-size: 0.75rem; color: var(--text-muted);" id="hlc-time">HLC: sync-ready</span>
      <button class="btn btn-outline btn-sm" onclick="fetchHealth()">↻ Ping Server</button>
    </div>
  </header>

  <nav class="nav-tabs">
    <button class="tab-btn active" onclick="switchTab('locator')">🔍 Where Is It?</button>
    <button class="tab-btn" onclick="switchTab('scan')">📷 QR & Scan HUD</button>
    <button class="tab-btn" onclick="switchTab('tree')">🌳 Storage Tree</button>
    <button class="tab-btn" onclick="switchTab('purchasing')">🛒 Purchasing</button>
    <button class="tab-btn" onclick="switchTab('manufacturing')">⚙️ Manufacturing</button>
    <button class="tab-btn" onclick="switchTab('projects')">⏱️ Projects</button>
    <button class="tab-btn" onclick="switchTab('invoicing')">🧾 Invoicing</button>
    <button class="tab-btn" onclick="switchTab('accounting')">⚖️ General Ledger</button>
    <button class="tab-btn" onclick="switchTab('labels')">🏷️ QR Labels</button>
    <button class="tab-btn" onclick="switchTab('system')">🔌 Modules & Sync</button>
  </nav>

  <main>
    <!-- 1. LOCATOR TAB -->
    <div id="tab-locator" class="tab-content active">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">🔍 Spatial Inventory Search ("Where Is It?")</div>
            <div class="card-desc">Type SKU, item name, lot number, or scan code to find physical storage coordinates and breadcrumbs.</div>
          </div>
        </div>
        <div class="search-box">
          <input type="text" id="locator-query" placeholder="Search by SKU (e.g. M8-HEX-BOLT), item name, or lot..." value="M8" oninput="runLocatorSearch()">
          <button class="btn" onclick="runLocatorSearch()">Search</button>
        </div>
        <div id="locator-results">
          <!-- Dynamic cards -->
        </div>
      </div>
    </div>

    <!-- 2. SCAN HUD TAB -->
    <div id="tab-scan" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">📷 Dual-Entity Smart QR & Rapid Action HUD</div>
            <div class="card-desc">Scan shelf/bin QR (<span class="mono">erp://loc/...</span>) to target a location, or item QR (<span class="mono">erp://item/...</span>) to move stock.</div>
          </div>
        </div>
        <div class="hud-box">
          <div style="font-weight: 700; margin-bottom: 0.5rem;">Target Location Context:</div>
          <div id="active-location-display" style="font-size: 1.1rem; color: var(--accent);">
            None (Scan a Location QR or select from tree to activate rapid putaway)
          </div>
          <div style="margin-top: 0.5rem;" id="clear-loc-btn-wrap" style="display:none;">
            <button class="btn btn-outline btn-sm" onclick="clearActiveLocation()">Clear Target Location</button>
          </div>
        </div>

        <div class="search-box">
          <input type="text" id="scan-payload-input" placeholder="Enter QR raw payload (e.g. erp://loc/loc-bin-4 or erp://item/M8-HEX-BOLT-316)..." value="erp://loc/loc-bin-4">
          <button class="btn" onclick="resolveQrScan()">Resolve Scan</button>
        </div>

        <div id="scan-resolution-card" style="display: none;" class="card">
          <!-- Resolution details -->
        </div>
      </div>
    </div>

    <!-- 3. STORAGE TREE TAB -->
    <div id="tab-tree" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">🌳 Physical Storage Hierarchy</div>
            <div class="card-desc">Infinitely nestable location tree with breadcrumbs and fast putaway context switching.</div>
          </div>
          <button class="btn btn-sm" onclick="loadLocationTree()">↻ Refresh Tree</button>
        </div>
        <div id="location-tree-container">
          Loading tree hierarchy...
        </div>
      </div>
    </div>

    <!-- 4. PURCHASING TAB -->
    <div id="tab-purchasing" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">🛒 Purchase Orders & Direct Putaway</div>
            <div class="card-desc">Vendor procurement orders with direct goods receipt into storage bins.</div>
          </div>
          <button class="btn btn-sm" onclick="loadPurchaseOrders()">↻ Refresh Orders</button>
        </div>
        <div id="po-table-container">
          Loading orders...
        </div>
      </div>
    </div>

    <!-- 5. MANUFACTURING TAB -->
    <div id="tab-manufacturing" class="tab-content">
      <div class="grid-2">
        <div class="card">
          <div class="card-header">
            <div>
              <div class="card-title">⚙️ Bill of Materials (BOM)</div>
              <div class="card-desc">Multi-level assembly component specifications.</div>
            </div>
            <button class="btn btn-sm" onclick="loadBoms()">↻ Refresh</button>
          </div>
          <div id="boms-container">Loading BOMs...</div>
        </div>

        <div class="card">
          <div class="card-header">
            <div>
              <div class="card-title">🏭 Work Orders</div>
              <div class="card-desc">Atomic component consumption & finished good receipts.</div>
            </div>
            <button class="btn btn-sm" onclick="loadWorkOrders()">↻ Refresh</button>
          </div>
          <div id="work-orders-container">Loading Work Orders...</div>
        </div>
      </div>
    </div>

    <!-- 6. PROJECTS TAB -->
    <div id="tab-projects" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">⏱️ Projects & Billable Timesheets</div>
            <div class="card-desc">Track project hours and convert unbilled time into sales invoices in 1 click.</div>
          </div>
          <button class="btn btn-sm" onclick="loadProjects()">↻ Refresh</button>
        </div>
        <div id="projects-container">Loading projects...</div>
      </div>
    </div>

    <!-- 7. INVOICING TAB -->
    <div id="tab-invoicing" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">🧾 Sales Invoices</div>
            <div class="card-desc">Customer invoices with automated stock deduction and AR posting.</div>
          </div>
          <button class="btn btn-sm" onclick="loadInvoices()">↻ Refresh Invoices</button>
        </div>
        <div id="invoices-container">Loading invoices...</div>
      </div>
    </div>

    <!-- 8. ACCOUNTING TAB -->
    <div id="tab-accounting" class="tab-content">
      <div class="grid-2">
        <div class="card">
          <div class="card-header">
            <div>
              <div class="card-title">⚖️ Chart of Accounts</div>
              <div class="card-desc">Standard 5-category double-entry ledger accounts.</div>
            </div>
            <button class="btn btn-sm" onclick="loadChartOfAccounts()">↻ Refresh</button>
          </div>
          <div id="accounts-container">Loading accounts...</div>
        </div>

        <div class="card">
          <div class="card-header">
            <div>
              <div class="card-title">📊 Trial Balance & Balance Verification</div>
              <div class="card-desc">Mathematical invariant: Sum(Debit) == Sum(Credit).</div>
            </div>
            <button class="btn btn-sm" onclick="loadTrialBalance()">↻ Refresh</button>
          </div>
          <div id="trial-balance-container">Loading trial balance...</div>
        </div>
      </div>
    </div>

    <!-- 9. QR LABELS TAB -->
    <div id="tab-labels" class="tab-content">
      <div class="card">
        <div class="card-header">
          <div>
            <div class="card-title">🏷️ Offline QR Thermal Label Generator</div>
            <div class="card-desc">Crisp SVG vector labels ready for Zebra/Brother thermal printers or standard paper.</div>
          </div>
        </div>
        <div class="search-box">
          <input type="text" id="label-sku-input" placeholder="SKU or Location code..." value="M8-HEX-BOLT-316">
          <input type="text" id="label-title-input" placeholder="Title..." value="M8x40mm Hex Bolt Stainless 316">
          <button class="btn" onclick="generateSvgLabel()">Generate Label</button>
        </div>
        <div style="background: white; padding: 2rem; border-radius: 8px; display: inline-block;" id="svg-preview-box">
          <div style="color: black; font-family: sans-serif; text-align: center;">Click "Generate Label" to preview SVG</div>
        </div>
      </div>
    </div>

    <!-- 10. SYSTEM & SYNC TAB -->
    <div id="tab-system" class="tab-content">
      <div class="grid-2">
        <div class="card">
          <div class="card-header">
            <div class="card-title">🔌 Active ERP Modules</div>
          </div>
          <div id="modules-list">Loading modules...</div>
        </div>
        <div class="card">
          <div class="card-header">
            <div class="card-title">📡 Hybrid Logical Clock (HLC) Sync Stream</div>
          </div>
          <div id="sync-stream">
            <p style="color: var(--text-muted); font-size: 0.9rem; margin-bottom: 0.75rem;">
              The headless server records monotonic HLC-stamped append-only domain events. Multiplatform clients sync peer-to-peer via <span class="mono">/api/v1/sync/pull</span> and <span class="mono">/api/v1/sync/push</span>.
            </p>
            <button class="btn btn-sm" onclick="fetchSyncEvents()">Pull Latest HLC Events</button>
            <div id="sync-events-container" style="margin-top: 1rem;"></div>
          </div>
        </div>
      </div>
    </div>
  </main>

  <div class="status-toast" id="toast">Operation successful</div>

  <!-- Modal for Goods Receipt Putaway -->
  <div class="modal-overlay" id="receipt-modal">
    <div class="modal">
      <h3 style="margin-bottom: 0.75rem;">Receive Goods & Direct Putaway</h3>
      <p style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 1rem;" id="receipt-modal-desc"></p>
      <div style="margin-bottom: 1rem;">
        <label style="font-size: 0.8rem; font-weight: 600; display: block; margin-bottom: 0.3rem;">Target Storage Bin:</label>
        <select id="receipt-destination-select"></select>
      </div>
      <div class="modal-actions">
        <button class="btn btn-outline" onclick="closeReceiptModal()">Cancel</button>
        <button class="btn" onclick="confirmGoodsReceipt()">Confirm & Record Stock Movement</button>
      </div>
    </div>
  </div>

  <script>
    let activeLocation = null;
    let cachedLocations = [];
    let activePoForReceipt = null;

    function showToast(msg) {
      const toast = document.getElementById('toast');
      toast.innerText = msg;
      toast.style.display = 'block';
      setTimeout(() => { toast.style.display = 'none'; }, 3000);
    }

    function switchTab(tabId) {
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
      event.target.classList.add('active');
      document.getElementById('tab-' + tabId).classList.add('active');

      if (tabId === 'locator') runLocatorSearch();
      if (tabId === 'tree') loadLocationTree();
      if (tabId === 'purchasing') loadPurchaseOrders();
      if (tabId === 'manufacturing') { loadBoms(); loadWorkOrders(); }
      if (tabId === 'projects') loadProjects();
      if (tabId === 'invoicing') loadInvoices();
      if (tabId === 'accounting') { loadChartOfAccounts(); loadTrialBalance(); }
      if (tabId === 'labels') generateSvgLabel();
      if (tabId === 'system') { loadModules(); fetchSyncEvents(); }
    }

    async function fetchHealth() {
      try {
        const res = await fetch('/api/v1/health');
        const data = await res.json();
        document.getElementById('server-status').innerText = 'ONLINE (' + data.status.toUpperCase() + ')';
        document.getElementById('hlc-time').innerText = 'HLC: ' + (data.hlc || data.server_hlc || 'active');
        showToast('Server ping successful');
      } catch (e) {
        document.getElementById('server-status').innerText = 'OFFLINE';
      }
    }

    async function runLocatorSearch() {
      const query = document.getElementById('locator-query').value.trim();
      const container = document.getElementById('locator-results');
      try {
        const res = await fetch('/api/v1/inventory/locate?q=' + encodeURIComponent(query));
        const items = await res.json();
        if (!items || items.length === 0) {
          container.innerHTML = '<p style="color: var(--text-muted); padding: 1rem 0;">No matching inventory locations found.</p>';
          return;
        }
        let html = '<div class="grid-2">';
        items.forEach(card => {
          html += `
            <div class="card" style="background: var(--bg-primary); border-left: 4px solid var(--accent);">
              <div style="display: flex; justify-content: space-between; align-items: start;">
                <div>
                  <div class="mono" style="font-weight: 700; color: var(--accent);">${card.sku}</div>
                  <div style="font-size: 1rem; font-weight: 600; margin-top: 0.2rem;">${card.item_name}</div>
                </div>
                <span class="badge blue" style="font-size: 0.9rem;">${card.quantity} ${card.unit}</span>
              </div>
              <div style="margin: 0.75rem 0;">
                <div class="breadcrumb">📍 ${card.breadcrumb_path}</div>
              </div>
              <div style="display: flex; justify-content: space-between; font-size: 0.8rem; color: var(--text-muted);">
                <span>Lot: ${card.lot_number || 'Default'}</span>
                <span>Coordinates: (${card.map_x ?? '-'}, ${card.map_y ?? '-'})</span>
                <button class="btn btn-outline btn-sm" onclick="printFromLocator('${card.sku}', '${card.item_name}')">Print Label</button>
              </div>
            </div>
          `;
        });
        html += '</div>';
        container.innerHTML = html;
      } catch (e) {
        container.innerHTML = '<p style="color: var(--danger)">Error querying locator: ' + e + '</p>';
      }
    }

    async function resolveQrScan() {
      const payload = document.getElementById('scan-payload-input').value.trim();
      const targetCard = document.getElementById('scan-resolution-card');
      try {
        const res = await fetch('/api/v1/inventory/scan', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            payload: payload,
            context_location_id: activeLocation ? activeLocation.id : null
          })
        });
        const data = await res.json();
        targetCard.style.display = 'block';

        let html = `
          <div class="card-title">Scan Resolution: <span class="badge blue">${data.resolution_type}</span></div>
          <p style="margin: 0.5rem 0; font-size: 0.9rem;">${data.message}</p>
        `;

        if (data.location) {
          activeLocation = data.location;
          updateActiveLocationDisplay();
          html += `
            <div style="margin-top: 0.5rem;" class="breadcrumb">
              Target Location Set: ${data.location.name} (${data.location.path_breadcrumb})
            </div>
          `;
        }

        if (data.matched_items && data.matched_items.length > 0) {
          html += '<div style="margin-top: 0.75rem;"><strong>Inventory at this Location:</strong><ul>';
          data.matched_items.forEach(it => {
            html += `<li style="font-size: 0.85rem; margin-top: 0.2rem;">${it.item_name} (${it.sku}): <strong>${it.quantity} ${it.unit}</strong></li>`;
          });
          html += '</ul></div>';
        }

        targetCard.innerHTML = html;
        showToast('Scan resolved: ' + data.resolution_type);
      } catch (e) {
        showToast('Scan error: ' + e);
      }
    }

    function updateActiveLocationDisplay() {
      const disp = document.getElementById('active-location-display');
      const wrap = document.getElementById('clear-loc-btn-wrap');
      if (activeLocation) {
        disp.innerHTML = `<strong>[${activeLocation.code}]</strong> ${activeLocation.name} <br><span class="mono" style="font-size: 0.8rem;">${activeLocation.path_breadcrumb}</span>`;
        if (wrap) wrap.style.display = 'block';
      } else {
        disp.innerText = 'None (Scan a Location QR or select from tree to activate rapid putaway)';
        if (wrap) wrap.style.display = 'none';
      }
    }

    function clearActiveLocation() {
      activeLocation = null;
      updateActiveLocationDisplay();
      showToast('Active target location cleared');
    }

    async function loadLocationTree() {
      const container = document.getElementById('location-tree-container');
      try {
        const res = await fetch('/api/v1/inventory/tree');
        const tree = await res.json();
        cachedLocations = [];

        function renderNodes(nodes, depth = 0) {
          let h = '';
          nodes.forEach(n => {
            cachedLocations.push(n.location);
            const indent = depth * 24;
            h += `
              <div style="margin-left: ${indent}px; padding: 0.5rem 0.75rem; border-left: 2px solid var(--border); margin-bottom: 0.3rem; display: flex; justify-content: space-between; align-items: center; background: rgba(255,255,255,0.01); border-radius: 4px;">
                <div>
                  <span class="badge ${depth === 0 ? 'blue' : 'amber'}">${n.location.location_type}</span>
                  <strong style="margin-left: 0.5rem;">${n.location.name}</strong>
                  <span class="mono" style="color: var(--text-muted); margin-left: 0.5rem;">[${n.location.code}]</span>
                </div>
                <div>
                  <button class="btn btn-outline btn-sm" onclick="setContextLocationById('${n.location.id}')">Target This</button>
                </div>
              </div>
            `;
            if (n.children && n.children.length > 0) {
              h += renderNodes(n.children, depth + 1);
            }
          });
          return h;
        }

        container.innerHTML = renderNodes(tree);
      } catch (e) {
        container.innerHTML = '<p style="color: var(--danger)">Error loading tree: ' + e + '</p>';
      }
    }

    function setContextLocationById(locId) {
      const found = cachedLocations.find(l => l.id === locId);
      if (found) {
        activeLocation = found;
        updateActiveLocationDisplay();
        switchTab('scan');
        showToast('Set active target: ' + found.name);
      }
    }

    async function loadPurchaseOrders() {
      const container = document.getElementById('po-table-container');
      try {
        const res = await fetch('/api/v1/purchasing/orders');
        const pos = await res.json();
        if (!pos || pos.length === 0) {
          container.innerHTML = '<p style="color: var(--text-muted)">No purchase orders recorded.</p>';
          return;
        }
        let html = `
          <table>
            <thead>
              <tr>
                <th>PO Number</th>
                <th>Supplier</th>
                <th>Status</th>
                <th>Total (USD)</th>
                <th>Lines</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
        `;
        pos.forEach(po => {
          html += `
            <tr>
              <td class="mono"><strong>${po.po_number}</strong></td>
              <td>${po.supplier_id}</td>
              <td><span class="badge ${po.status === 'Received' ? 'green' : 'amber'}">${po.status}</span></td>
              <td class="mono"><strong>$${po.grand_total}</strong></td>
              <td>${po.lines ? po.lines.length : 0} item(s)</td>
              <td>
                ${po.status !== 'Received' ? `<button class="btn btn-sm" onclick="openReceiveModal('${po.id}')">Receive & Putaway</button>` : `<span style="color: var(--success); font-size: 0.8rem;">✓ Putaway Done</span>`}
              </td>
            </tr>
          `;
        });
        html += '</tbody></table>';
        container.innerHTML = html;
      } catch (e) {
        container.innerHTML = '<p style="color: var(--danger)">Error loading POs: ' + e + '</p>';
      }
    }

    async function openReceiveModal(poId) {
      activePoForReceipt = poId;
      document.getElementById('receipt-modal-desc').innerText = 'Select destination bin to automatically record inbound stock movements for PO: ' + poId;
      const select = document.getElementById('receipt-destination-select');
      select.innerHTML = '';
      if (cachedLocations.length === 0) {
        const res = await fetch('/api/v1/inventory/locations');
        cachedLocations = await res.json();
      }
      cachedLocations.forEach(loc => {
        const opt = document.createElement('option');
        opt.value = loc.id;
        opt.innerText = `[${loc.code}] ${loc.name} (${loc.location_type})`;
        select.appendChild(opt);
      });
      document.getElementById('receipt-modal').classList.add('active');
    }

    function closeReceiptModal() {
      document.getElementById('receipt-modal').classList.remove('active');
    }

    async function confirmGoodsReceipt() {
      const destId = document.getElementById('receipt-destination-select').value;
      try {
        const res = await fetch('/api/v1/purchasing/receive', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            po_id: activePoForReceipt,
            supplier_id: 'supp-1',
            destination_location_id: destId,
            items: [
              { item_id: 'item-1', quantity_received: '500', unit_cost: '2.90', lot_number: 'LOT-2026-Q1' }
            ],
            notes: 'Web Console Goods Receipt & Putaway'
          })
        });
        if (res.ok) {
          showToast('Goods received and putaway recorded in stock movements ledger!');
          closeReceiptModal();
          loadPurchaseOrders();
        } else {
          showToast('Failed to record receipt');
        }
      } catch (e) {
        showToast('Receipt error: ' + e);
      }
    }

    async function loadBoms() {
      const c = document.getElementById('boms-container');
      try {
        const res = await fetch('/api/v1/manufacturing/boms');
        const boms = await res.json();
        let h = '';
        boms.forEach(b => {
          h += `
            <div class="card" style="background: var(--bg-primary);">
              <div style="display: flex; justify-content: space-between;">
                <div>
                  <div class="mono" style="font-weight: 700; color: var(--accent);">${b.bom_number}</div>
                  <div style="font-weight: 600;">${b.name}</div>
                </div>
                <span class="badge blue">Batch: ${b.quantity}</span>
              </div>
              <div style="margin-top: 0.5rem; font-size: 0.8rem; color: var(--text-muted);">${b.routing_notes || 'Standard routing'}</div>
              <div style="margin-top: 0.5rem; font-size: 0.85rem;">Components: <strong>${b.components ? b.components.length : 0}</strong> parts</div>
            </div>
          `;
        });
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function loadWorkOrders() {
      const c = document.getElementById('work-orders-container');
      try {
        const res = await fetch('/api/v1/manufacturing/work-orders');
        const wos = await res.json();
        let h = '';
        wos.forEach(wo => {
          const isComp = wo.status.toLowerCase() === 'completed';
          h += `
            <div class="card" style="background: var(--bg-primary);">
              <div style="display: flex; justify-content: space-between; align-items: start;">
                <div>
                  <div class="mono" style="font-weight: 700;">${wo.work_order_number}</div>
                  <div style="font-size: 0.85rem; color: var(--text-muted);">Target: ${wo.target_quantity} units</div>
                </div>
                <span class="badge ${isComp ? 'green' : 'amber'}">${wo.status}</span>
              </div>
              <div style="margin-top: 0.75rem; display: flex; justify-content: flex-end;">
                ${!isComp ? `<button class="btn btn-sm" onclick="executeWorkOrder('${wo.id}')">Execute Assembly</button>` : `<span style="color: var(--success); font-size: 0.8rem;">✓ Finished Goods Received</span>`}
              </div>
            </div>
          `;
        });
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function executeWorkOrder(woId) {
      try {
        const res = await fetch(`/api/v1/manufacturing/work-orders/${woId}/execute`, { method: 'POST' });
        if (res.ok) {
          showToast('Work order executed: Components consumed and finished goods produced!');
          loadWorkOrders();
        } else {
          showToast('Execution failed: Check raw component stock.');
        }
      } catch (e) {
        showToast('Error: ' + e);
      }
    }

    async function loadProjects() {
      const c = document.getElementById('projects-container');
      try {
        const res = await fetch('/api/v1/projects');
        const prjs = await res.json();
        let h = '<div class="grid-2">';
        prjs.forEach(p => {
          h += `
            <div class="card" style="background: var(--bg-primary);">
              <div style="display: flex; justify-content: space-between;">
                <div>
                  <div class="mono" style="color: var(--accent); font-weight: 700;">${p.code}</div>
                  <div style="font-weight: 600; font-size: 1rem;">${p.name}</div>
                </div>
                <span class="badge blue">${p.status}</span>
              </div>
              <div style="margin: 0.5rem 0; font-size: 0.85rem; color: var(--text-muted);">
                Billing Rate: <strong>$${p.billing_rate_hourly || '100.00'}/hr</strong> | Budget: ${p.estimated_hours || 0} hrs
              </div>
              <div style="display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.75rem;">
                <button class="btn btn-outline btn-sm" onclick="logProjectTime('${p.id}')">Log Hours</button>
                <button class="btn btn-sm" onclick="invoiceProjectTimesheets('${p.id}')">1-Click Invoice</button>
              </div>
            </div>
          `;
        });
        h += '</div>';
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function logProjectTime(projectId) {
      const hrs = prompt('Enter hours to log for this project:', '4.0');
      if (!hrs) return;
      try {
        const res = await fetch('/api/v1/projects/timesheets', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            project_id: projectId,
            hours: hrs,
            description: 'Engineering and system integration work',
            is_billable: true
          })
        });
        if (res.ok) {
          showToast(`Logged ${hrs} hours successfully!`);
        }
      } catch (e) { showToast('Error: ' + e); }
    }

    async function invoiceProjectTimesheets(projectId) {
      try {
        const res = await fetch(`/api/v1/projects/${projectId}/invoice`, { method: 'POST' });
        const inv = await res.json();
        if (inv && inv.invoice_number) {
          showToast(`Generated Invoice ${inv.invoice_number} ($${inv.grand_total}) for unbilled hours!`);
        } else {
          showToast('No unbilled hours or error invoicing.');
        }
      } catch (e) { showToast('Error: ' + e); }
    }

    async function loadInvoices() {
      const c = document.getElementById('invoices-container');
      try {
        const res = await fetch('/api/v1/invoices');
        const invs = await res.json();
        let h = `
          <table>
            <thead>
              <tr>
                <th>Invoice #</th>
                <th>Customer</th>
                <th>Status</th>
                <th>Subtotal</th>
                <th>Tax</th>
                <th>Grand Total</th>
              </tr>
            </thead>
            <tbody>
        `;
        invs.forEach(inv => {
          h += `
            <tr>
              <td class="mono"><strong>${inv.invoice_number}</strong></td>
              <td>${inv.customer_id}</td>
              <td><span class="badge ${inv.status === 'Paid' ? 'green' : 'blue'}">${inv.status}</span></td>
              <td class="mono">$${inv.subtotal}</td>
              <td class="mono">$${inv.tax_total}</td>
              <td class="mono"><strong>$${inv.grand_total}</strong></td>
            </tr>
          `;
        });
        h += '</tbody></table>';
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function loadChartOfAccounts() {
      const c = document.getElementById('accounts-container');
      try {
        const res = await fetch('/api/v1/accounting/accounts');
        const accs = await res.json();
        let h = '<table><thead><tr><th>Code</th><th>Name</th><th>Type</th><th>Balance</th></tr></thead><tbody>';
        accs.forEach(a => {
          h += `
            <tr>
              <td class="mono">${a.account_code}</td>
              <td>${a.name}</td>
              <td><span class="badge blue">${a.account_type}</span></td>
              <td class="mono"><strong>$${a.balance}</strong></td>
            </tr>
          `;
        });
        h += '</tbody></table>';
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function loadTrialBalance() {
      const c = document.getElementById('trial-balance-container');
      try {
        const res = await fetch('/api/v1/accounting/trial-balance');
        const tb = await res.json();
        let h = `
          <div style="display: flex; gap: 1rem; margin-bottom: 1rem;">
            <div class="card" style="flex: 1; background: var(--bg-primary); text-align: center;">
              <div style="font-size: 0.8rem; color: var(--text-muted);">TOTAL DEBITS</div>
              <div class="mono" style="font-size: 1.25rem; font-weight: 700; color: var(--accent);">$${tb.total_debits}</div>
            </div>
            <div class="card" style="flex: 1; background: var(--bg-primary); text-align: center;">
              <div style="font-size: 0.8rem; color: var(--text-muted);">TOTAL CREDITS</div>
              <div class="mono" style="font-size: 1.25rem; font-weight: 700; color: var(--accent);">$${tb.total_credits}</div>
            </div>
          </div>
          <div style="text-align: center; margin-bottom: 1rem;">
            <span class="badge ${tb.is_balanced ? 'green' : 'amber'}" style="font-size: 0.9rem; padding: 0.4rem 1rem;">
              ${tb.is_balanced ? '✓ LEDGER IS MATHEMATICALLY BALANCED' : '⚠️ UNBALANCED'}
            </span>
          </div>
        `;
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function generateSvgLabel() {
      const sku = document.getElementById('label-sku-input').value.trim();
      const title = document.getElementById('label-title-input').value.trim();
      const preview = document.getElementById('svg-preview-box');
      try {
        const res = await fetch(`/api/v1/inventory/label?sku=${encodeURIComponent(sku)}&title=${encodeURIComponent(title)}`);
        const data = await res.json();
        preview.innerHTML = data.svg;
        showToast('Generated offline SVG thermal label');
      } catch (e) { preview.innerHTML = '<p style="color: red">' + e + '</p>'; }
    }

    function printFromLocator(sku, name) {
      document.getElementById('label-sku-input').value = sku;
      document.getElementById('label-title-input').value = name;
      switchTab('labels');
    }

    async function loadModules() {
      const c = document.getElementById('modules-list');
      try {
        const res = await fetch('/api/v1/modules');
        const mods = await res.json();
        let h = '<ul>';
        mods.forEach(m => {
          h += `
            <li style="margin-bottom: 0.75rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--border);">
              <div style="display: flex; justify-content: space-between;">
                <strong>${m.name}</strong>
                <span class="badge ${m.is_enabled ? 'green' : 'amber'}">${m.is_enabled ? 'ENABLED' : 'DISABLED'}</span>
              </div>
              <div style="font-size: 0.8rem; color: var(--text-muted); margin-top: 0.2rem;">${m.description}</div>
            </li>
          `;
        });
        h += '</ul>';
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    async function fetchSyncEvents() {
      const c = document.getElementById('sync-events-container');
      try {
        const res = await fetch('/api/v1/sync/pull?limit=5');
        const data = await res.json();
        let h = '<div class="mono" style="font-size: 0.75rem;">';
        if (data.events && data.events.length > 0) {
          data.events.forEach(ev => {
            h += `<div style="padding: 0.3rem 0; border-bottom: 1px solid var(--border);">[${ev.hlc_timestamp}] ${ev.event_type} (${ev.module_id})</div>`;
          });
        } else {
          h += '<div style="color: var(--text-muted);">No events in sync log yet.</div>';
        }
        h += '</div>';
        c.innerHTML = h;
      } catch (e) { c.innerHTML = '<p>' + e + '</p>'; }
    }

    // Initial load
    fetchHealth();
    runLocatorSearch();
  </script>
</body>
</html>
"#;
