package org.erp.ledger.features.inventory

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
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
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.ScanResolutionTypeDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RapidScanMoveHud(
    onNavigateBack: () -> Unit = {}
) {
    val activeLocation by ErpBridge.activeContextLocation.collectAsState()
    val recentResolution by ErpBridge.recentScanResolution.collectAsState()

    var isAuditMode by remember { mutableStateOf(false) }
    var simulatedBarcode by remember { mutableStateOf("") }
    var transferQuantity by remember { mutableStateOf("1.0") }
    var auditCount by remember { mutableStateOf(140) }
    var confirmationMessage by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            if (isAuditMode) "Scan-to-Audit (Cycle Count)" else "Rapid Scan-Move HUD",
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            if (isAuditMode) "Verify physical vs system stock" else "2-Tap: Scan Bin → Scan Item",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    // Mode Toggle (Move vs Audit)
                    FilterChip(
                        selected = isAuditMode,
                        onClick = { isAuditMode = !isAuditMode },
                        label = { Text(if (isAuditMode) "Audit Mode" else "Move Mode", fontSize = 11.sp) },
                        leadingIcon = {
                            Icon(
                                if (isAuditMode) Icons.Default.Checklist else Icons.Default.SwapHoriz,
                                contentDescription = null,
                                modifier = Modifier.size(16.dp)
                            )
                        }
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    if (activeLocation != null) {
                        TextButton(onClick = { ErpBridge.clearActiveContextLocation() }) {
                            Text("Clear", color = MaterialTheme.colorScheme.error)
                        }
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
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            // STEP 1 BANNER: Active Target Location Context HUD
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(
                    containerColor = if (activeLocation != null) {
                        if (isAuditMode) Color(0xFF1E3A8A) else Color(0xFF065F46)
                    } else MaterialTheme.colorScheme.surfaceVariant
                )
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(40.dp)
                            .clip(CircleShape)
                            .background(
                                if (activeLocation != null) {
                                    if (isAuditMode) Color(0xFF3B82F6) else Color(0xFF10B981)
                                } else MaterialTheme.colorScheme.outline
                            ),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            if (isAuditMode) Icons.Default.FactCheck else Icons.Default.Place,
                            contentDescription = null,
                            tint = Color.White
                        )
                    }

                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = if (activeLocation != null) {
                                if (isAuditMode) "AUDITING TARGET BIN" else "STEP 1: TARGET LOCATION ACTIVE"
                            } else "STEP 1: SCAN A BIN / LOCATION QR",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (activeLocation != null) Color(0xFFA7F3D0) else MaterialTheme.colorScheme.outline
                        )
                        Text(
                            text = activeLocation?.name ?: "No target shelf/bin active",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = if (activeLocation != null) Color.White else MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        activeLocation?.pathBreadcrumb?.let {
                            Text(
                                text = it,
                                style = MaterialTheme.typography.bodySmall,
                                color = Color(0xFFD1FAE5)
                            )
                        }
                    }
                }
            }

            // SIMULATED CAMERA SCANNER VIEWFINDER
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f)
                    .clip(RoundedCornerShape(16.dp))
                    .background(Color(0xFF0F172A))
                    .border(
                        2.dp,
                        if (activeLocation != null) {
                            if (isAuditMode) Color(0xFF3B82F6) else Color(0xFF10B981)
                        } else Color(0xFF64748B),
                        RoundedCornerShape(16.dp)
                    ),
                contentAlignment = Alignment.Center
            ) {
                // Viewfinder Reticle / Crosshairs
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center
                ) {
                    Icon(
                        Icons.Default.CropFree,
                        contentDescription = "Target",
                        modifier = Modifier.size(130.dp),
                        tint = if (activeLocation != null) {
                            if (isAuditMode) Color(0xFF60A5FA) else Color(0xFF10B981)
                        } else Color(0xFF94A3B8)
                    )
                    Spacer(modifier = Modifier.height(10.dp))
                    Text(
                        text = if (activeLocation != null) {
                            if (isAuditMode) "Scan item to verify physical count in this bin" else "Point camera at Product/Part QR to Putaway"
                        } else "Point camera at Bin / Shelf QR code",
                        color = Color.White.copy(alpha = 0.85f),
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Medium
                    )
                }

                // Temporary scan feedback banner
                AnimatedVisibility(
                    visible = confirmationMessage != null,
                    enter = fadeIn() + slideInVertically(),
                    exit = fadeOut() + slideOutVertically(),
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = 16.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(8.dp))
                            .background(Color(0xFF10B981))
                            .padding(horizontal = 16.dp, vertical = 8.dp)
                    ) {
                        Text(
                            text = confirmationMessage ?: "",
                            color = Color.White,
                            fontWeight = FontWeight.Bold,
                            fontSize = 13.sp
                        )
                    }
                }
            }

            // SIMULATION BARCODE QUICK-TRIGGER BUTTONS (Mobile & Desktop HID Testing)
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
            ) {
                Column(
                    modifier = Modifier.padding(14.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Text(
                        text = "RAPID SIMULATOR / USB KEYBOARD SCANNER",
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.outline
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedButton(
                            onClick = {
                                val res = ErpBridge.resolveQrScan("erp://loc/loc-bin-4")
                                confirmationMessage = "✓ Scanned Bin: ${res.location?.name}"
                            },
                            modifier = Modifier.weight(1f)
                        ) {
                            Text("Scan Bin #4", fontSize = 12.sp)
                        }

                        Button(
                            onClick = {
                                val res = ErpBridge.resolveQrScan("erp://item/M8-HEX-BOLT-316")
                                if (res.resolutionType == ScanResolutionTypeDto.RAPID_STOCK_ACTION) {
                                    confirmationMessage = "✓ Ready to putaway ${res.item?.sku}"
                                }
                            },
                            modifier = Modifier.weight(1f)
                        ) {
                            Text("Scan M8 Bolt", fontSize = 12.sp)
                        }

                        OutlinedButton(
                            onClick = {
                                val res = ErpBridge.resolveQrScan("(01)07350012345678(10)LOT-2026A")
                                confirmationMessage = "✓ Parsed GS1: ${res.item?.sku ?: "07350012345678"}"
                            },
                            modifier = Modifier.weight(1f)
                        ) {
                            Text("Scan GS1 AI", fontSize = 12.sp)
                        }
                    }

                    // Manual payload entry
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        OutlinedTextField(
                            value = simulatedBarcode,
                            onValueChange = { simulatedBarcode = it },
                            placeholder = { Text("erp://loc/... or erp://item/...") },
                            modifier = Modifier.weight(1f),
                            singleLine = true
                        )
                        Button(
                            onClick = {
                                if (simulatedBarcode.isNotEmpty()) {
                                    ErpBridge.resolveQrScan(simulatedBarcode)
                                    simulatedBarcode = ""
                                }
                            }
                        ) {
                            Text("Send")
                        }
                    }
                }
            }

            // WORKFLOW ACTIONS BASED ON MODE
            if (isAuditMode && activeLocation != null) {
                // CYCLE COUNT AUDIT RECONCILIATION CARD
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(14.dp),
                    colors = CardDefaults.cardColors(containerColor = Color(0xFFEFF6FF))
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Text(
                            "CYCLE COUNT AUDIT: M8x40mm Hex Bolt Stainless 316",
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF1E3A8A),
                            fontSize = 13.sp
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Text("SYSTEM BALANCE", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6B7280))
                                Text("142.0 pcs", fontSize = 18.sp, fontWeight = FontWeight.Bold, color = Color(0xFF1F2937))
                            }

                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("PHYSICAL COUNT", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6B7280))
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    IconButton(onClick = { if (auditCount > 0) auditCount -= 1 }) {
                                        Icon(Icons.Default.Remove, contentDescription = "-1")
                                    }
                                    Text("$auditCount pcs", fontSize = 18.sp, fontWeight = FontWeight.Bold, color = Color(0xFF2563EB))
                                    IconButton(onClick = { auditCount += 1 }) {
                                        Icon(Icons.Default.Add, contentDescription = "+1")
                                    }
                                }
                            }

                            Column(horizontalAlignment = Alignment.End) {
                                val discrepancy = auditCount - 142
                                Text("DISCREPANCY", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6B7280))
                                Text(
                                    "${if (discrepancy > 0) "+$discrepancy" else "$discrepancy"} pcs",
                                    fontSize = 18.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = if (discrepancy == 0) Color(0xFF059669) else Color(0xFFDC2626)
                                )
                            }
                        }

                        Button(
                            onClick = {
                                val loc = activeLocation
                                if (loc != null) {
                                    ErpBridge.reconcileAudit(loc.id, "item-1", auditCount.toString(), "LOT-2026-Q1")
                                    confirmationMessage = "✓ Cycle Count Audit reconciled in immutable stock ledger!"
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2563EB))
                        ) {
                            Icon(Icons.Default.DoneAll, contentDescription = null)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Reconcile & Commit Ledger Adjustment")
                        }
                    }
                }
            } else if (recentResolution?.resolutionType == ScanResolutionTypeDto.RAPID_STOCK_ACTION) {
                // RAPID PUTAWAY / MOVE WORKFLOW CARD
                val actionRes = recentResolution!!
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(14.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer
                    )
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Text(
                                    text = "ACTION: ${actionRes.suggestedAction}",
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                                    fontSize = 12.sp
                                )
                                Text(
                                    text = actionRes.item?.name ?: "",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.onPrimaryContainer
                                )
                            }
                            Text(
                                text = "Target: ${actionRes.location?.name}",
                                style = MaterialTheme.typography.labelMedium,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }

                        // Quantity Selector
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            listOf("1.0", "5.0", "10.0", "25.0").forEach { q ->
                                FilterChip(
                                    selected = transferQuantity == q,
                                    onClick = { transferQuantity = q },
                                    label = { Text("+$q") }
                                )
                            }
                        }

                        // One-tap Confirm button
                        Button(
                            onClick = {
                                val item = actionRes.item
                                val loc = actionRes.location
                                if (item != null && loc != null) {
                                    ErpBridge.executeQuickTransfer(item.id, loc.id, transferQuantity)
                                    confirmationMessage = "✓ Transferred $transferQuantity ${item.unit} into ${loc.name}"
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = MaterialTheme.colorScheme.primary
                            )
                        ) {
                            Icon(Icons.Default.Check, contentDescription = null)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Confirm & Record Stock Movement")
                        }
                    }
                }
            }
        }
    }
}
