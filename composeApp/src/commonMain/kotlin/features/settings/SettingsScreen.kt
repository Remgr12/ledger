package org.erp.ledger.features.settings

import androidx.compose.foundation.background
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge

data class WasmPluginUi(
    val id: String,
    val name: String,
    val version: String,
    val description: String,
    val permissions: List<String>,
    var isInstalled: Boolean
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    val modules by ErpBridge.availableModules.collectAsState()

    val wasmPlugins = remember {
        mutableStateListOf(
            WasmPluginUi(
                id = "supplier-gs1-decoder",
                name = "Supplier GS1-128 QR Decoder",
                version = "1.2.0",
                description = "Intercepts proprietary automotive & aerospace barcode matrices into standard ERP SKUs and lot serials.",
                permissions = listOf("read:inventory", "hook:on_qr_scanned"),
                isInstalled = true
            ),
            WasmPluginUi(
                id = "bin-capacity-indicator",
                name = "Visual Bin Volumetric Predictor",
                version = "0.9.4",
                description = "Declarative UI node card calculating volumetric bin fill percentages in Compose.",
                permissions = listOf("read:inventory", "ui:custom_field"),
                isInstalled = false
            )
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Modules & Wasm Marketplace", fontWeight = FontWeight.Bold) }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            item {
                val currentUrl by ErpBridge.serverUrl.collectAsState()
                val serverStatus by ErpBridge.serverStatus.collectAsState()
                var inputUrl by remember(currentUrl) { mutableStateOf(currentUrl ?: "http://localhost:8080") }

                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Text(
                                    text = "⚡ Headless Server & Multi-Device Access",
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 16.sp
                                )
                                Text(
                                    text = "Connect apps to a headless Linux/Cloud server or host the Web Console",
                                    fontSize = 12.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Surface(
                                color = Color(0xFF2E7D32).copy(alpha = 0.15f),
                                shape = RoundedCornerShape(12.dp)
                            ) {
                                Text(
                                    text = "ACTIVE",
                                    color = Color(0xFF2E7D32),
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 11.sp,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(12.dp))

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

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                Icons.Default.CheckCircle,
                                contentDescription = null,
                                tint = Color(0xFF2E7D32),
                                modifier = Modifier.size(16.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = serverStatus,
                                fontSize = 12.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }

                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "💡 Tip: Run 'ledger-server --bind 0.0.0.0:8080' or 'ledger-cli server' on any server or VPS to access via web browser or connect your team's desktop/mobile apps.",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.outline
                        )
                    }
                }
            }

            item {
                Text(
                    text = "MODULAR ARCHITECTURE (\"ENABLE WHAT YOU NEED\")",
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

            item {
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "SANDBOXED WEBASSEMBLY (WASM) PLUGINS",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.outline
                )
            }

            items(wasmPlugins) { plugin ->
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
                            Text("${plugin.name} v${plugin.version}", fontWeight = FontWeight.Bold, fontSize = 15.sp)
                            Button(
                                onClick = { plugin.isInstalled = !plugin.isInstalled },
                                colors = if (plugin.isInstalled) {
                                    ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondaryContainer)
                                } else {
                                    ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
                                }
                            ) {
                                Text(
                                    if (plugin.isInstalled) "Uninstall" else "Install .wasm",
                                    fontSize = 12.sp,
                                    color = if (plugin.isInstalled) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onPrimary
                                )
                            }
                        }

                        Text(plugin.description, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                        // Capabilities / Permissions
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Permissions:", fontSize = 11.sp, fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.outline)
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
    }
}
