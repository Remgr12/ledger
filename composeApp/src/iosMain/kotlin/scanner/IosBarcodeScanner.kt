package org.erp.ledger.scanner

import platform.AVFoundation.*
import platform.darwin.NSObject
import platform.darwin.dispatch_queue_create

/**
 * iOS AVFoundation QR & Barcode capture session and metadata output handler.
 * Configures AVCaptureMetadataOutput with AVMetadataObjectTypeQRCode,
 * AVMetadataObjectTypeEAN13Code, and AVMetadataObjectTypeCode128Code,
 * and streams recognized payloads to the Rapid Scan & Move HUD.
 */
class IosBarcodeScanner(
    private val onBarcodeScanned: (String) -> Unit
) {
    private var captureSession: AVCaptureSession? = null
    private var lastScannedPayload: String? = null
    private var lastScanTime = 0.0

    private val metadataDelegate = object : NSObject(), AVCaptureMetadataOutputObjectsDelegateProtocol {
        override fun captureOutput(
            output: AVCaptureOutput,
            didOutputMetadataObjects: List<*>,
            fromConnection: AVCaptureConnection
        ) {
            val firstObject = didOutputMetadataObjects.firstOrNull() as? AVMetadataMachineReadableCodeObject
            val payload = firstObject?.stringValue
            if (!payload.isNullOrBlank()) {
                val currentTime = platform.Foundation.NSDate().timeIntervalSince1970
                if (payload != lastScannedPayload || (currentTime - lastScanTime) > 0.5) {
                    lastScannedPayload = payload
                    lastScanTime = currentTime
                    onBarcodeScanned(payload)
                }
            }
        }
    }

    fun startScanning() {
        val session = AVCaptureSession()
        captureSession = session

        val device = AVCaptureDevice.defaultDeviceWithMediaType(AVMediaTypeVideo) ?: return
        val input = AVCaptureDeviceInput.deviceInputWithDevice(device, null) ?: return

        if (session.canAddInput(input)) {
            session.addInput(input)
        }

        val metadataOutput = AVCaptureMetadataOutput()
        if (session.canAddOutput(metadataOutput)) {
            session.addOutput(metadataOutput)
            val queue = dispatch_queue_create("org.erp.ledger.barcodeQueue", null)
            metadataOutput.setMetadataObjectsDelegate(metadataDelegate, queue)
            metadataOutput.setMetadataObjectTypes(
                listOf(
                    AVMetadataObjectTypeQRCode,
                    AVMetadataObjectTypeEAN13Code,
                    AVMetadataObjectTypeCode128Code,
                    AVMetadataObjectTypeDataMatrixCode
                )
            )
        }

        session.startRunning()
    }

    fun stopScanning() {
        captureSession?.stopRunning()
        captureSession = null
    }

    fun toggleTorch(enable: Boolean) {
        val device = AVCaptureDevice.defaultDeviceWithMediaType(AVMediaTypeVideo) ?: return
        if (device.hasTorch) {
            device.lockForConfiguration(null)
            device.torchMode = if (enable) AVCaptureTorchModeOn else AVCaptureTorchModeOff
            device.unlockForConfiguration()
        }
    }

    companion object {
        fun getSupportedMetadataTypes(): List<String> {
            return listOf(
                "AVMetadataObjectTypeQRCode",
                "AVMetadataObjectTypeEAN13Code",
                "AVMetadataObjectTypeCode128Code",
                "AVMetadataObjectTypeDataMatrixCode"
            )
        }
    }
}
