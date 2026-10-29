package org.erp.ledger.features.invoicing

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

data class InvoiceItemUi(
    val id: String,
    val invoiceNumber: String,
    val customerName: String,
    val totalAmount: String,
    var status: String,
    val lineCount: Int
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InvoicingScreen() {
    val invoices = remember {
        mutableStateListOf(
            InvoiceItemUi("inv-1", "INV-2026-001", "Acme Corporation", "$1,450.00", "Finalized", 3),
            InvoiceItemUi("inv-2", "INV-2026-002", "North Star Logistics", "$520.80", "Draft", 2),
            InvoiceItemUi("inv-3", "INV-2026-003", "Apex Robotics Ltd", "$3,110.00", "Draft", 5)
        )
    }

    var notificationMessage by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("Invoices & Billing", fontWeight = FontWeight.Bold)
                        Text(
                            "Quotes, sales invoices, and automated stock deductions",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
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
            notificationMessage?.let {
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

            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                items(invoices) { inv ->
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column {
                                    Text(inv.invoiceNumber, fontWeight = FontWeight.Bold, fontSize = 16.sp)
                                    Text(inv.customerName, color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 13.sp)
                                }

                                val isFinalized = inv.status == "Finalized"
                                Box(
                                    modifier = Modifier
                                        .clip(RoundedCornerShape(6.dp))
                                        .background(if (isFinalized) Color(0xFF10B981).copy(alpha = 0.15f) else Color(0xFFF59E0B).copy(alpha = 0.15f))
                                        .padding(horizontal = 8.dp, vertical = 4.dp)
                                ) {
                                    Text(
                                        inv.status,
                                        fontWeight = FontWeight.Bold,
                                        fontSize = 12.sp,
                                        color = if (isFinalized) Color(0xFF047857) else Color(0xFFB45309)
                                    )
                                }
                            }

                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text("${inv.lineCount} item line(s) • Total: ${inv.totalAmount}", fontWeight = FontWeight.SemiBold)

                                if (inv.status == "Draft") {
                                    Button(
                                        onClick = {
                                            inv.status = "Finalized"
                                            notificationMessage = "✓ Invoice ${inv.invoiceNumber} finalized: mod-inventory automatically deducted stock from primary bins!"
                                        },
                                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)
                                    ) {
                                        Icon(Icons.Default.Check, contentDescription = null, modifier = Modifier.size(16.dp))
                                        Spacer(modifier = Modifier.width(4.dp))
                                        Text("Finalize & Pick Stock", fontSize = 12.sp)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
