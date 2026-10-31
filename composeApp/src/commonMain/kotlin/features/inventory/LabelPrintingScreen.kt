package org.erp.ledger.features.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import org.erp.ledger.printing.LabelFormat
import org.erp.ledger.printing.ThermalPrintGenerator
import org.erp.ledger.printing.ThermalPrinterService

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
    var selectedFormat by remember { mutableStateOf(LabelFormat.BROTHER_62MM) }
    var copies by remember { mutableStateOf(1) }
    var printStatus by remember { mutableStateOf<String?>(null) }
    var isStatusError by remember { mutableStateOf(false) }
    var rawPortInput by remember { mutableStateOf("/dev/usb/lp0") }
    var showRawInspector by remember { mutableStateOf(false) }

    val scrollState = rememberScrollState()

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
                .verticalScroll(scrollState)
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
                    Text(
                        "LABEL METADATA & FORMAT",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        OutlinedTextField(
                            value = sku,
                            onValueChange = { sku = it },
                            label = { Text("SKU / Code") },
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = locationCode,
                            onValueChange = { locationCode = it },
                            label = { Text("Storage Bin") },
                            modifier = Modifier.width(130.dp)
                        )
                    }

                    OutlinedTextField(
                        value = itemName,
                        onValueChange = { itemName = it },
                        label = { Text("Item / Location Title") },
                        modifier = Modifier.fillMaxWidth()
                    )

                    // Format selector buttons
                    Text(
                        "MEDIA & FORM FACTOR",
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        LabelFormat.values().forEach { format ->
                            FilterChip(
                                selected = selectedFormat == format,
                                onClick = { selectedFormat = format },
                                label = {
                                    Text(
                                        when (format) {
                                            LabelFormat.BROTHER_62MM -> "Brother 62mm"
                                            LabelFormat.AVERY_DIE_CUT -> "Avery Die-Cut"
                                            LabelFormat.ESCPOS_58MM -> "ESC/POS 58mm"
                                            LabelFormat.TSPL_4X6 -> "TSPL 4x6\""
                                        },
                                        fontSize = 11.sp
                                    )
                                }
                            )
                        }
                    }

                    Text(
                        selectedFormat.description,
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    // Copies and port
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("Copies: ", fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                            IconButton(onClick = { if (copies > 1) copies-- }) {
                                Icon(Icons.Default.Remove, contentDescription = "Decrease")
                            }
                            Text("$copies", fontWeight = FontWeight.Bold, fontSize = 14.sp)
                            IconButton(onClick = { copies++ }) {
                                Icon(Icons.Default.Add, contentDescription = "Increase")
                            }
                        }

                        OutlinedTextField(
                            value = rawPortInput,
                            onValueChange = { rawPortInput = it },
                            label = { Text("Raw Port / Spooler") },
                            modifier = Modifier.weight(1f),
                            singleLine = true
                        )
                    }
                }
            }

            // Crisp Offline Vector Preview Representation
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
                            "VECTOR THERMAL PREVIEW (${selectedFormat.widthMm.toInt()} x ${selectedFormat.heightMm.toInt()} mm)",
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
                            Text(
                                if (selectedFormat.isContinuous) "CONTINUOUS ROLL" else "DIE-CUT SHEET",
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF4338CA)
                            )
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

            // Print action buttons
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Button(
                    onClick = {
                        val result = ThermalPrinterService.printViaSystemDialog(
                            sku = sku,
                            itemName = itemName,
                            locationCode = locationCode,
                            format = selectedFormat,
                            copies = copies
                        )
                        printStatus = result.message
                        isStatusError = !result.success
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Icon(Icons.Default.Print, contentDescription = null)
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("System Print Dialog")
                }

                OutlinedButton(
                    onClick = {
                        val rawBytes = when (selectedFormat) {
                            LabelFormat.TSPL_4X6 -> {
                                val tspl = ThermalPrintGenerator.generateTspl(sku, itemName, locationCode, copies)
                                tspl.encodeToByteArray()
                            }
                            else -> {
                                ThermalPrintGenerator.generateEscPos(sku, itemName, locationCode, copies)
                            }
                        }

                        val result = ThermalPrinterService.sendRawToPrinter(
                            rawBytes = rawBytes,
                            targetPortOrDevice = rawPortInput.ifBlank { null }
                        )
                        printStatus = result.message
                        isStatusError = !result.success
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Icon(Icons.Default.Bolt, contentDescription = null)
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("Raw ESC/POS Spooler")
                }
            }

            // Status message
            printStatus?.let { msg ->
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (isStatusError) Color(0xFFFEE2E2) else Color(0xFF10B981).copy(alpha = 0.15f))
                        .padding(12.dp)
                ) {
                    Text(
                        msg,
                        color = if (isStatusError) Color(0xFFB91C1C) else Color(0xFF047857),
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 13.sp
                    )
                }
            }

            // Raw protocol inspector accordion
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            "RAW PROTOCOL STREAM (ESC/POS & TSPL)",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.outline
                        )
                        TextButton(onClick = { showRawInspector = !showRawInspector }) {
                            Text(if (showRawInspector) "Hide" else "Inspect")
                        }
                    }

                    if (showRawInspector) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Text("TSPL / ZPL Command Output:", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        Text(
                            ThermalPrintGenerator.generateTspl(sku, itemName, locationCode, copies),
                            fontFamily = FontFamily.Monospace,
                            fontSize = 10.sp,
                            color = MaterialTheme.colorScheme.primary
                        )

                        Spacer(modifier = Modifier.height(8.dp))
                        val escposBytes = ThermalPrintGenerator.generateEscPos(sku, itemName, locationCode, 1)
                        Text("ESC/POS Binary Stream (${escposBytes.size} bytes):", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        Text(
                            escposBytes.take(48).joinToString(" ") { "%02X".format(it) } + "...",
                            fontFamily = FontFamily.Monospace,
                            fontSize = 10.sp,
                            color = MaterialTheme.colorScheme.outline
                        )
                    }
                }
            }
        }
    }
}
