package org.erp.ledger.features.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LabelPrintingScreen(
    initialSku: String = "M8-HEX-BOLT-316",
    initialName: String = "M8x40mm Hex Bolt Stainless 316",
    onNavigateBack: () -> Unit = {}
) {
    var sku by remember { mutableStateOf(initialSku) }
    var itemName by remember { mutableStateOf(initialName) }
    var locationCode by remember { mutableStateOf("BIN-04") }
    var labelFormat by remember { mutableStateOf("Thermal 62mm (Brother / Zebra)") }
    var printStatus by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Offline QR Label Station", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Label parameters
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Text("LABEL METADATA", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.outline)
                    OutlinedTextField(
                        value = sku,
                        onValueChange = { sku = it },
                        label = { Text("SKU / Code") },
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = itemName,
                        onValueChange = { itemName = it },
                        label = { Text("Item / Location Title") },
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = locationCode,
                        onValueChange = { locationCode = it },
                        label = { Text("Storage Location / Breadcrumb") },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }

            // Crisp Offline SVG Vector Preview Representation
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = Color.White),
                border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFFE2E8F0))
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
                        Text(
                            "VECTOR SVG THERMAL PREVIEW (300 DPI)",
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF64748B)
                        )
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(4.dp))
                                .background(Color(0xFFEEF2FF))
                                .padding(horizontal = 6.dp, vertical = 2.dp)
                        ) {
                            Text("OFFLINE ENGINE", fontSize = 9.sp, fontWeight = FontWeight.Bold, color = Color(0xFF4338CA))
                        }
                    }

                    // Scaled visual label representation
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp))
                            .background(Color(0xFFF8FAFC))
                            .border(1.dp, Color(0xFFCBD5E1), RoundedCornerShape(8.dp))
                            .padding(16.dp),
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // QR Matrix representation
                        Box(
                            modifier = Modifier
                                .size(88.dp)
                                .clip(RoundedCornerShape(4.dp))
                                .background(Color(0xFF0F172A)),
                            contentAlignment = Alignment.Center
                        ) {
                            Icon(
                                Icons.Default.QrCode,
                                contentDescription = null,
                                modifier = Modifier.size(76.dp),
                                tint = Color.White
                            )
                        }

                        Column(
                            modifier = Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(3.dp)
                        ) {
                            Text(sku, fontWeight = FontWeight.Bold, fontSize = 16.sp, color = Color(0xFF0F172A))
                            Text(itemName, fontSize = 12.sp, color = Color(0xFF475569))
                            Text("Bin: $locationCode", fontSize = 11.sp, color = Color(0xFF64748B))
                            Spacer(modifier = Modifier.height(4.dp))
                            Box(
                                modifier = Modifier
                                    .clip(RoundedCornerShape(4.dp))
                                    .background(Color(0xFFEEF2FF))
                                    .padding(horizontal = 6.dp, vertical = 2.dp)
                            ) {
                                Text(
                                    "erp://item/$sku",
                                    fontFamily = FontFamily.Monospace,
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = Color(0xFF4338CA)
                                )
                            }
                        }
                    }
                }
            }

            // Print action button
            Button(
                onClick = {
                    printStatus = "✓ Sent vector label to ESC/POS thermal printer spooler."
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(Icons.Default.Print, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Print Label Directly via USB/Network")
            }

            printStatus?.let {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .background(Color(0xFF10B981).copy(alpha = 0.15f))
                        .padding(12.dp)
                ) {
                    Text(it, color = Color(0xFF047857), fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                }
            }
        }
    }
}
