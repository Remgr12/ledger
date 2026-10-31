package org.erp.ledger.features.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import uniffi.erp_ffi.LocationDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LocationTreeScreen(
    onSelectLocation: (LocationDto) -> Unit = {}
) {
    var locations by remember { mutableStateOf(ErpBridge.getStorageLocations()) }
    val activeLocation by ErpBridge.activeContextLocation.collectAsState()

    fun reload() {
        locations = ErpBridge.getStorageLocations()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Hierarchical Storage Tree & Floor Plan", fontWeight = FontWeight.Bold) },
                actions = {
                    IconButton(onClick = { reload() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            // Interactive Warehouse Floor Plan Radar Canvas with Drag & Drop
            item {
                FloorPlanCanvas(
                    locations = locations,
                    activeLocation = activeLocation,
                    onLocationTapped = { loc ->
                        ErpBridge.setActiveContextLocation(loc)
                        onSelectLocation(loc)
                    },
                    onLocationMoved = { loc, newX, newY ->
                        ErpBridge.updateLocationCoordinates(loc.id, newX, newY)
                        reload()
                    }
                )
            }

            item {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        "STORAGE HIERARCHY TREE",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )
                    Text(
                        "${locations.size} locations configured",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            items(locations) { loc ->
                LocationTreeItem(
                    location = loc,
                    onClick = {
                        ErpBridge.setActiveContextLocation(loc)
                        onSelectLocation(loc)
                    }
                )
            }
        }
    }
}

@Composable
fun LocationTreeItem(
    location: LocationDto,
    onClick: () -> Unit
) {
    val indent = when (location.locationType) {
        "Warehouse" -> 0.dp
        "Room" -> 16.dp
        "Zone", "Aisle" -> 32.dp
        "Rack" -> 48.dp
        "Shelf" -> 64.dp
        "Bin", "Container" -> 80.dp
        else -> 0.dp
    }

    val icon = when (location.locationType) {
        "Warehouse" -> Icons.Default.Warehouse
        "Room" -> Icons.Default.MeetingRoom
        "Rack" -> Icons.Default.ViewColumn
        "Shelf" -> Icons.Default.TableRows
        "Bin" -> Icons.Default.Inbox
        else -> Icons.Default.Folder
    }

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = indent)
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(10.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Icon(
                icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(24.dp)
            )

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = location.name,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Code: ${location.code} • ${location.locationType} • 2D: (${location.mapX?.toInt() ?: 0}, ${location.mapY?.toInt() ?: 0})",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.outline
                )
            }

            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(6.dp))
                    .background(MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f))
                    .padding(horizontal = 8.dp, vertical = 4.dp)
            ) {
                Text(
                    text = "Target",
                    fontSize = 11.sp,
                    color = MaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.SemiBold
                )
            }
        }
    }
}
