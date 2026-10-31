package org.erp.ledger.scanner

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.ScanResolutionDto

/**
 * Cross-platform QR & Barcode Scanner Controller.
 * Handles Camera scanning (Android CameraX / iOS AVFoundation),
 * USB Barcode Scanner HID keyboard wedges (Desktop), and rapid action callbacks.
 */
class QrScannerController {
    private val _isTorchEnabled = MutableStateFlow(false)
    val isTorchEnabled: StateFlow<Boolean> = _isTorchEnabled.asStateFlow()

    private val _isCameraActive = MutableStateFlow(true)
    val isCameraActive: StateFlow<Boolean> = _isCameraActive.asStateFlow()

    private val _scanEvents = MutableSharedFlow<ScanResolutionDto>(replay = 0, extraBufferCapacity = 16)
    val scanEvents: SharedFlow<ScanResolutionDto> = _scanEvents.asSharedFlow()

    fun toggleTorch() {
        _isTorchEnabled.value = !_isTorchEnabled.value
    }

    fun setCameraActive(active: Boolean) {
        _isCameraActive.value = active
    }

    /**
     * Invoked when a barcode or QR code is detected by CameraX, AVFoundation,
     * or typed via a USB HID keyboard barcode scanner.
     */
    fun onCodeScanned(rawPayload: String): ScanResolutionDto {
        val resolution = ErpBridge.resolveQrScan(rawPayload)
        _scanEvents.tryEmit(resolution)
        return resolution
    }
}
