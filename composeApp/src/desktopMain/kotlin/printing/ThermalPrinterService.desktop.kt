package org.erp.ledger.printing

import java.awt.Color
import java.awt.Font
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.GraphicsEnvironment
import java.awt.print.PageFormat
import java.awt.print.Paper
import java.awt.print.Printable
import java.awt.print.PrinterJob
import java.io.File
import java.io.FileOutputStream
import javax.print.DocFlavor
import javax.print.PrintServiceLookup
import javax.print.SimpleDoc
import javax.print.attribute.HashPrintRequestAttributeSet
import javax.print.attribute.standard.Copies

/**
 * Desktop Thermal & System Printing Implementation.
 * Supports:
 * 1. Native OS Print Dialog with crisp vector Graphics2D label rendering.
 * 2. Raw ESC/POS / TSPL direct byte streaming to USB/serial devices or CUPS/WinSpool raw queues.
 */
actual object ThermalPrinterService {

    actual fun printViaSystemDialog(
        sku: String,
        itemName: String,
        locationCode: String,
        format: LabelFormat,
        copies: Int
    ): PrintResult {
        return try {
            if (GraphicsEnvironment.isHeadless()) {
                // In headless Linux or CI testing environment: write preview/spool file
                val spoolDir = File("build/spool").apply { mkdirs() }
                val spoolFile = File(spoolDir, "label_${sku}_${System.currentTimeMillis()}.svg")
                val svg = ThermalPrintGenerator.generateSvgLabel(sku, itemName, locationCode, format)
                spoolFile.writeText(svg)
                return PrintResult(
                    success = true,
                    message = "Headless environment: rendered vector label to ${spoolFile.path}",
                    bytesSent = svg.toByteArray().size
                )
            }

            val job = PrinterJob.getPrinterJob()
            val pageFormat = job.defaultPage()
            val paper = pageFormat.paper

            // Convert mm to points (72 points = 1 inch = 25.4mm -> 1mm = 72/25.4 ≈ 2.8346 pt)
            val ptWidth = format.widthMm * (72.0 / 25.4)
            val ptHeight = format.heightMm * (72.0 / 25.4)
            paper.setSize(ptWidth, ptHeight)
            paper.setImageableArea(0.0, 0.0, ptWidth, ptHeight)
            pageFormat.paper = paper

            job.setPrintable(Printable { graphics: Graphics, pf: PageFormat, pageIndex: Int ->
                if (pageIndex > 0) return@Printable Printable.NO_SUCH_PAGE
                val g2d = graphics as Graphics2D

                // Background
                g2d.color = Color.WHITE
                g2d.fillRect(0, 0, pf.imageableWidth.toInt(), pf.imageableHeight.toInt())

                // Border
                g2d.color = Color.LIGHT_GRAY
                g2d.drawRect(2, 2, pf.imageableWidth.toInt() - 4, pf.imageableHeight.toInt() - 4)

                // Simulated QR block
                val qrSize = (pf.imageableHeight * 0.7).toInt()
                g2d.color = Color.BLACK
                g2d.fillRect(8, 8, qrSize, qrSize)
                g2d.color = Color.WHITE
                g2d.fillRect(14, 14, qrSize / 4, qrSize / 4)
                g2d.fillRect(qrSize - 2 - qrSize / 4, 14, qrSize / 4, qrSize / 4)
                g2d.fillRect(14, qrSize - 2 - qrSize / 4, qrSize / 4, qrSize / 4)

                // Typography
                val textX = 16 + qrSize
                g2d.color = Color.BLACK
                g2d.font = Font("SansSerif", Font.BOLD, 10)
                g2d.drawString(sku, textX, 22)

                g2d.font = Font("SansSerif", Font.PLAIN, 8)
                g2d.drawString(itemName.take(24), textX, 36)

                g2d.color = Color.DARK_GRAY
                g2d.font = Font("SansSerif", Font.BOLD, 8)
                g2d.drawString("Bin: $locationCode", textX, 48)

                g2d.color = Color.BLUE
                g2d.font = Font("Monospaced", Font.PLAIN, 7)
                g2d.drawString("erp://item/$sku", textX, 60)

                Printable.PAGE_EXISTS
            }, pageFormat)

            job.setCopies(copies)
            if (job.printDialog()) {
                job.print()
                PrintResult(true, "Successfully submitted print job to ${job.printService.name}")
            } else {
                PrintResult(false, "Print canceled by user")
            }
        } catch (e: Exception) {
            PrintResult(false, "System print failed: ${e.message}")
        }
    }

    actual fun sendRawToPrinter(
        rawBytes: ByteArray,
        targetPortOrDevice: String?
    ): PrintResult {
        return try {
            val destination = targetPortOrDevice?.trim()

            // 1. If destination is a device path or file path (e.g. /dev/usb/lp0, COM1, or file)
            if (!destination.isNullOrBlank() && (destination.startsWith("/") || destination.startsWith("COM") || destination.contains("."))) {
                val file = File(destination)
                FileOutputStream(file).use { fos ->
                    fos.write(rawBytes)
                    fos.flush()
                }
                return PrintResult(true, "Sent ${rawBytes.size} bytes directly to raw device $destination", rawBytes.size)
            }

            // 2. Direct javax.print lookup for RAW/AUTOSENSE
            val services = PrintServiceLookup.lookupPrintServices(DocFlavor.BYTE_ARRAY.AUTOSENSE, null)
            val targetService = if (!destination.isNullOrBlank()) {
                services.firstOrNull { it.name.contains(destination, ignoreCase = true) }
            } else {
                PrintServiceLookup.lookupDefaultPrintService() ?: services.firstOrNull()
            }

            if (targetService != null) {
                val docPrintJob = targetService.createPrintJob()
                val doc = SimpleDoc(rawBytes, DocFlavor.BYTE_ARRAY.AUTOSENSE, null)
                val attrSet = HashPrintRequestAttributeSet()
                attrSet.add(Copies(1))
                docPrintJob.print(doc, attrSet)
                PrintResult(true, "Sent ${rawBytes.size} bytes raw ESC/POS to printer ${targetService.name}", rawBytes.size)
            } else {
                // Fallback: write spool file for raw testing
                val spoolDir = File("build/spool").apply { mkdirs() }
                val spoolFile = File(spoolDir, "raw_thermal_${System.currentTimeMillis()}.bin")
                spoolFile.writeBytes(rawBytes)
                PrintResult(true, "No physical thermal printer attached. Raw stream (${rawBytes.size} bytes) saved to ${spoolFile.path}", rawBytes.size)
            }
        } catch (e: Exception) {
            PrintResult(false, "Direct ESC/POS printing error: ${e.message}")
        }
    }
}
