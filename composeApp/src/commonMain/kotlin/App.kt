package org.erp.ledger

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge
import org.erp.ledger.features.accounting.AccountingScreen
import org.erp.ledger.features.inventory.InventoryLocatorScreen
import org.erp.ledger.features.inventory.LabelPrintingScreen
import org.erp.ledger.features.inventory.LocationTreeScreen
import org.erp.ledger.features.inventory.RapidScanMoveHud
import org.erp.ledger.features.invoicing.InvoicingScreen
import org.erp.ledger.features.manufacturing.ManufacturingScreen
import org.erp.ledger.features.projects.ProjectsScreen
import org.erp.ledger.features.purchasing.PurchasingScreen
import org.erp.ledger.features.settings.SettingsScreen
import org.erp.ledger.ui.theme.LedgerTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun App() {
    LaunchedEffect(Unit) {
        if (!ErpBridge.isInitialized.value) {
            ErpBridge.initialize()
        }
    }

    val currentScreen by ErpBridge.currentScreen.collectAsState()
    var selectedLabelSku by remember { mutableStateOf("M8-HEX-BOLT-316") }
    var selectedLabelName by remember { mutableStateOf("M8x40mm Hex Bolt Stainless 316") }

    val activeLocation by ErpBridge.activeContextLocation.collectAsState()
    val availableModules by ErpBridge.availableModules.collectAsState()

    val isInventoryEnabled = availableModules.firstOrNull { it.id == "mod-inventory" }?.isEnabled ?: true
    val isInvoicingEnabled = availableModules.firstOrNull { it.id == "mod-invoicing" }?.isEnabled ?: true
    val isAccountingEnabled = availableModules.firstOrNull { it.id == "mod-accounting" }?.isEnabled ?: true
    val isPurchasingEnabled = availableModules.firstOrNull { it.id == "mod-purchasing" }?.isEnabled ?: true
    val isManufacturingEnabled = availableModules.firstOrNull { it.id == "mod-manufacturing" }?.isEnabled ?: true
    val isProjectsEnabled = availableModules.firstOrNull { it.id == "mod-projects" }?.isEnabled ?: true

    LedgerTheme {
        BoxWithConstraints(modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background)) {
            val isWideScreen = maxWidth > 840.dp

            if (isWideScreen) {
                // Wide Desktop Layout with Side Navigation Rail
                Row(modifier = Modifier.fillMaxSize()) {
                    NavigationRail(
                        modifier = Modifier.fillMaxHeight(),
                        header = {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier.padding(vertical = 12.dp)
                            ) {
                                Text(
                                    text = "LEDGER",
                                    fontWeight = FontWeight.ExtraBold,
                                    fontSize = 15.sp,
                                    color = MaterialTheme.colorScheme.primary
                                )
                                Text(
                                    text = "LOCAL ERP",
                                    fontWeight = FontWeight.SemiBold,
                                    fontSize = 9.sp,
                                    color = MaterialTheme.colorScheme.outline
                                )
                                Spacer(modifier = Modifier.height(4.dp))
                                val syncInfo = remember { ErpBridge.getSyncStatus() }
                                Surface(
                                    shape = androidx.compose.foundation.shape.RoundedCornerShape(4.dp),
                                    color = if (syncInfo.convergenceState == "Converged") androidx.compose.ui.graphics.Color(0xFFD1FAE5) else androidx.compose.ui.graphics.Color(0xFFFEF3C7)
                                ) {
                                    Text(
                                        text = "${syncInfo.convergenceState} (${syncInfo.connectedPeersCount} peers)",
                                        fontSize = 8.sp,
                                        fontWeight = FontWeight.Bold,
                                        color = if (syncInfo.convergenceState == "Converged") androidx.compose.ui.graphics.Color(0xFF065F46) else androidx.compose.ui.graphics.Color(0xFF92400E),
                                        modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp)
                                    )
                                }
                            }
                        }
                    ) {
                        if (isInventoryEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "locator" || currentScreen == "labels",
                                onClick = { ErpBridge.navigateTo("locator") },
                                icon = { Icon(Icons.Default.Search, contentDescription = "Locator") },
                                label = { Text("Inventory", fontSize = 11.sp) }
                            )
                            NavigationRailItem(
                                selected = currentScreen == "scan_hud",
                                onClick = { ErpBridge.navigateTo("scan_hud") },
                                icon = {
                                    BadgedBox(badge = {
                                        if (activeLocation != null) {
                                            Badge { Text("1") }
                                        }
                                    }) {
                                        Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan HUD")
                                    }
                                },
                                label = { Text("Scan-Move", fontSize = 11.sp) }
                            )
                            NavigationRailItem(
                                selected = currentScreen == "tree",
                                onClick = { ErpBridge.navigateTo("tree") },
                                icon = { Icon(Icons.Default.AccountTree, contentDescription = "Storage Tree") },
                                label = { Text("Locations", fontSize = 11.sp) }
                            )
                        }

                        if (isPurchasingEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "purchasing",
                                onClick = { ErpBridge.navigateTo("purchasing") },
                                icon = { Icon(Icons.Default.ShoppingCart, contentDescription = "Purchasing") },
                                label = { Text("Purchasing", fontSize = 11.sp) }
                            )
                        }

                        if (isManufacturingEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "manufacturing",
                                onClick = { ErpBridge.navigateTo("manufacturing") },
                                icon = { Icon(Icons.Default.Build, contentDescription = "Manufacturing") },
                                label = { Text("Assembly", fontSize = 11.sp) }
                            )
                        }

                        if (isProjectsEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "projects",
                                onClick = { ErpBridge.navigateTo("projects") },
                                icon = { Icon(Icons.Default.Assignment, contentDescription = "Projects") },
                                label = { Text("Projects", fontSize = 11.sp) }
                            )
                        }

                        if (isInvoicingEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "invoicing",
                                onClick = { ErpBridge.navigateTo("invoicing") },
                                icon = { Icon(Icons.Default.ReceiptLong, contentDescription = "Invoices") },
                                label = { Text("Invoicing", fontSize = 11.sp) }
                            )
                        }

                        if (isAccountingEnabled) {
                            NavigationRailItem(
                                selected = currentScreen == "accounting",
                                onClick = { ErpBridge.navigateTo("accounting") },
                                icon = { Icon(Icons.Default.Assessment, contentDescription = "Accounting") },
                                label = { Text("Accounts", fontSize = 11.sp) }
                            )
                        }

                        Spacer(modifier = Modifier.weight(1f))

                        NavigationRailItem(
                            selected = currentScreen == "settings",
                            onClick = { ErpBridge.navigateTo("settings") },
                            icon = { Icon(Icons.Default.Settings, contentDescription = "Settings") },
                            label = { Text("Modules", fontSize = 11.sp) }
                        )
                    }

                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .fillMaxHeight()
                    ) {
                        RenderCurrentScreen(
                            currentScreen = currentScreen,
                            selectedLabelSku = selectedLabelSku,
                            selectedLabelName = selectedLabelName,
                            onPrintLabel = { sku, name ->
                                selectedLabelSku = sku
                                selectedLabelName = name
                                ErpBridge.navigateTo("labels")
                            }
                        )
                    }
                }
            } else {
                // Mobile Layout with Bottom Navigation Bar
                Scaffold(
                    bottomBar = {
                        NavigationBar {
                            if (isInventoryEnabled) {
                                NavigationBarItem(
                                    selected = currentScreen == "locator",
                                    onClick = { ErpBridge.navigateTo("locator") },
                                    icon = { Icon(Icons.Default.Search, contentDescription = "Locator") },
                                    label = { Text("Find", fontSize = 10.sp) }
                                )
                                NavigationBarItem(
                                    selected = currentScreen == "scan_hud",
                                    onClick = { ErpBridge.navigateTo("scan_hud") },
                                    icon = {
                                        BadgedBox(badge = {
                                            if (activeLocation != null) {
                                                Badge { Text("1") }
                                            }
                                        }) {
                                            Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan HUD")
                                        }
                                    },
                                    label = { Text("Scan", fontSize = 10.sp) }
                                )
                            }

                            if (isPurchasingEnabled) {
                                NavigationBarItem(
                                    selected = currentScreen == "purchasing",
                                    onClick = { ErpBridge.navigateTo("purchasing") },
                                    icon = { Icon(Icons.Default.ShoppingCart, contentDescription = "Buying") },
                                    label = { Text("Buy", fontSize = 10.sp) }
                                )
                            }

                            if (isManufacturingEnabled) {
                                NavigationBarItem(
                                    selected = currentScreen == "manufacturing",
                                    onClick = { ErpBridge.navigateTo("manufacturing") },
                                    icon = { Icon(Icons.Default.Build, contentDescription = "Assembly") },
                                    label = { Text("BOM", fontSize = 10.sp) }
                                )
                            }

                            if (isInvoicingEnabled) {
                                NavigationBarItem(
                                    selected = currentScreen == "invoicing",
                                    onClick = { ErpBridge.navigateTo("invoicing") },
                                    icon = { Icon(Icons.Default.ReceiptLong, contentDescription = "Invoices") },
                                    label = { Text("Invoice", fontSize = 10.sp) }
                                )
                            }

                            if (isAccountingEnabled) {
                                NavigationBarItem(
                                    selected = currentScreen == "accounting",
                                    onClick = { ErpBridge.navigateTo("accounting") },
                                    icon = { Icon(Icons.Default.Assessment, contentDescription = "Accounts") },
                                    label = { Text("GL", fontSize = 10.sp) }
                                )
                            }

                            NavigationBarItem(
                                selected = currentScreen == "settings",
                                onClick = { ErpBridge.navigateTo("settings") },
                                icon = { Icon(Icons.Default.Settings, contentDescription = "Settings") },
                                label = { Text("More", fontSize = 10.sp) }
                            )
                        }
                    }
                ) { innerPadding ->
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(innerPadding)
                    ) {
                        RenderCurrentScreen(
                            currentScreen = currentScreen,
                            selectedLabelSku = selectedLabelSku,
                            selectedLabelName = selectedLabelName,
                            onPrintLabel = { sku, name ->
                                selectedLabelSku = sku
                                selectedLabelName = name
                                ErpBridge.navigateTo("labels")
                            }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun RenderCurrentScreen(
    currentScreen: String,
    selectedLabelSku: String,
    selectedLabelName: String,
    onPrintLabel: (sku: String, name: String) -> Unit
) {
    when (currentScreen) {
        "locator" -> InventoryLocatorScreen(
            onNavigateToScanHud = { ErpBridge.navigateTo("scan_hud") },
            onPrintLabel = onPrintLabel
        )
        "scan_hud" -> RapidScanMoveHud(
            onNavigateBack = { ErpBridge.navigateTo("locator") }
        )
        "tree" -> LocationTreeScreen(
            onSelectLocation = { ErpBridge.navigateTo("scan_hud") }
        )
        "labels" -> LabelPrintingScreen(
            initialSku = selectedLabelSku,
            initialName = selectedLabelName,
            onNavigateBack = { ErpBridge.navigateTo("locator") }
        )
        "invoicing" -> InvoicingScreen()
        "accounting" -> AccountingScreen()
        "purchasing" -> PurchasingScreen()
        "manufacturing" -> ManufacturingScreen()
        "projects" -> ProjectsScreen()
        "settings" -> SettingsScreen()
        else -> InventoryLocatorScreen(
            onNavigateToScanHud = { ErpBridge.navigateTo("scan_hud") },
            onPrintLabel = onPrintLabel
        )
    }
}
