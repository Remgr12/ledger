package org.erp.ledger

import kotlin.test.Test
import kotlin.test.assertTrue
import kotlin.test.assertEquals
import org.erp.ledger.printing.LabelFormat
import org.erp.ledger.printing.ThermalPrintGenerator
import org.erp.ledger.printing.ThermalPrinterService

class ThermalPrintingTest {

    @Test
    fun testEscPosGenerationContainsQrAndText() {
        val bytes = ThermalPrintGenerator.generateEscPos(
            sku = "SKU-TEST-100",
            itemName = "Test Bearing 608RS",
            locationCode = "RACK-1",
            copies = 1
        )
        assertTrue(bytes.isNotEmpty(), "ESC/POS byte stream should not be empty")

        // Should start with ESC @ (0x1B, 0x40)
        assertEquals(0x1B.toByte(), bytes[0])
        assertEquals(0x40.toByte(), bytes[1])

        // String conversion should contain SKU and location
        val asText = bytes.toString(Charsets.ISO_8859_1)
        assertTrue(asText.contains("SKU-TEST-100"))
        assertTrue(asText.contains("RACK-1"))
        assertTrue(asText.contains("erp://item/SKU-TEST-100"))
    }

    @Test
    fun testTsplAndZplGeneration() {
        val tspl = ThermalPrintGenerator.generateTspl(
            sku = "SKU-BOLT-M8",
            itemName = "Stainless Bolt",
            locationCode = "BIN-02",
            copies = 2
        )
        assertTrue(tspl.contains("SIZE 62 mm, 29 mm"))
        assertTrue(tspl.contains("QRCODE 25,20,L,5,A,0,\"erp://item/SKU-BOLT-M8\""))
        assertTrue(tspl.contains("PRINT 1,2"))

        val zpl = ThermalPrintGenerator.generateZpl(
            sku = "SKU-BOLT-M8",
            itemName = "Stainless Bolt",
            locationCode = "BIN-02",
            copies = 2
        )
        assertTrue(zpl.contains("^XA"))
        assertTrue(zpl.contains("^XZ"))
        assertTrue(zpl.contains("erp://item/SKU-BOLT-M8"))
    }

    @Test
    fun testThermalPrinterServiceDispatch() {
        val printResult = ThermalPrinterService.printViaSystemDialog(
            sku = "SKU-TEST",
            itemName = "Item",
            locationCode = "BIN-1",
            format = LabelFormat.BROTHER_62MM,
            copies = 1
        )
        assertTrue(printResult.success)

        val rawResult = ThermalPrinterService.sendRawToPrinter(
            rawBytes = "TEST-RAW-DATA".encodeToByteArray(),
            targetPortOrDevice = null
        )
        assertTrue(rawResult.success)
    }
}
