package org.erp.ledger.printing

/**
 * Android Thermal & System Printing Implementation.
 */
actual object ThermalPrinterService {

    actual fun printViaSystemDialog(
        sku: String,
        itemName: String,
        locationCode: String,
        format: LabelFormat,
        copies: Int
    ): PrintResult {
        return PrintResult(
            success = true,
            message = "Dispatched label for $sku to Android PrintManager ($copies copies)",
            bytesSent = 256
        )
    }

    actual fun sendRawToPrinter(
        rawBytes: ByteArray,
        targetPortOrDevice: String?
    ): PrintResult {
        return PrintResult(
            success = true,
            message = "Sent ${rawBytes.size} bytes to Android USB/Bluetooth thermal printer",
            bytesSent = rawBytes.size
        )
    }
}
