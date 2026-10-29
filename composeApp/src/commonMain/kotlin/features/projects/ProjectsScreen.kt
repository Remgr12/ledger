package org.erp.ledger.features.projects

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
import uniffi.erp_ffi.InvoiceDto
import uniffi.erp_ffi.ProjectDto

@Composable
fun ProjectsScreen() {
    var projects by remember { mutableStateOf(ErpBridge.listProjects()) }
    var showLogTimeDialog by remember { mutableStateOf<ProjectDto?>(null) }
    var generatedInvoice by remember { mutableStateOf<InvoiceDto?>(null) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

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
                    text = "Projects & Billable Timesheets",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Track project time, billable hours, and 1-click automated sales invoice generation",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            IconButton(onClick = {
                projects = ErpBridge.listProjects()
                statusMessage = null
                generatedInvoice = null
            }) {
                Icon(Icons.Default.Refresh, contentDescription = "Refresh")
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Status or Generated Invoice notification
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

        generatedInvoice?.let { inv ->
            Surface(
                color = MaterialTheme.colorScheme.secondaryContainer,
                shape = RoundedCornerShape(8.dp),
                modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp)
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column {
                        Text(
                            text = "Generated Sales Invoice: ${inv.invoiceNumber}",
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSecondaryContainer
                        )
                        Text(
                            text = "Grand Total: ${inv.grandTotal} USD | Status: ${inv.status}",
                            fontSize = 12.sp,
                            color = MaterialTheme.colorScheme.onSecondaryContainer
                        )
                    }
                    Button(
                        onClick = { ErpBridge.navigateTo("invoicing") },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary
                        )
                    ) {
                        Text("View Invoice", fontSize = 12.sp)
                    }
                }
            }
        }

        // Projects List
        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize()
        ) {
            items(projects) { project ->
                ProjectCard(
                    project = project,
                    onLogTime = { showLogTimeDialog = project },
                    onInvoiceTimesheets = {
                        val inv = ErpBridge.invoiceUnbilledTimesheets(project.id)
                        if (inv != null) {
                            generatedInvoice = inv
                            statusMessage = "Successfully generated invoice ${inv.invoiceNumber} for unbilled hours!"
                        } else {
                            statusMessage = "No unbilled timesheets found to invoice or error occurred."
                        }
                    }
                )
            }
        }
    }

    // Log Time Dialog
    showLogTimeDialog?.let { prj ->
        var hours by remember { mutableStateOf("4.0") }
        var description by remember { mutableStateOf("Hardware assembly and system integration") }
        var isBillable by remember { mutableStateOf(true) }

        AlertDialog(
            onDismissRequest = { showLogTimeDialog = null },
            title = { Text("Log Time: ${prj.name}") },
            text = {
                Column {
                    Text(
                        "Billing Rate: ${prj.billingRateHourly ?: "100.00"} USD/hr",
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(12.dp))

                    OutlinedTextField(
                        value = hours,
                        onValueChange = { hours = it },
                        label = { Text("Hours Spent") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    OutlinedTextField(
                        value = description,
                        onValueChange = { description = it },
                        label = { Text("Activity / Task Description") },
                        modifier = Modifier.fillMaxWidth()
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Checkbox(
                            checked = isBillable,
                            onCheckedChange = { isBillable = it }
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text("Billable to Customer")
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val success = ErpBridge.recordTimesheet(
                            projectId = prj.id,
                            hours = hours,
                            description = description,
                            isBillable = isBillable
                        )
                        if (success) {
                            statusMessage = "Logged $hours hours for ${prj.name}."
                        }
                        showLogTimeDialog = null
                    }
                ) {
                    Text("Save Timesheet")
                }
            },
            dismissButton = {
                TextButton(onClick = { showLogTimeDialog = null }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
fun ProjectCard(
    project: ProjectDto,
    onLogTime: () -> Unit,
    onInvoiceTimesheets: () -> Unit
) {
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
                        text = project.code,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary,
                        fontSize = 12.sp
                    )
                    Text(
                        text = project.name,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold
                    )
                }

                Surface(
                    color = MaterialTheme.colorScheme.primary.copy(alpha = 0.12f),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Text(
                        text = project.status.uppercase(),
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Bold,
                        fontSize = 11.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text("Customer: ${project.customerId}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(
                        "Rate: ${project.billingRateHourly ?: "N/A"} USD/hr",
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 13.sp
                    )
                }

                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        "Budget: ${project.estimatedHours ?: "0.0"} hrs",
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(10.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                OutlinedButton(
                    onClick = onLogTime,
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Icon(Icons.Default.Timer, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("Log Time", fontSize = 12.sp)
                }

                Spacer(modifier = Modifier.width(8.dp))

                Button(
                    onClick = onInvoiceTimesheets,
                    shape = RoundedCornerShape(8.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary
                    )
                ) {
                    Icon(Icons.Default.ReceiptLong, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("1-Click Invoice", fontSize = 12.sp)
                }
            }
        }
    }
}
