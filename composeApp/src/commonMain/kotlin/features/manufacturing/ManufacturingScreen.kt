package org.erp.ledger.features.manufacturing

import androidx.compose.foundation.background
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.BillOfMaterialsDto
import uniffi.erp_ffi.BomComponentDto
import uniffi.erp_ffi.WorkOrderDto

@Composable
fun ManufacturingScreen() {
    var selectedTab by remember { mutableStateOf(0) }
    var boms by remember { mutableStateOf(ErpBridge.listBoms()) }
    var workOrders by remember { mutableStateOf(ErpBridge.listWorkOrders()) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    var selectedGenealogyWo by remember { mutableStateOf<WorkOrderDto?>(null) }
    var genealogyData by remember { mutableStateOf<uniffi.erp_ffi.ComponentGenealogyDto?>(null) }
    var showKanbanView by remember { mutableStateOf(true) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .padding(16.dp)
    ) {
        // Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(
                    text = "Manufacturing & Assembly",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "BOMs, interactive Kanban workflow, and component lot genealogy tracking",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                if (selectedTab == 0) {
                    FilterChip(
                        selected = showKanbanView,
                        onClick = { showKanbanView = !showKanbanView },
                        label = { Text(if (showKanbanView) "Kanban Board" else "List View", fontSize = 12.sp) },
                        leadingIcon = { Icon(if (showKanbanView) Icons.Default.ViewKanban else Icons.Default.List, contentDescription = null, modifier = Modifier.size(16.dp)) }
                    )
                }

                IconButton(onClick = {
                    boms = ErpBridge.listBoms()
                    workOrders = ErpBridge.listWorkOrders()
                    statusMessage = null
                }) {
                    Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Status banner if any
        statusMessage?.let { msg ->
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer,
                shape = RoundedCornerShape(8.dp),
                modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp)
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        Icons.Default.CheckCircle,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = msg,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                }
            }
        }

        // Tab Row
        TabRow(
            selectedTabIndex = selectedTab,
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
            contentColor = MaterialTheme.colorScheme.primary,
            modifier = Modifier.clip(RoundedCornerShape(12.dp))
        ) {
            Tab(
                selected = selectedTab == 0,
                onClick = { selectedTab = 0 },
                text = { Text("Work Orders (${workOrders.size})") },
                icon = { Icon(Icons.Default.PrecisionManufacturing, contentDescription = null) }
            )
            Tab(
                selected = selectedTab == 1,
                onClick = { selectedTab = 1 },
                text = { Text("Bill of Materials (${boms.size})") },
                icon = { Icon(Icons.Default.Schema, contentDescription = null) }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        if (selectedTab == 0) {
            if (showKanbanView) {
                // Interactive Kanban Swimlanes (LEDGER-14)
                val stages = listOf("Planned", "In Progress", "Quality Check", "Completed")
                Row(
                    modifier = Modifier.fillMaxSize(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    stages.forEach { stage ->
                        val stageOrders = workOrders.filter {
                            if (stage == "Planned") it.status.equals("Planned", ignoreCase = true) || it.status.equals("Draft", ignoreCase = true)
                            else it.status.equals(stage, ignoreCase = true)
                        }

                        Card(
                            modifier = Modifier.weight(1f).fillMaxHeight(),
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Column(modifier = Modifier.padding(10.dp)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text(stage, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                                    Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surface) {
                                        Text("${stageOrders.size}", modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                                    }
                                }

                                Spacer(modifier = Modifier.height(8.dp))

                                LazyColumn(
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                    modifier = Modifier.fillMaxSize()
                                ) {
                                    items(stageOrders) { wo ->
                                        WorkOrderCard(
                                            wo = wo,
                                            onAdvanceStatus = { nextStatus ->
                                                ErpBridge.updateWorkOrderStatus(wo.id, nextStatus)
                                                workOrders = ErpBridge.listWorkOrders()
                                                statusMessage = "✓ Advanced ${wo.workOrderNumber} to '$nextStatus' (Logged to CRDT event log)"
                                            },
                                            onViewGenealogy = {
                                                selectedGenealogyWo = wo
                                                genealogyData = ErpBridge.getWorkOrderGenealogy(wo.id)
                                            }
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Work Orders List View
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    modifier = Modifier.fillMaxSize()
                ) {
                    items(workOrders) { wo ->
                        WorkOrderCard(
                            wo = wo,
                            onAdvanceStatus = { nextStatus ->
                                ErpBridge.updateWorkOrderStatus(wo.id, nextStatus)
                                workOrders = ErpBridge.listWorkOrders()
                                statusMessage = "✓ Advanced ${wo.workOrderNumber} to '$nextStatus'"
                            },
                            onViewGenealogy = {
                                selectedGenealogyWo = wo
                                genealogyData = ErpBridge.getWorkOrderGenealogy(wo.id)
                            }
                        )
                    }
                }
            }
        } else {
            // Bill of Materials View
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                items(boms) { bom ->
                    BomCard(bom)
                }
            }
        }
    }

    // Component Genealogy Tree Dialog (LEDGER-15)
    selectedGenealogyWo?.let { wo ->
        AlertDialog(
            onDismissRequest = { selectedGenealogyWo = null },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.AccountTree, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Component Genealogy: ${wo.workOrderNumber}")
                }
            },
            text = {
                val data = genealogyData
                if (data != null) {
                    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer)) {
                            Column(modifier = Modifier.padding(10.dp)) {
                                Text("Finished Good: ${data.finishedName} (${data.finishedSku})", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                                Text("Batch Lot Assigned: ${data.finishedLotNumber}", fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Bold, fontSize = 12.sp, color = MaterialTheme.colorScheme.primary)
                                Text("Output Quantity: ${data.quantity} units", fontSize = 12.sp)
                            }
                        }

                        Text("Consumed Raw Component Lots:", fontWeight = FontWeight.Bold, fontSize = 12.sp)
                        LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.heightIn(max = 240.dp)) {
                            items(data.components) { comp ->
                                Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))) {
                                    Column(modifier = Modifier.padding(8.dp)) {
                                        Text("${comp.componentName} (${comp.componentSku}) • Qty: ${comp.componentQuantity}", fontWeight = FontWeight.SemiBold, fontSize = 12.sp)
                                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                            Text("Lot: ${comp.componentLotNumber ?: "N/A"}", fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                                            comp.expirationDate?.let { Text("Exp: $it", fontSize = 11.sp, color = Color(0xFF047857)) }
                                            comp.quarantineStatus?.let {
                                                Surface(shape = RoundedCornerShape(4.dp), color = if (it == "quarantine") Color(0xFFFDE68A) else Color(0xFFD1FAE5)) {
                                                    Text(it.uppercase(), fontSize = 9.sp, fontWeight = FontWeight.Bold, modifier = Modifier.padding(horizontal = 4.dp, vertical = 1.dp))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    Text("No genealogy data recorded for this work order yet. Complete assembly to link component lots.")
                }
            },
            confirmButton = {
                Button(onClick = { selectedGenealogyWo = null }) {
                    Text("Close")
                }
            }
        )
    }
}

@Composable
fun WorkOrderCard(
    wo: WorkOrderDto,
    onAdvanceStatus: (String) -> Unit,
    onViewGenealogy: () -> Unit
) {
    val isCompleted = wo.status.equals("Completed", ignoreCase = true)
    val statusColor = when (wo.status.lowercase()) {
        "completed" -> Color(0xFF2E7D32)
        "in progress" -> Color(0xFF1976D2)
        "quality check" -> Color(0xFF7C3AED)
        else -> MaterialTheme.colorScheme.outline
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.Engineering,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(20.dp)
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = wo.workOrderNumber,
                        fontWeight = FontWeight.Bold,
                        style = MaterialTheme.typography.titleMedium
                    )
                }

                Surface(
                    color = statusColor.copy(alpha = 0.15f),
                    shape = RoundedCornerShape(16.dp)
                ) {
                    Text(
                        text = wo.status.uppercase(),
                        color = statusColor,
                        fontWeight = FontWeight.Bold,
                        fontSize = 10.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            Text("Item: ${wo.itemId} • Qty: ${wo.targetQuantity} units", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
            Text("Dest: ${wo.destinationLocationId}", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

            Spacer(modifier = Modifier.height(10.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedButton(
                    onClick = onViewGenealogy,
                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp)
                ) {
                    Icon(Icons.Default.AccountTree, contentDescription = null, modifier = Modifier.size(14.dp))
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("Genealogy", fontSize = 11.sp)
                }

                when (wo.status.lowercase()) {
                    "draft", "planned" -> {
                        Button(
                            onClick = { onAdvanceStatus("In Progress") },
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp)
                        ) {
                            Text("Start →", fontSize = 11.sp)
                        }
                    }
                    "in progress" -> {
                        Button(
                            onClick = { onAdvanceStatus("Quality Check") },
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF7C3AED))
                        ) {
                            Text("QA Check →", fontSize = 11.sp)
                        }
                    }
                    "quality check" -> {
                        Button(
                            onClick = { onAdvanceStatus("Completed") },
                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF16A34A))
                        ) {
                            Text("Complete & Link", fontSize = 11.sp)
                        }
                    }
                    else -> {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Check, contentDescription = null, tint = Color(0xFF16A34A), modifier = Modifier.size(16.dp))
                            Spacer(modifier = Modifier.width(4.dp))
                            Text("Done", fontSize = 11.sp, color = Color(0xFF16A34A), fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun BomCard(bom: BillOfMaterialsDto) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = bom.bomNumber,
                        fontWeight = FontWeight.Bold,
                        style = MaterialTheme.typography.titleMedium,
                        fontFamily = FontFamily.Monospace
                    )
                    Text(
                        text = bom.name,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold
                    )
                }

                Surface(
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Text(
                        text = "Batch: ${bom.quantity} unit(s)",
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                        fontSize = 12.sp,
                        fontWeight = FontWeight.Medium
                    )
                }
            }

            bom.routingNotes?.let { notes ->
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = "Routing: $notes",
                    fontSize = 12.sp,
                    fontStyle = androidx.compose.ui.text.font.FontStyle.Italic,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Spacer(modifier = Modifier.height(12.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = "Required Components (${bom.components.size}):",
                fontWeight = FontWeight.SemiBold,
                fontSize = 13.sp
            )

            Spacer(modifier = Modifier.height(6.dp))

            bom.components.forEach { comp ->
                BomComponentRow(comp)
            }
        }
    }
}

@Composable
fun BomComponentRow(comp: BomComponentDto) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                Icons.Default.SubdirectoryArrowRight,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
                tint = MaterialTheme.colorScheme.outline
            )
            Spacer(modifier = Modifier.width(6.dp))
            Column {
                Text(
                    text = comp.notes ?: comp.itemId,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = "Item ID: ${comp.itemId}",
                    fontSize = 11.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "${comp.quantity} units",
                fontSize = 13.sp,
                fontWeight = FontWeight.SemiBold,
                fontFamily = FontFamily.Monospace
            )
            if (comp.scrapPercentage != "0.0" && comp.scrapPercentage != "0") {
                Spacer(modifier = Modifier.width(6.dp))
                Surface(
                    color = MaterialTheme.colorScheme.errorContainer,
                    shape = RoundedCornerShape(4.dp)
                ) {
                    Text(
                        text = "+${comp.scrapPercentage}% scrap",
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp)
                    )
                }
            }
        }
    }
}
