package org.erp.ledger.scanner

import java.awt.KeyEventDispatcher
import java.awt.KeyboardFocusManager
import java.awt.event.KeyEvent

/**
 * Desktop Barcode Scanner Handler.
 * Intercepts rapid sequential keystrokes from USB / Bluetooth HID barcode scanners (keyboard wedges)
 * that end with an Enter key (VK_ENTER).
 */
class DesktopBarcodeScanner(
    private val onBarcodeCaptured: (String) -> Unit
) {
    private val buffer = StringBuilder()
    private var lastKeyTime = 0L
    private val maxInterKeyDelayMs = 50L // Hardware barcode scanners type characters with < 30ms latency

    private val dispatcher = KeyEventDispatcher { e ->
        if (e.id == KeyEvent.KEY_TYPED) {
            val now = System.currentTimeMillis()
            if (now - lastKeyTime > maxInterKeyDelayMs && buffer.isNotEmpty()) {
                buffer.clear()
            }
            lastKeyTime = now

            if (e.keyChar == '\n' || e.keyChar == '\r') {
                if (buffer.length >= 3) {
                    val barcode = buffer.toString().trim()
                    onBarcodeCaptured(barcode)
                    buffer.clear()
                    return@KeyEventDispatcher true
                }
                buffer.clear()
            } else if (!e.keyChar.isISOControl()) {
                buffer.append(e.keyChar)
            }
        }
        false
    }

    fun startListening() {
        KeyboardFocusManager.getCurrentKeyboardFocusManager().addKeyEventDispatcher(dispatcher)
    }

    fun stopListening() {
        KeyboardFocusManager.getCurrentKeyboardFocusManager().removeKeyEventDispatcher(dispatcher)
    }
}
