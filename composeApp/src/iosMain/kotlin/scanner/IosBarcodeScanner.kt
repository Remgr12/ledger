package org.erp.ledger.scanner

/**
 * iOS AVFoundation QR & Barcode capture metadata configuration.
 */
object IosBarcodeScanner {
    fun getSupportedMetadataTypes(): List<String> {
        return listOf(
            "AVMetadataObjectTypeQRCode",
            "AVMetadataObjectTypeEAN13Code",
            "AVMetadataObjectTypeCode128Code"
        )
    }
}
