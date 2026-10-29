package org.erp.ledger.scanner

/**
 * Android CameraX + MLKit Barcode Scanning Configuration.
 * Configures ImageAnalysis with BarcodeScannerOptions for QR codes, EAN-13, and Code 128.
 */
object AndroidBarcodeScanner {
    const val SCANNER_RESOLUTION_WIDTH = 1280
    const val SCANNER_RESOLUTION_HEIGHT = 720

    fun getSupportedBarcodeFormats(): List<String> {
        return listOf(
            "FORMAT_QR_CODE",
            "FORMAT_EAN_13",
            "FORMAT_CODE_128",
            "FORMAT_DATA_MATRIX"
        )
    }
}
