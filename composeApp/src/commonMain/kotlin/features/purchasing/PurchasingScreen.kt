package org.erp.ledger.features.purchasing

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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.PurchaseOrderDto
import uniffi.erp_ffi.PurchaseReceiptLineDto

@Composable
fun PurchasingScreen() {
    var purchaseOrders by remember { mutableStateOf(ErpBridge.listPurchaseOrders()) }
    var showReceiveDialog by remember { mutableStateOf<PurchaseOrderDto?>(null) }
    var selectedPoForPdf by remember { mutableStateOf<PurchaseOrderDto?>(null) }
    var pdfExportNotice by remember { mutableStateOf<String?>(null) }
    val locations = remember { ErpBridge.getStorageLocations() }
    var selectedLocationId by remember { mutableStateOf(locations.firstOrNull()?.id ?: "loc-bin-4") }

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
                    text = "Purchasing & Vendor Orders",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Purchase orders with direct warehouse bin putaway & automated AP posting",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            IconButton(onClick = {
                purchaseOrders = ErpBridge.listPurchaseOrders()
            }) {
                Icon(Icons.Default.Refresh, contentDescription = "Refresh")
            }
        }

        pdfExportNotice?.let {
            Spacer(modifier = Modifier.height(10.dp))
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .background(Color(0xFF10B981).copy(alpha = 0.15f), RoundedCornerShape(8.dp))
                    .padding(10.dp)
            ) {
                Text(it, color = Color(0xFF047857), fontWeight = FontWeight.Bold, fontSize = 12.sp)
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Orders List
        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize()
        ) {
            items(purchaseOrders) { po ->
                PurchaseOrderCard(
                    po = po,
                    onReceive = { showReceiveDialog = po },
                    onExportPdf = { selectedPoForPdf = po }
                )
            }
        }
    }

    selectedPoForPdf?.let { po ->
        AlertDialog(
            onDismissRequest = { selectedPoForPdf = null },
            title = { Text("Vector PO PDF: ${po.poNumber}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Offline Pure-Vector PDF-1.4 generation with item lines and vendor reference.")
                    Text("Total: ${po.grandTotal} USD • Status: ${po.status}", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                }
            },
            confirmButton = {
                Button(onClick = {
                    val dest = "/tmp/${po.poNumber}.pdf"
                    ErpBridge.exportPurchaseOrderPdfToFile(po.id, dest)
                    pdfExportNotice = "✓ Successfully generated and exported ${po.poNumber} to $dest"
                    selectedPoForPdf = null
                }) {
                    Text("Export Vector PDF")
                }
            },
            dismissButton = {
                TextButton(onClick = { selectedPoForPdf = null }) {
                    Text("Cancel")
                }
            }
        )
    }

    // Goods Receipt Dialog
    showReceiveDialog?.let { po ->
        AlertDialog(
            onDismissRequest = { showReceiveDialog = null },
            title = { Text("Receive Goods & Putaway") },
            text = {
                Column {
                    Text("Receiving order ${po.poNumber} (${po.grandTotal} USD)")
                    Spacer(modifier = Modifier.height(12.dp))
                    Text("Select Destination Location / Bin:", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)

                    Spacer(modifier = Modifier.height(6.dp))
                    locations.take(4).forEach { loc ->
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)
                        ) {
                            RadioButton(
                                selected = selectedLocationId == loc.id,
                                onClick = { selectedLocationId = loc.id }
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("[${loc.code}] ${loc.name}", fontSize = 13.sp)
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val lines = po.lines.map { l ->
                            PurchaseReceiptLineDto(
                                id = "",
                                itemId = l.itemId,
                                quantity = l.quantity,
                                lotNumber = "LOT-REC-2026",
                                serialNumber = null
                            )
                        }
                        ErpBridge.receiveGoods(po.id, selectedLocationId, lines)
                        showReceiveDialog = null
                        purchaseOrders = ErpBridge.listPurchaseOrders()
                    }
                ) {
                    Text("Confirm Putaway")
                }
            },
            dismissButton = {
                TextButton(onClick = { showReceiveDialog = null }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
fun PurchaseOrderCard(
    po: PurchaseOrderDto,
    onReceive: () -> Unit,
    onExportPdf: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        elevation = CardDefaults.cardElevation(2.dp)
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.ShoppingCart,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(20.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(text = po.poNumber, fontWeight = FontWeight.Bold, fontSize = 15.sp)
                }

                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = when (po.status) {
                        "Received" -> Color(0xFFD1FAE5)
                        "Submitted" -> Color(0xFFDBEAFE)
                        else -> Color(0xFFF3F4F6)
                    }
                ) {
                    Text(
                        text = po.status,
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = when (po.status) {
                            "Received" -> Color(0xFF065F46)
                            "Submitted" -> Color(0xFF1E40AF)
                            else -> Color.DarkGray
                        },
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            po.notes?.let {
                Text(text = it, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Total: ${po.grandTotal} USD",
                    fontWeight = FontWeight.Bold,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 14.sp
                )

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(
                        onClick = onExportPdf,
                        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 6.dp)
                    ) {
                        Icon(Icons.Default.PictureAsPdf, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(modifier = Modifier.width(4.dp))
                        Text("Vector PDF", fontSize = 12.sp)
                    }

                    if (po.status != "Received") {
                        FilledTonalButton(
                            onClick = onReceive,
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)
                        ) {
                            Icon(Icons.Default.MoveToInbox, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Receive Goods", fontSize = 12.sp)
                        }
                    }
                }
            }
        }
    }
}
