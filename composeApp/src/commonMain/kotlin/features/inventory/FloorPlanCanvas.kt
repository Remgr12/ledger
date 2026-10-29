package org.erp.ledger.features.inventory

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
import uniffi.erp_ffi.LocationDto

/**
 * 2D Interactive Warehouse Floor Plan Canvas.
 * Renders spatial zones, aisles, and storage racks with pinpoint active location radar highlighting.
 */
@Composable
fun FloorPlanCanvas(
    locations: List<LocationDto>,
    activeLocation: LocationDto?,
    onLocationTapped: (LocationDto) -> Unit = {},
    modifier: Modifier = Modifier
) {
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
                "SPATIAL FLOOR PLAN (2D RADAR)",
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = Color(0xFF94A3B8)
            )
            activeLocation?.let {
                Text(
                    "TARGET: ${it.name} (${it.mapX?.toInt() ?: 0}, ${it.mapY?.toInt() ?: 0})",
                    fontSize = 10.sp,
                    fontWeight = FontWeight.Bold,
                    color = Color(0xFF34D399)
                )
            }
        }

        Canvas(
            modifier = Modifier
                .fillMaxWidth()
                .height(180.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(Color(0xFF020617))
                .pointerInput(locations) {
                    detectTapGestures { offset ->
                        // Find closest location within 30px radius
                        val tapped = locations.minByOrNull { loc ->
                            val lx = loc.mapX?.toFloat() ?: -100f
                            val ly = loc.mapY?.toFloat() ?: -100f
                            val dx = offset.x - lx
                            val dy = offset.y - ly
                            dx * dx + dy * dy
                        }
                        if (tapped != null) {
                            val lx = tapped.mapX?.toFloat() ?: -100f
                            val ly = tapped.mapY?.toFloat() ?: -100f
                            val dist = kotlin.math.sqrt((offset.x - lx) * (offset.x - lx) + (offset.y - ly) * (offset.y - ly))
                            if (dist < 40f) {
                                onLocationTapped(tapped)
                            }
                        }
                    }
                }
        ) {
            // 1. Draw subtle background coordinate grid
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

            // 2. Draw static warehouse zones (Receiving Bay, Aisle A, Aisle B, Packing)
            drawRect(
                color = Color(0xFF1E3A8A).copy(alpha = 0.3f),
                topLeft = Offset(20f, 20f),
                size = Size(80f, 140f),
                style = Stroke(width = 2f)
            )

            drawRect(
                color = Color(0xFF065F46).copy(alpha = 0.3f),
                topLeft = Offset(130f, 20f),
                size = Size(200f, 140f),
                style = Stroke(width = 2f)
            )

            // 3. Draw Rack & Bin nodes
            for (loc in locations) {
                val x = loc.mapX?.toFloat() ?: continue
                val y = loc.mapY?.toFloat() ?: continue
                val isTarget = loc.id == activeLocation?.id

                if (isTarget) {
                    // Pulsing radar target rings
                    drawCircle(
                        color = Color(0xFF10B981).copy(alpha = 0.2f),
                        radius = 28f,
                        center = Offset(x, y)
                    )
                    drawCircle(
                        color = Color(0xFF10B981).copy(alpha = 0.4f),
                        radius = 18f,
                        center = Offset(x, y),
                        style = Stroke(width = 2f)
                    )
                    drawCircle(
                        color = Color(0xFF34D399),
                        radius = 8f,
                        center = Offset(x, y)
                    )
                } else {
                    // Standard storage node
                    val nodeColor = when (loc.locationType) {
                        "Warehouse" -> Color(0xFF64748B)
                        "Room" -> Color(0xFF3B82F6)
                        "Rack" -> Color(0xFFF59E0B)
                        else -> Color(0xFF10B981)
                    }
                    drawCircle(
                        color = nodeColor.copy(alpha = 0.7f),
                        radius = 5f,
                        center = Offset(x, y)
                    )
                }
            }
        }
    }
}
