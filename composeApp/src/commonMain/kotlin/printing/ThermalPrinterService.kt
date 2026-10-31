package org.erp.ledger.printing

data class PrintResult(
    val success: Boolean,
    val message: String,
    val bytesSent: Int = 0
)

/**
 * Cross-platform thermal and document printer service.
 * Dispatches to OS print spooler (CUPS/WinSpool/AirPrint) or raw USB/Serial printer endpoints.
 */
expect object ThermalPrinterService {
    fun printViaSystemDialog(
        sku: String,
        itemName: String,
        locationCode: String,
        format: LabelFormat,
        copies: Int = 1
    ): PrintResult

    fun sendRawToPrinter(
        rawBytes: ByteArray,
        targetPortOrDevice: String? = null
    ): PrintResult
}
