package org.erp.ledger.features.accounting

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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.AccountDto
import uniffi.erp_ffi.TrialBalanceRowDto

@Composable
fun AccountingScreen() {
    var selectedTab by remember { mutableStateOf(0) }
    var accounts by remember { mutableStateOf(ErpBridge.getChartOfAccounts()) }
    var trialBalance by remember { mutableStateOf(ErpBridge.getTrialBalance()) }

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
                    text = "Accounting & General Ledger",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Double-entry ledger with mathematical debit/credit balance enforcement",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            IconButton(onClick = {
                accounts = ErpBridge.getChartOfAccounts()
                trialBalance = ErpBridge.getTrialBalance()
            }) {
                Icon(Icons.Default.Refresh, contentDescription = "Refresh")
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

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
                text = { Text("Chart of Accounts") },
                icon = { Icon(Icons.Default.AccountTree, contentDescription = null) }
            )
            Tab(
                selected = selectedTab == 1,
                onClick = { selectedTab = 1 },
                text = { Text("Trial Balance & P&L") },
                icon = { Icon(Icons.Default.Assessment, contentDescription = null) }
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        if (selectedTab == 0) {
            // Chart of Accounts List
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                items(accounts) { acc ->
                    AccountCard(acc)
                }
            }
        } else {
            // Trial Balance View
            val tb = trialBalance
            if (tb != null) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    elevation = CardDefaults.cardElevation(2.dp)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "Trial Balance (${tb.asOfOrPeriod})",
                                fontWeight = FontWeight.Bold,
                                style = MaterialTheme.typography.titleMedium
                            )

                            val isBalanced = tb.totalDebit == tb.totalCredit
                            Surface(
                                shape = RoundedCornerShape(16.dp),
                                color = if (isBalanced) Color(0xFFD1FAE5) else Color(0xFFFEE2E2)
                            ) {
                                Text(
                                    text = if (isBalanced) "✓ Balanced" else "⚠ Discrepancy",
                                    color = if (isBalanced) Color(0xFF065F46) else Color(0xFF991B1B),
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                                )
                            }
                        }

                        Divider(modifier = Modifier.padding(vertical = 12.dp))

                        LazyColumn(
                            verticalArrangement = Arrangement.spacedBy(6.dp),
                            modifier = Modifier.weight(1f, fill = false)
                        ) {
                            items(tb.rows) { row ->
                                TrialBalanceItemRow(row)
                            }
                        }

                        Divider(modifier = Modifier.padding(vertical = 12.dp))

                        // Totals Row
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text("Total Debit / Credit:", fontWeight = FontWeight.Bold)
                            Text(
                                "${tb.totalDebit} / ${tb.totalCredit}",
                                fontWeight = FontWeight.Bold,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                    }
                }
            } else {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("No trial balance records available.")
                }
            }
        }
    }
}

@Composable
fun AccountCard(acc: AccountDto) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(
                    shape = RoundedCornerShape(6.dp),
                    color = when (acc.rootType) {
                        "Asset" -> Color(0xFFE0F2FE)
                        "Liability" -> Color(0xFFFEF3C7)
                        "Equity" -> Color(0xFFF3E8FF)
                        "Income" -> Color(0xFFDCFCE7)
                        else -> Color(0xFFFFE4E6)
                    }
                ) {
                    Text(
                        text = acc.code,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                        fontSize = 12.sp,
                        color = Color.DarkGray,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }

                Spacer(modifier = Modifier.width(10.dp))

                Column {
                    Text(text = acc.name, fontWeight = FontWeight.SemiBold, fontSize = 14.sp)
                    Text(
                        text = "${acc.rootType} ${if (acc.isGroup) "• Group" else ""}",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Text(
                text = "${acc.balance} USD",
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Bold,
                fontSize = 14.sp
            )
        }
    }
}

@Composable
fun TrialBalanceItemRow(row: TrialBalanceRowDto) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = "[${row.accountCode}] ${row.accountName}",
            fontSize = 13.sp,
            modifier = Modifier.weight(1f)
        )
        Text(
            text = "D: ${row.totalDebit} | C: ${row.totalCredit}",
            fontFamily = FontFamily.Monospace,
            fontSize = 12.sp,
            color = Color.Gray
        )
    }
}
