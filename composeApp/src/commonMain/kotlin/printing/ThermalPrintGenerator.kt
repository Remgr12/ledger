package org.erp.ledger.printing

/**
 * Generator for thermal printer protocols:
 * - ESC/POS binary command stream with QR barcode commands
 * - TSPL / TSPL2 plain-text command sequence (TSC, Godex, Brother in TSPL mode)
 * - ZPL II plain-text commands (Zebra)
 * - Crisp SVG vector label markup
 */
object ThermalPrintGenerator {

    /**
     * Generates a raw ESC/POS byte sequence for thermal receipt and label printers.
     * Uses Standard Epson ESC/POS GS ( k 2D QR Code commands.
     */
    fun generateEscPos(
        sku: String,
        itemName: String,
        locationCode: String,
        copies: Int = 1
    ): ByteArray {
        val payload = "erp://item/$sku"
        val payloadBytes = payload.encodeToByteArray()
        val stream = mutableListOf<Byte>()

        for (c in 0 until copies) {
            // 1. Initialize printer: ESC @ (0x1B, 0x40)
            stream.add(0x1B.toByte())
            stream.add(0x40.toByte())

            // 2. Alignment: Center (ESC a 1)
            stream.add(0x1B.toByte())
            stream.add(0x61.toByte())
            stream.add(0x01.toByte())

            // 3. QR Code - Function 165: Select Model (Model 2)
            // GS ( k 4 0 49 65 50 0
            stream.addAll(listOf(0x1D, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00).map { it.toByte() })

            // 4. QR Code - Function 167: Set module size (6 dots per module)
            // GS ( k 3 0 49 67 6
            stream.addAll(listOf(0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, 0x06).map { it.toByte() })

            // 5. QR Code - Function 169: Set error correction (Level L = 48)
            // GS ( k 3 0 49 69 48
            stream.addAll(listOf(0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x45, 0x30).map { it.toByte() })

            // 6. QR Code - Function 180: Store QR data in symbol storage area
            // GS ( k pL pH 49 80 48 <data...>
            val len = payloadBytes.size + 3
            val pL = (len and 0xFF).toByte()
            val pH = ((len shr 8) and 0xFF).toByte()
            stream.add(0x1D.toByte())
            stream.add(0x28.toByte())
            stream.add(0x6B.toByte())
            stream.add(pL)
            stream.add(pH)
            stream.add(0x31.toByte())
            stream.add(0x50.toByte())
            stream.add(0x30.toByte())
            payloadBytes.forEach { stream.add(it) }

            // 7. QR Code - Function 181: Print QR code symbol
            // GS ( k 3 0 49 81 48
            stream.addAll(listOf(0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x81.toInt().toByte(), 0x30).map { it.toByte() })

            // 8. Line feed
            stream.add(0x0A.toByte())

            // 9. Bold Text: ESC E 1
            stream.add(0x1B.toByte())
            stream.add(0x45.toByte())
            stream.add(0x01.toByte())

            // SKU in bold
            val skuBytes = "$sku\n".encodeToByteArray()
            skuBytes.forEach { stream.add(it) }

            // Bold Text: ESC E 0 (Off)
            stream.add(0x1B.toByte())
            stream.add(0x45.toByte())
            stream.add(0x00.toByte())

            // Item Name & Location
            val descBytes = "$itemName\nBin: $locationCode\n$payload\n".encodeToByteArray()
            descBytes.forEach { stream.add(it) }

            // 10. Feed 3 lines: ESC d 3
            stream.add(0x1B.toByte())
            stream.add(0x64.toByte())
            stream.add(0x03.toByte())

            // 11. Partial cut: GS V 66 0
            stream.addAll(listOf(0x1D, 0x56, 0x42, 0x00).map { it.toByte() })
        }

        return stream.toByteArray()
    }

    /**
     * Generates TSPL command sequence for TSC / Brother / Godex thermal printers.
     */
    fun generateTspl(
        sku: String,
        itemName: String,
        locationCode: String,
        copies: Int = 1
    ): String {
        return buildString {
            appendLine("SIZE 62 mm, 29 mm")
            appendLine("GAP 3 mm, 0 mm")
            appendLine("DIRECTION 1")
            appendLine("CLS")
            appendLine("QRCODE 25,20,L,5,A,0,\"erp://item/$sku\"")
            appendLine("TEXT 180,20,\"3\",0,1,1,\"$sku\"")
            appendLine("TEXT 180,55,\"2\",0,1,1,\"$itemName\"")
            appendLine("TEXT 180,85,\"2\",0,1,1,\"Bin: $locationCode\"")
            appendLine("TEXT 180,115,\"1\",0,1,1,\"erp://item/$sku\"")
            appendLine("PRINT 1,$copies")
        }
    }

    /**
     * Generates ZPL II command sequence for Zebra direct thermal printers.
     */
    fun generateZpl(
        sku: String,
        itemName: String,
        locationCode: String,
        copies: Int = 1
    ): String {
        return buildString {
            appendLine("^XA")
            appendLine("^PW496")
            appendLine("^FO30,30^BQN,2,5^FDLA,erp://item/$sku^FS")
            appendLine("^FO180,30^A0N,28,28^FD$sku^FS")
            appendLine("^FO180,65^A0N,20,20^FD$itemName^FS")
            appendLine("^FO180,95^A0N,20,20^FDBin: $locationCode^FS")
            appendLine("^PQ$copies")
            appendLine("^XZ")
        }
    }

    /**
     * Generates clean SVG markup representation suitable for vector rendering and Avery sheet layouts.
     */
    fun generateSvgLabel(
        sku: String,
        itemName: String,
        locationCode: String,
        format: LabelFormat
    ): String {
        val width = (format.widthMm * 3.7795).toInt()
        val height = (format.heightMm * 3.7795).toInt()

        return buildString {
            appendLine("""<svg xmlns="http://www.w3.org/2000/svg" width="${width}px" height="${height}px" viewBox="0 0 $width $height">""")
            appendLine("""  <rect width="100%" height="100%" fill="#ffffff" stroke="#cbd5e1" stroke-width="1" rx="4"/>""")
            appendLine("""  <rect x="12" y="12" width="64" height="64" fill="#0f172a" rx="4"/>""")
            appendLine("""  <rect x="20" y="20" width="16" height="16" fill="#ffffff"/>""")
            appendLine("""  <rect x="24" y="24" width="8" height="8" fill="#0f172a"/>""")
            appendLine("""  <rect x="52" y="20" width="16" height="16" fill="#ffffff"/>""")
            appendLine("""  <rect x="56" y="24" width="8" height="8" fill="#0f172a"/>""")
            appendLine("""  <rect x="20" y="52" width="16" height="16" fill="#ffffff"/>""")
            appendLine("""  <rect x="24" y="56" width="8" height="8" fill="#0f172a"/>""")
            appendLine("""  <text x="88" y="32" font-family="system-ui, sans-serif" font-size="14" font-weight="bold" fill="#0f172a">$sku</text>""")
            appendLine("""  <text x="88" y="50" font-family="system-ui, sans-serif" font-size="11" fill="#475569">$itemName</text>""")
            appendLine("""  <text x="88" y="66" font-family="system-ui, sans-serif" font-size="10" font-weight="bold" fill="#64748b">Bin: $locationCode</text>""")
            appendLine("""  <text x="88" y="80" font-family="monospace" font-size="9" fill="#4338ca">erp://item/$sku</text>""")
            appendLine("""</svg>""")
        }
    }
}
