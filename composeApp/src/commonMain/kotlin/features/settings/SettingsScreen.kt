package org.erp.ledger.features.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.DynamicPluginUiRenderer
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.PluginManifestDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    val modules by ErpBridge.availableModules.collectAsState()

    // Database Backup & Export/Import States (LEDGER-9)
    var backupDestPath by remember { mutableStateOf("ledger_backup.db") }
    var backupResultMsg by remember { mutableStateOf<String?>(null) }
    var isBackupError by remember { mutableStateOf(false) }

    var exportEntity by remember { mutableStateOf("items") }
    var exportFormat by remember { mutableStateOf("csv") }
    var exportedData by remember { mutableStateOf<String?>(null) }
    var exportResultMsg by remember { mutableStateOf<String?>(null) }

    var importEntity by remember { mutableStateOf("items") }
    var importCsvContent by remember {
        mutableStateOf("sku,name,unit,description,min_stock\nRES-10K,10k Resistor 1/4W,pcs,Precision metal film,100\nCAP-100UF,100uF Capacitor 25V,pcs,Electrolytic low ESR,50")
    }
    var importResultMsg by remember { mutableStateOf<String?>(null) }
    var isImportError by remember { mutableStateOf(false) }

    // WASM Plugin States (LEDGER-10)
    var installedPlugins by remember { mutableStateOf(ErpBridge.listInstalledPlugins()) }
    var wasmFilePath by remember { mutableStateOf("plugins/sample_supplier_plugin.wasm") }
    var pluginId by remember { mutableStateOf("sample-supplier-plugin") }
    var pluginName by remember { mutableStateOf("Supplier GS1 & Quality Inspector") }
    var pluginVersion by remember { mutableStateOf("1.0.0") }
    var pluginDescription by remember { mutableStateOf("Decodes proprietary supplier QR matrices and displays QA badges.") }
    var pluginCapabilities by remember { mutableStateOf("read:inventory, ui:custom_field, hook:on_qr_scanned") }
    var pluginStatusMsg by remember { mutableStateOf<String?>(null) }
    var isPluginError by remember { mutableStateOf(false) }
    var pluginUiNodes by remember { mutableStateOf(ErpBridge.getPluginUiNodes("inventory_detail")) }

    fun refreshPlugins() {
        installedPlugins = ErpBridge.listInstalledPlugins()
        pluginUiNodes = ErpBridge.getPluginUiNodes("inventory_detail")
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings, Backup & Extensions", fontWeight = FontWeight.Bold) }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(18.dp)
        ) {
            // ==========================================
            // 1. Headless Server & Multi-Device Access
            // ==========================================
            item {
                val currentUrl by ErpBridge.serverUrl.collectAsState()
                val serverStatus by ErpBridge.serverStatus.collectAsState()
                var inputUrl by remember(currentUrl) { mutableStateOf(currentUrl ?: "http://localhost:8080") }

                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Text(
                                    text = "⚡ Headless Server & Sync",
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 15.sp
                                )
                                Text(
                                    text = "Connect desktop and mobile apps to a headless team server",
                                    fontSize = 12.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Surface(
                                color = Color(0xFF2E7D32).copy(alpha = 0.15f),
                                shape = RoundedCornerShape(12.dp)
                            ) {
                                Text(
                                    text = "LOCAL-FIRST",
                                    color = Color(0xFF2E7D32),
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 11.sp,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))

                        OutlinedTextField(
                            value = inputUrl,
                            onValueChange = { inputUrl = it },
                            label = { Text("Server Base URL (IP / Hostname)") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true,
                            leadingIcon = { Icon(Icons.Default.Cloud, contentDescription = null) },
                            trailingIcon = {
                                TextButton(onClick = { ErpBridge.setServerUrl(inputUrl) }) {
                                    Text("Apply")
                                }
                            }
                        )

                        Spacer(modifier = Modifier.height(6.dp))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                Icons.Default.CheckCircle,
                                contentDescription = null,
                                tint = Color(0xFF2E7D32),
                                modifier = Modifier.size(14.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = serverStatus,
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }

            // ==========================================
            // 2. Local SQLite Backup & Export / Import (LEDGER-9)
            // ==========================================
            item {
                Text(
                    text = "LOCAL SQLITE BACKUP & DATA PORTABILITY (LEDGER-9)",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.outline
                )
            }

            // SQLite Safe WAL Backup Card
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Icon(Icons.Default.Storage, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                Column {
                                    Text("Full SQLite Database Backup", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                                    Text("Safe WAL checkpoint flush & online backup API", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                            }
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            OutlinedTextField(
                                value = backupDestPath,
                                onValueChange = { backupDestPath = it },
                                label = { Text("Backup Destination Path (.db)") },
                                modifier = Modifier.weight(1f),
                                singleLine = true
                            )

                            Button(
                                onClick = {
                                    val success = ErpBridge.backupDatabase(backupDestPath)
                                    if (success) {
                                        backupResultMsg = "✓ Safe WAL checkpoint flushed. Full backup written to $backupDestPath"
                                        isBackupError = false
                                    } else {
                                        backupResultMsg = "Failed to create database backup at $backupDestPath"
                                        isBackupError = true
                                    }
                                }
                            ) {
                                Icon(Icons.Default.Save, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("Backup Now")
                            }
                        }

                        backupResultMsg?.let { msg ->
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(if (isBackupError) Color(0xFFFEE2E2) else Color(0xFFD1FAE5))
                                    .padding(8.dp)
                            ) {
                                Text(
                                    msg,
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = if (isBackupError) Color(0xFFB91C1C) else Color(0xFF047857)
                                )
                            }
                        }
                    }
                }
            }

            // Data Export Card (CSV / JSON)
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Icon(Icons.Default.Upload, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                            Text("Export Entity Data (CSV / JSON)", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                        }

                        Text("Select table entity and desired serialization format:", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            listOf("items" to "Items / Inventory", "contacts" to "Contacts & Customers", "journal" to "GL Journal").forEach { (key, label) ->
                                FilterChip(
                                    selected = exportEntity == key,
                                    onClick = { exportEntity = key },
                                    label = { Text(label, fontSize = 11.sp) }
                                )
                            }
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            listOf("csv" to "CSV Format", "json" to "JSON Format").forEach { (fmt, label) ->
                                FilterChip(
                                    selected = exportFormat == fmt,
                                    onClick = { exportFormat = fmt },
                                    label = { Text(label, fontSize = 11.sp) }
                                )
                            }

                            Spacer(modifier = Modifier.weight(1f))

                            Button(
                                onClick = {
                                    val data = if (exportFormat == "csv") {
                                        ErpBridge.exportDataCsv(exportEntity)
                                    } else {
                                        ErpBridge.exportDataJson(exportEntity)
                                    }
                                    exportedData = data
                                    val lineCount = data.lines().size
                                    exportResultMsg = "✓ Exported $lineCount lines (${data.length} characters) for '$exportEntity'"
                                }
                            ) {
                                Text("Generate Export")
                            }
                        }

                        exportResultMsg?.let { msg ->
                            Text(msg, fontSize = 12.sp, fontWeight = FontWeight.SemiBold, color = Color(0xFF047857))
                        }

                        exportedData?.let { content ->
                            Column(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(8.dp))
                                    .background(Color(0xFF0F172A))
                                    .padding(10.dp)
                            ) {
                                Text(
                                    content.take(800) + if (content.length > 800) "\n... [truncated]" else "",
                                    fontFamily = FontFamily.Monospace,
                                    fontSize = 11.sp,
                                    color = Color(0xFFE2E8F0)
                                )
                            }
                        }
                    }
                }
            }

            // Data Import Card (CSV)
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Icon(Icons.Default.Download, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                            Text("Import Initial Data from CSV Spreadsheets", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            listOf("items" to "Import Items", "contacts" to "Import Contacts").forEach { (key, label) ->
                                FilterChip(
                                    selected = importEntity == key,
                                    onClick = {
                                        importEntity = key
                                        if (key == "items") {
                                            importCsvContent = "sku,name,unit,description,min_stock\nRES-10K,10k Resistor 1/4W,pcs,Precision metal film,100\nCAP-100UF,100uF Capacitor 25V,pcs,Electrolytic low ESR,50"
                                        } else {
                                            importCsvContent = "name,email,phone,tax_id\nAcme Supplies,orders@acme.com,+1-555-0100,US-EIN-991\nGlobal Fasteners,sales@globalfast.com,+1-555-0200,US-EIN-882"
                                        }
                                    },
                                    label = { Text(label, fontSize = 11.sp) }
                                )
                            }
                        }

                        OutlinedTextField(
                            value = importCsvContent,
                            onValueChange = { importCsvContent = it },
                            label = { Text("CSV Records (First line must be header)") },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(130.dp),
                            textStyle = androidx.compose.ui.text.TextStyle(fontFamily = FontFamily.Monospace, fontSize = 11.sp)
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                "Expected format: ${if (importEntity == "items") "sku,name,unit,description,min_stock" else "name,email,phone,tax_id"}",
                                fontSize = 10.sp,
                                color = MaterialTheme.colorScheme.outline
                            )

                            Button(
                                onClick = {
                                    val count = ErpBridge.importDataCsv(importEntity, importCsvContent)
                                    if (count > 0) {
                                        importResultMsg = "✓ Successfully imported $count records into local SQLite database"
                                        isImportError = false
                                    } else {
                                        importResultMsg = "No records were imported. Verify CSV format and required fields."
                                        isImportError = true
                                    }
                                }
                            ) {
                                Icon(Icons.Default.UploadFile, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("Import Records")
                            }
                        }

                        importResultMsg?.let { msg ->
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(if (isImportError) Color(0xFFFEE2E2) else Color(0xFFD1FAE5))
                                    .padding(8.dp)
                            ) {
                                Text(
                                    msg,
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = if (isImportError) Color(0xFFB91C1C) else Color(0xFF047857)
                                )
                            }
                        }
                    }
                }
            }

            // ==========================================
            // 3. Sandboxed WebAssembly (WASM) Plugins (LEDGER-10)
            // ==========================================
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "SANDBOXED WEBASSEMBLY (WASM) PLUGINS (LEDGER-10)",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )
                    IconButton(onClick = { refreshPlugins() }, modifier = Modifier.size(24.dp)) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh", modifier = Modifier.size(16.dp))
                    }
                }
            }

            // Installed Plugins List
            if (installedPlugins.isEmpty()) {
                item {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f))
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(20.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(6.dp)
                        ) {
                            Icon(Icons.Default.Extension, contentDescription = null, tint = MaterialTheme.colorScheme.outline, modifier = Modifier.size(32.dp))
                            Text("No External WASM Plugins Installed", fontWeight = FontWeight.Bold, fontSize = 14.sp)
                            Text("Use the installer below to register a compiled .wasm module from your filesystem.", fontSize = 12.sp, color = MaterialTheme.colorScheme.outline)
                        }
                    }
                }
            } else {
                items(installedPlugins) { plugin ->
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Icon(Icons.Default.Extension, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                    Column {
                                        Text("${plugin.name} v${plugin.version}", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                                        Text("ID: ${plugin.id}", fontSize = 11.sp, color = MaterialTheme.colorScheme.outline)
                                    }
                                }

                                Button(
                                    onClick = {
                                        ErpBridge.uninstallPlugin(plugin.id)
                                        refreshPlugins()
                                    },
                                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.errorContainer)
                                ) {
                                    Text("Uninstall", fontSize = 12.sp, color = MaterialTheme.colorScheme.onErrorContainer)
                                }
                            }

                            Text(plugin.description, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                            // Capabilities
                            Row(
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text("Capabilities:", fontSize = 11.sp, fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.outline)
                                plugin.permissions.forEach { perm ->
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(4.dp))
                                            .background(Color(0xFFEEF2FF))
                                            .padding(horizontal = 6.dp, vertical = 2.dp)
                                    ) {
                                        Text(perm, fontSize = 10.sp, fontWeight = FontWeight.SemiBold, color = Color(0xFF4338CA))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // In-app WASM Plugin Installer Card
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Icon(Icons.Default.AddCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                            Text("Install Sandboxed WASM Plugin from Local Filesystem", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                        }

                        OutlinedTextField(
                            value = wasmFilePath,
                            onValueChange = { wasmFilePath = it },
                            label = { Text("Path to .wasm Binary") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            OutlinedTextField(
                                value = pluginId,
                                onValueChange = { pluginId = it },
                                label = { Text("Plugin ID") },
                                modifier = Modifier.weight(1f),
                                singleLine = true
                            )
                            OutlinedTextField(
                                value = pluginVersion,
                                onValueChange = { pluginVersion = it },
                                label = { Text("Version") },
                                modifier = Modifier.width(90.dp),
                                singleLine = true
                            )
                        }

                        OutlinedTextField(
                            value = pluginName,
                            onValueChange = { pluginName = it },
                            label = { Text("Display Name") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true
                        )

                        OutlinedTextField(
                            value = pluginDescription,
                            onValueChange = { pluginDescription = it },
                            label = { Text("Description") },
                            modifier = Modifier.fillMaxWidth()
                        )

                        OutlinedTextField(
                            value = pluginCapabilities,
                            onValueChange = { pluginCapabilities = it },
                            label = { Text("Permissions / Capabilities (comma-separated)") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true
                        )

                        Button(
                            onClick = {
                                val permsJson = pluginCapabilities.split(",")
                                    .map { "\"${it.trim()}\"" }
                                    .joinToString(", ")
                                val manifestJson = """
                                {
                                    "id": "$pluginId",
                                    "name": "$pluginName",
                                    "version": "$pluginVersion",
                                    "description": "$pluginDescription",
                                    "permissions": [$permsJson]
                                }
                                """.trimIndent()

                                val success = ErpBridge.installWasmPluginFile(wasmFilePath, manifestJson)
                                if (success) {
                                    pluginStatusMsg = "✓ Successfully installed and initialized plugin '$pluginName'!"
                                    isPluginError = false
                                    refreshPlugins()
                                } else {
                                    pluginStatusMsg = "Failed to load WASM binary or initialize Extism runtime."
                                    isPluginError = true
                                }
                            },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Icon(Icons.Default.Download, contentDescription = null)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Install & Activate .wasm Module")
                        }

                        pluginStatusMsg?.let { msg ->
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(if (isPluginError) Color(0xFFFEE2E2) else Color(0xFFD1FAE5))
                                    .padding(8.dp)
                            ) {
                                Text(
                                    msg,
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = if (isPluginError) Color(0xFFB91C1C) else Color(0xFF047857)
                                )
                            }
                        }
                    }
                }
            }

            // Live Plugin UI Slot Preview ("inventory_detail")
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                "DYNAMIC PLUGIN UI SLOT PREVIEW ('inventory_detail')",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.outline
                            )
                            TextButton(onClick = { refreshPlugins() }) {
                                Text("Re-render Slot", fontSize = 11.sp)
                            }
                        }

                        if (pluginUiNodes.isEmpty()) {
                            Text(
                                "No active plugins have contributed UI nodes to slot 'inventory_detail'. Install the sample plugin above to render declarative dynamic components.",
                                fontSize = 12.sp,
                                color = MaterialTheme.colorScheme.outline
                            )
                        } else {
                            DynamicPluginUiRenderer(nodes = pluginUiNodes)
                        }
                    }
                }
            }

            // ==========================================
            // 4. Built-in Core Modules
            // ==========================================
            item {
                Text(
                    text = "CORE ARCHITECTURAL MODULES (\"ENABLE WHAT YOU NEED\")",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.outline
                )
            }

            items(modules) { mod ->
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(mod.name, fontWeight = FontWeight.Bold, fontSize = 15.sp)
                            Text(mod.description, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            if (mod.dependencies.isNotEmpty()) {
                                Spacer(modifier = Modifier.height(4.dp))
                                Text("Requires: ${mod.dependencies.joinToString()}", fontSize = 11.sp, color = MaterialTheme.colorScheme.primary)
                            }
                        }

                        Switch(
                            checked = mod.isEnabled,
                            onCheckedChange = { isChecked ->
                                ErpBridge.toggleModule(mod.id, isChecked)
                            }
                        )
                    }
                }
            }
        }
    }
}
