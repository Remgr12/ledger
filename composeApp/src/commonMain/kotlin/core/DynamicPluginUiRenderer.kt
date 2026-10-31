package org.erp.ledger.core

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import uniffi.erp_ffi.PluginUiNodeDto

/**
 * DynamicPluginUiRenderer: Native Compose Multiplatform renderer for declarative
 * WebAssembly plugin UI trees returned across the Mozilla UniFFI bridge.
 */
@Composable
fun DynamicPluginUiRenderer(
    nodes: List<PluginUiNodeDto>,
    onActionClick: (buttonId: String) -> Unit = {},
    onFieldValueChange: (fieldKey: String, value: String) -> Unit = { _, _ -> },
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        for (node in nodes) {
            PluginNodeComponent(
                node = node,
                onActionClick = onActionClick,
                onFieldValueChange = onFieldValueChange
            )
        }
    }
}

@Composable
fun PluginNodeComponent(
    node: PluginUiNodeDto,
    onActionClick: (buttonId: String) -> Unit,
    onFieldValueChange: (fieldKey: String, value: String) -> Unit
) {
    when (node.nodeKind) {
        "Container" -> {
            val isRow = node.titleOrLabel.equals("Row", ignoreCase = true)
            val spacing = (node.secondaryText?.toIntOrNull() ?: 8).dp

            if (isRow) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(spacing),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    for (child in node.children) {
                        PluginNodeComponent(child, onActionClick, onFieldValueChange)
                    }
                }
            } else {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(spacing)
                ) {
                    for (child in node.children) {
                        PluginNodeComponent(child, onActionClick, onFieldValueChange)
                    }
                }
            }
        }

        "Card" -> {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                ),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    for (child in node.children) {
                        PluginNodeComponent(child, onActionClick, onFieldValueChange)
                    }
                }
            }
        }

        "Text" -> {
            val style = node.secondaryText ?: "Body"
            when (style) {
                "Headline" -> Text(
                    text = node.titleOrLabel,
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
                "Title" -> Text(
                    text = node.titleOrLabel,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface
                )
                "Caption" -> Text(
                    text = node.titleOrLabel,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                else -> Text(
                    text = node.titleOrLabel,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }
        }

        "Badge" -> {
            val parsedColor = parseHexColor(node.colorHex) ?: MaterialTheme.colorScheme.primary
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(6.dp))
                    .background(parsedColor.copy(alpha = 0.15f))
                    .border(1.dp, parsedColor.copy(alpha = 0.4f), RoundedCornerShape(6.dp))
                    .padding(horizontal = 8.dp, vertical = 4.dp)
            ) {
                Text(
                    text = node.titleOrLabel,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = parsedColor
                )
            }
        }

        "StatCard" -> {
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(10.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text(
                            text = node.titleOrLabel,
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            text = node.secondaryText ?: "-",
                            style = MaterialTheme.typography.headlineMedium,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                    }
                    node.colorHex?.let { color ->
                        Text(
                            text = color,
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary
                        )
                    }
                }
            }
        }

        "Button" -> {
            val style = node.secondaryText ?: "Primary"
            val buttonId = node.customFieldKey ?: node.nodeId

            when (style) {
                "Secondary" -> OutlinedButton(
                    onClick = { onActionClick(buttonId) }
                ) {
                    Text(node.titleOrLabel)
                }
                "Danger" -> Button(
                    onClick = { onActionClick(buttonId) },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text(node.titleOrLabel, color = MaterialTheme.colorScheme.onError)
                }
                else -> Button(
                    onClick = { onActionClick(buttonId) }
                ) {
                    Text(node.titleOrLabel)
                }
            }
        }

        "CustomFieldInput" -> {
            var textValue by remember { mutableStateOf(node.secondaryText ?: "") }
            OutlinedTextField(
                value = textValue,
                onValueChange = {
                    textValue = it
                    node.customFieldKey?.let { key -> onFieldValueChange(key, it) }
                },
                label = { Text(node.titleOrLabel) },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
        }
    }
}

private fun parseHexColor(hex: String?): Color? {
    if (hex == null || !hex.startsWith("#") || hex.length !in listOf(7, 9)) return null
    return try {
        val colorInt = hex.removePrefix("#").toLong(16)
        if (hex.length == 7) {
            Color((0xFF000000 or colorInt).toInt())
        } else {
            Color(colorInt)
        }
    } catch (_: Exception) {
        null
    }
}
