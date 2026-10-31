package org.erp.ledger.features.projects

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
    var projectTasks by remember { mutableStateOf(ErpBridge.listProjectTasks(null)) }
    var unbilledSummary by remember { mutableStateOf(ErpBridge.getUnbilledHoursSummary(null)) }
    var showLogTimeDialog by remember { mutableStateOf<ProjectDto?>(null) }
    var generatedInvoice by remember { mutableStateOf<InvoiceDto?>(null) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    var showKanbanTasks by remember { mutableStateOf(false) }

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
                    text = "Interactive Kanban task workflows, quick time chips, and automated invoicing",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                FilterChip(
                    selected = showKanbanTasks,
                    onClick = { showKanbanTasks = !showKanbanTasks },
                    label = { Text(if (showKanbanTasks) "Task Kanban" else "Projects View", fontSize = 12.sp) },
                    leadingIcon = { Icon(if (showKanbanTasks) Icons.Default.ViewKanban else Icons.Default.Folder, contentDescription = null, modifier = Modifier.size(16.dp)) }
                )

                IconButton(onClick = {
                    projects = ErpBridge.listProjects()
                    projectTasks = ErpBridge.listProjectTasks(null)
                    unbilledSummary = ErpBridge.getUnbilledHoursSummary(null)
                    statusMessage = null
                    generatedInvoice = null
                }) {
                    Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                }
            }
        }

        Spacer(modifier = Modifier.height(14.dp))

        // LEDGER-14: Unbilled Hours Summary Card
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.6f)),
            shape = RoundedCornerShape(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(14.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text("UNBILLED WORK IN PROGRESS", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                    Text("${unbilledSummary.totalUnbilledHours} hours pending (${unbilledSummary.unbilledEntriesCount} entries)", fontWeight = FontWeight.Bold, fontSize = 14.sp)
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text("BILLABLE VALUE", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                    Text("${unbilledSummary.totalUnbilledAmount} USD", fontWeight = FontWeight.Bold, fontSize = 16.sp, fontFamily = FontFamily.Monospace, color = MaterialTheme.colorScheme.primary)
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

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

        if (showKanbanTasks) {
            // Task Kanban Board (LEDGER-14)
            val columns = listOf("Backlog", "In Progress", "Review", "Done")
            Row(
                modifier = Modifier.fillMaxSize(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                columns.forEach { colName ->
                    val tasksInCol = projectTasks.filter { it.status.equals(colName, ignoreCase = true) }
                    Card(
                        modifier = Modifier.weight(1f).fillMaxHeight(),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
                        shape = RoundedCornerShape(10.dp)
                    ) {
                        Column(modifier = Modifier.padding(8.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(colName, fontWeight = FontWeight.Bold, fontSize = 12.sp)
                                Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surface) {
                                    Text("${tasksInCol.size}", modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp), fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                }
                            }

                            Spacer(modifier = Modifier.height(8.dp))

                            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxSize()) {
                                items(tasksInCol) { task ->
                                    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
                                        Column(modifier = Modifier.padding(10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                            Text(task.title, fontWeight = FontWeight.Bold, fontSize = 12.sp)
                                            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                                                Text("${task.estimatedHours}h est", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                                Surface(shape = RoundedCornerShape(4.dp), color = MaterialTheme.colorScheme.secondaryContainer) {
                                                    Text(task.priority, fontSize = 10.sp, modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp))
                                                }
                                            }

                                            // Advance status button
                                            val nextStatus = when (colName) {
                                                "Backlog" -> "In Progress"
                                                "In Progress" -> "Review"
                                                "Review" -> "Done"
                                                else -> null
                                            }
                                            if (nextStatus != null) {
                                                Button(
                                                    onClick = {
                                                        ErpBridge.updateProjectTaskStatus(task.id, nextStatus)
                                                        projectTasks = ErpBridge.listProjectTasks(null)
                                                        statusMessage = "✓ Advanced '${task.title}' to $nextStatus"
                                                    },
                                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                                    modifier = Modifier.fillMaxWidth()
                                                ) {
                                                    Text("Advance → $nextStatus", fontSize = 10.sp)
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
        } else {
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
                                unbilledSummary = ErpBridge.getUnbilledHoursSummary(null)
                                statusMessage = "Successfully generated invoice ${inv.invoiceNumber} for unbilled hours!"
                            } else {
                                statusMessage = "No unbilled timesheets found to invoice or error occurred."
                            }
                        }
                    )
                }
            }
        }
    }

    // Log Time Dialog with quick chips (LEDGER-14)
    showLogTimeDialog?.let { prj ->
        var hours by remember { mutableStateOf("4.0") }
        var description by remember { mutableStateOf("Hardware assembly and system integration") }
        var isBillable by remember { mutableStateOf(true) }

        AlertDialog(
            onDismissRequest = { showLogTimeDialog = null },
            title = { Text("Log Time: ${prj.name}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Billing Rate: ${prj.billingRateHourly ?: "100.00"} USD/hr",
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    // Quick Time Chips (+1h, +2h, +4h, +8h)
                    Text("Quick Presets:", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("1.0", "2.0", "4.0", "8.0").forEach { h ->
                            FilterChip(
                                selected = hours == h,
                                onClick = { hours = h },
                                label = { Text("+${h}h", fontSize = 11.sp) }
                            )
                        }
                    }

                    OutlinedTextField(
                        value = hours,
                        onValueChange = { hours = it },
                        label = { Text("Hours Spent") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    OutlinedTextField(
                        value = description,
                        onValueChange = { description = it },
                        label = { Text("Activity / Task Description") },
                        modifier = Modifier.fillMaxWidth()
                    )

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
                            unbilledSummary = ErpBridge.getUnbilledHoursSummary(null)
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
