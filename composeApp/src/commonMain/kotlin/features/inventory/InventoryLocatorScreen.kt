package org.erp.ledger.features.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
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
import org.erp.ledger.core.DynamicPluginUiRenderer
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.ItemLocationCardDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InventoryLocatorScreen(
    onNavigateToScanHud: () -> Unit = {},
    onPrintLabel: (sku: String, name: String) -> Unit = { _, _ -> }
) {
    var searchQuery by remember { mutableStateOf("") }
    val results = remember(searchQuery) {
        ErpBridge.searchItemLocations(searchQuery)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("Item Locator", fontWeight = FontWeight.Bold)
                        Text(
                            "Instant 'Where Is It?' search with spatial coordinates",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                },
                actions = {
                    IconButton(onClick = onNavigateToScanHud) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan QR")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Search Input Field
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                label = { Text("Search by SKU, item name, barcode, or lot #") },
                placeholder = { Text("e.g. M8-HEX-BOLT, BEARING, LOT-2026") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                trailingIcon = {
                    if (searchQuery.isNotEmpty()) {
                        IconButton(onClick = { searchQuery = "" }) {
                            Icon(Icons.Default.Clear, contentDescription = "Clear")
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                shape = RoundedCornerShape(12.dp)
            )

            // Quick suggestion chips
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                listOf("M8-HEX-BOLT", "BEARING-608", "Blue Bin #4").forEach { chip ->
                    SuggestionChip(
                        onClick = { searchQuery = chip },
                        label = { Text(chip, fontSize = 12.sp) }
                    )
                }
            }

            // Interactive 2D Spatial Floor Plan Radar
            val activeLoc by ErpBridge.activeContextLocation.collectAsState()
            val allLocations = remember { ErpBridge.getStorageLocations() }
            FloorPlanCanvas(
                locations = allLocations,
                activeLocation = activeLoc,
                onLocationTapped = { loc ->
                    ErpBridge.setActiveContextLocation(loc)
                }
            )

            // Results List
            if (results.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            Icons.Default.SearchOff,
                            contentDescription = null,
                            modifier = Modifier.size(48.dp),
                            tint = MaterialTheme.colorScheme.outline
                        )
                        Text(
                            "No inventory locations matched '$searchQuery'",
                            color = MaterialTheme.colorScheme.outline
                        )
                    }
                }
            } else {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                    contentPadding = PaddingValues(bottom = 24.dp)
                ) {
                    items(results) { match ->
                        ItemLocationCard(
                            match = match,
                            onPrintLabel = { onPrintLabel(match.sku, match.itemName) }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun ItemLocationCard(
    match: ItemLocationCardDto,
    onPrintLabel: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            // Header: Item Name, SKU, and Stock Badge
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = match.itemName,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = "SKU: ${match.sku}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.SemiBold
                    )
                }

                // Quantity badge
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(Color(0xFF10B981).copy(alpha = 0.15f))
                        .border(1.dp, Color(0xFF10B981).copy(alpha = 0.3f), RoundedCornerShape(8.dp))
                        .padding(horizontal = 10.dp, vertical = 6.dp)
                ) {
                    Text(
                        text = "${match.quantity} ${match.unit}",
                        fontWeight = FontWeight.Bold,
                        fontSize = 14.sp,
                        color = Color(0xFF047857)
                    )
                }
            }

            // Human-Readable Breadcrumb Path
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(8.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
                    .padding(horizontal = 12.dp, vertical = 8.dp)
            ) {
                Text(
                    text = "STORAGE PATH",
                    fontSize = 10.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.outline
                )
                Spacer(modifier = Modifier.height(2.dp))
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    Icon(
                        Icons.Default.Place,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                    Text(
                        text = match.breadcrumbPath,
                        style = MaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Medium,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }
            }

            // Visual Shelf Grid & Floor Map Preview
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Shelf-Grid Visual Indicator
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "SHELF POSITION",
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    VisualShelfGrid(activeBinIndex = 5)
                }

                // 2D Floor-Plan Coordinates Preview
                if (match.mapX != null && match.mapY != null) {
                    Column(
                        modifier = Modifier
                            .width(100.dp)
                            .clip(RoundedCornerShape(8.dp))
                            .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
                            .padding(8.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Text(
                            text = "MAP 2D",
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.outline
                        )
                        val mx = match.mapX?.toInt() ?: 0
                        val my = match.mapY?.toInt() ?: 0
                        Text(
                            text = "($mx, $my)",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                }
            }

            // Lot/Serial metadata if present
            if (match.lotNumber != null || match.serialNumber != null) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    match.lotNumber?.let {
                        AssistChip(
                            onClick = {},
                            label = { Text("Lot: $it", fontSize = 11.sp) },
                            leadingIcon = { Icon(Icons.Default.Tag, contentDescription = null, modifier = Modifier.size(14.dp)) }
                        )
                    }
                    match.serialNumber?.let {
                        AssistChip(
                            onClick = {},
                            label = { Text("SN: $it", fontSize = 11.sp) },
                            leadingIcon = { Icon(Icons.Default.ConfirmationNumber, contentDescription = null, modifier = Modifier.size(14.dp)) }
                        )
                    }
                }
            }

            // Dynamic WebAssembly Plugin UI Slot ("inventory_detail")
            val pluginNodes = remember { ErpBridge.getPluginUiNodes("inventory_detail") }
            if (pluginNodes.isNotEmpty()) {
                DynamicPluginUiRenderer(nodes = pluginNodes)
            }

            // Action row
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedButton(
                    onClick = onPrintLabel,
                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)
                ) {
                    Icon(Icons.Default.Print, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("Print QR", fontSize = 12.sp)
                }
            }
        }
    }
}

/**
 * Visual Shelf Grid: Represents rack shelving levels and highlights the target bin position.
 */
@Composable
fun VisualShelfGrid(activeBinIndex: Int = 5) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(6.dp))
            .background(Color(0xFF1E293B))
            .padding(6.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        for (row in 0 until 3) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                for (col in 0 until 4) {
                    val index = row * 4 + col
                    val isActive = index == activeBinIndex
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height(14.dp)
                            .clip(RoundedCornerShape(2.dp))
                            .background(
                                if (isActive) Color(0xFF10B981) else Color(0xFF334155)
                            )
                    )
                }
            }
        }
    }
}
