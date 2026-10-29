package org.erp.ledger

import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import org.erp.ledger.core.ErpBridge

fun main() = application {
    val windowState = rememberWindowState(width = 1200.dp, height = 800.dp)
    val activeLoc by ErpBridge.activeContextLocation.collectAsState()

    val windowTitle = if (activeLoc != null) {
        "Ledger ERP — Active Target: [${activeLoc?.code}] ${activeLoc?.name}"
    } else {
        "Ledger ERP — Local-First Modular System"
    }

    Window(
        onCloseRequest = ::exitApplication,
        state = windowState,
        title = windowTitle,
        onKeyEvent = { event ->
            if (event.type == KeyEventType.KeyDown) {
                val hasModifier = event.isCtrlPressed || event.isMetaPressed
                when {
                    hasModifier && event.key == Key.F -> {
                        ErpBridge.navigateTo("locator")
                        true
                    }
                    hasModifier && event.key == Key.S -> {
                        ErpBridge.navigateTo("scan_hud")
                        true
                    }
                    hasModifier && event.key == Key.T -> {
                        ErpBridge.navigateTo("tree")
                        true
                    }
                    hasModifier && event.key == Key.P -> {
                        ErpBridge.navigateTo("labels")
                        true
                    }
                    hasModifier && event.key == Key.I -> {
                        ErpBridge.navigateTo("invoicing")
                        true
                    }
                    hasModifier && event.key == Key.A -> {
                        ErpBridge.navigateTo("accounting")
                        true
                    }
                    hasModifier && event.key == Key.B -> {
                        ErpBridge.navigateTo("purchasing")
                        true
                    }
                    hasModifier && event.key == Key.O -> {
                        ErpBridge.navigateTo("manufacturing")
                        true
                    }
                    hasModifier && event.key == Key.J -> {
                        ErpBridge.navigateTo("projects")
                        true
                    }
                    hasModifier && event.key == Key.M -> {
                        ErpBridge.navigateTo("settings")
                        true
                    }
                    event.key == Key.Escape -> {
                        ErpBridge.clearActiveContextLocation()
                        true
                    }
                    else -> false
                }
            } else {
                false
            }
        }
    ) {
        App()
    }
}
