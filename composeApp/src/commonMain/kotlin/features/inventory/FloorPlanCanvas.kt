package org.erp.ledger.features.inventory

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.LocationDto
import kotlin.math.sqrt

/**
 * 2D Interactive Warehouse Floor Plan Canvas.
 * Supports clicking and drag-and-drop to update 2D map coordinates in SQLite.
 */
@Composable
fun FloorPlanCanvas(
    locations: List<LocationDto>,
    activeLocation: LocationDto?,
    onLocationTapped: (LocationDto) -> Unit = {},
    onLocationMoved: (LocationDto, Double, Double) -> Unit = { loc, x, y ->
        ErpBridge.updateLocationCoordinates(loc.id, x, y)
    },
    modifier: Modifier = Modifier
) {
    // Local coordinate map allowing instant 60fps drag feedback before and after DB sync
    var localCoords by remember(locations) {
        mutableStateOf(
            locations.associate { it.id to (Offset(it.mapX?.toFloat() ?: 100f, it.mapY?.toFloat() ?: 100f)) }
        )
    }

    var draggingLocationId by remember { mutableStateOf<String?>(null) }
    val draggedLoc = locations.find { it.id == draggingLocationId }
    val draggedOffset = draggingLocationId?.let { localCoords[it] }

    Column(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(Color(0xFF0F172A))
            .border(1.dp, Color(0xFF334155), RoundedCornerShape(12.dp))
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                "SPATIAL FLOOR PLAN (DRAG & DROP TO REPOSITION)",
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = Color(0xFF94A3B8)
            )
            val infoText = when {
                draggedLoc != null && draggedOffset != null -> {
                    "MOVING: ${draggedLoc.name} (${draggedOffset.x.toInt()}, ${draggedOffset.y.toInt()})"
                }
                activeLocation != null -> {
                    "TARGET: ${activeLocation.name} (${activeLocation.mapX?.toInt() ?: 0}, ${activeLocation.mapY?.toInt() ?: 0})"
                }
                else -> "TAP TO TARGET • DRAG NODE TO REPOSITION"
            }
            Text(
                infoText,
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = if (draggedLoc != null) Color(0xFFFBBF24) else Color(0xFF34D399)
            )
        }

        Canvas(
            modifier = Modifier
                .fillMaxWidth()
                .height(200.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(Color(0xFF020617))
                .pointerInput(locations) {
                    detectTapGestures { tapOffset ->
                        val tapped = locations.minByOrNull { loc ->
                            val pos = localCoords[loc.id] ?: Offset(loc.mapX?.toFloat() ?: -100f, loc.mapY?.toFloat() ?: -100f)
                            val dx = tapOffset.x - pos.x
                            val dy = tapOffset.y - pos.y
                            dx * dx + dy * dy
                        }
                        if (tapped != null) {
                            val pos = localCoords[tapped.id] ?: Offset(tapped.mapX?.toFloat() ?: -100f, tapped.mapY?.toFloat() ?: -100f)
                            val dist = sqrt((tapOffset.x - pos.x) * (tapOffset.x - pos.x) + (tapOffset.y - pos.y) * (tapOffset.y - pos.y))
                            if (dist < 40f) {
                                onLocationTapped(tapped)
                            }
                        }
                    }
                }
                .pointerInput(locations) {
                    detectDragGestures(
                        onDragStart = { startOffset ->
                            val closest = locations.minByOrNull { loc ->
                                val pos = localCoords[loc.id] ?: Offset(loc.mapX?.toFloat() ?: -100f, loc.mapY?.toFloat() ?: -100f)
                                val dx = startOffset.x - pos.x
                                val dy = startOffset.y - pos.y
                                dx * dx + dy * dy
                            }
                            if (closest != null) {
                                val pos = localCoords[closest.id] ?: Offset(closest.mapX?.toFloat() ?: -100f, closest.mapY?.toFloat() ?: -100f)
                                val dist = sqrt((startOffset.x - pos.x) * (startOffset.x - pos.x) + (startOffset.y - pos.y) * (startOffset.y - pos.y))
                                if (dist < 40f) {
                                    draggingLocationId = closest.id
                                    onLocationTapped(closest)
                                }
                            }
                        },
                        onDrag = { change, dragAmount ->
                            change.consume()
                            val locId = draggingLocationId ?: return@detectDragGestures
                            val currentPos = localCoords[locId] ?: return@detectDragGestures
                            val newX = (currentPos.x + dragAmount.x).coerceIn(10f, 1000f)
                            val newY = (currentPos.y + dragAmount.y).coerceIn(10f, 500f)
                            localCoords = localCoords.toMutableMap().apply {
                                put(locId, Offset(newX, newY))
                            }
                        },
                        onDragEnd = {
                            val locId = draggingLocationId
                            if (locId != null) {
                                val loc = locations.find { it.id == locId }
                                val finalPos = localCoords[locId]
                                if (loc != null && finalPos != null) {
                                    onLocationMoved(loc, finalPos.x.toDouble(), finalPos.y.toDouble())
                                }
                            }
                            draggingLocationId = null
                        },
                        onDragCancel = {
                            draggingLocationId = null
                        }
                    )
                }
        ) {
            // 1. Draw coordinate grid
            val gridSpacing = 40f
            for (x in 0..(size.width / gridSpacing).toInt()) {
                drawLine(
                    color = Color(0xFF1E293B),
                    start = Offset(x * gridSpacing, 0f),
                    end = Offset(x * gridSpacing, size.height),
                    strokeWidth = 1f
                )
            }
            for (y in 0..(size.height / gridSpacing).toInt()) {
                drawLine(
                    color = Color(0xFF1E293B),
                    start = Offset(0f, y * gridSpacing),
                    end = Offset(size.width, y * gridSpacing),
                    strokeWidth = 1f
                )
            }

            // 2. Draw static warehouse perimeter zones
            drawRect(
                color = Color(0xFF1E3A8A).copy(alpha = 0.25f),
                topLeft = Offset(20f, 20f),
                size = Size(80f, 150f),
                style = Stroke(width = 2f)
            )

            drawRect(
                color = Color(0xFF065F46).copy(alpha = 0.25f),
                topLeft = Offset(130f, 20f),
                size = Size(200f, 150f),
                style = Stroke(width = 2f)
            )

            // 3. Draw Rack, Room, Bin and Warehouse nodes
            for (loc in locations) {
                val pos = localCoords[loc.id] ?: Offset(loc.mapX?.toFloat() ?: continue, loc.mapY?.toFloat() ?: continue)
                val isTarget = loc.id == activeLocation?.id
                val isBeingDragged = loc.id == draggingLocationId

                if (isBeingDragged) {
                    // Pulsing amber halo while actively dragging
                    drawCircle(
                        color = Color(0xFFF59E0B).copy(alpha = 0.35f),
                        radius = 32f,
                        center = pos
                    )
                    drawCircle(
                        color = Color(0xFFFBBF24),
                        radius = 20f,
                        center = pos,
                        style = Stroke(width = 3f)
                    )
                    drawCircle(
                        color = Color(0xFFF59E0B),
                        radius = 9f,
                        center = pos
                    )
                } else if (isTarget) {
                    // Green radar rings for target
                    drawCircle(
                        color = Color(0xFF10B981).copy(alpha = 0.2f),
                        radius = 28f,
                        center = pos
                    )
                    drawCircle(
                        color = Color(0xFF10B981).copy(alpha = 0.4f),
                        radius = 18f,
                        center = pos,
                        style = Stroke(width = 2f)
                    )
                    drawCircle(
                        color = Color(0xFF34D399),
                        radius = 8f,
                        center = pos
                    )
                } else {
                    // Distinct color coding based on location hierarchy type
                    val nodeColor = when (loc.locationType) {
                        "Warehouse" -> Color(0xFF64748B)
                        "Room" -> Color(0xFF3B82F6)
                        "Rack" -> Color(0xFFF59E0B)
                        "Shelf" -> Color(0xFFA855F7)
                        else -> Color(0xFF10B981)
                    }
                    drawCircle(
                        color = nodeColor.copy(alpha = 0.85f),
                        radius = 6f,
                        center = pos
                    )
                }
            }
        }
    }
}
