package org.erp.ledger

import org.erp.ledger.core.ErpBridge
import uniffi.erp_ffi.BomComponentDto
import kotlin.test.*

class V003FeaturesTest {

    @BeforeTest
    fun setup() {
        ErpBridge.initialize(":memory:")
        val engine = ErpBridge.getNativeEngine()
        if (engine != null) {
            for (m in listOf("mod-accounting", "mod-invoicing", "mod-purchasing", "mod-manufacturing", "mod-projects")) {
                engine.setModuleEnabled(m, true)
            }
        }
    }

    @Test
    fun testVectorPdfGeneration() {
        val invoicePdf = ErpBridge.generateInvoicePdf("inv-1")
        assertNotNull(invoicePdf)
        assertTrue(invoicePdf.isNotEmpty())
        val pdfStr = invoicePdf.decodeToString()
        assertTrue(pdfStr.startsWith("%PDF-1.4"))

        val poPdf = ErpBridge.generatePurchaseOrderPdf("po-1")
        assertNotNull(poPdf)
        assertTrue(poPdf.isNotEmpty())
        assertTrue(poPdf.decodeToString().startsWith("%PDF-1.4"))
    }

    @Test
    fun testPeerToPeerLanSync() {
        val status = ErpBridge.getSyncStatus()
        assertNotNull(status)
        assertTrue(status.connectedPeersCount >= 0u)
        assertNotNull(status.localHlc)

        val synced = ErpBridge.triggerLanSync()
        assertNotNull(synced)
        assertEquals(0u, synced.unsyncedEventsCount)
        assertEquals("Converged", synced.convergenceState)
    }

    @Test
    fun testLowStockAlertsAndReorder() {
        val engine = ErpBridge.getNativeEngine()
        if (engine != null) {
            val item = engine.createItem(
                "TEST-BEARING-608",
                "Deep Groove Ball Bearing 608",
                "Critical spare part",
                "pcs",
                false,
                false,
                "50",
                null
            )
            val alerts = ErpBridge.getLowStockAlerts()
            assertTrue(alerts.any { it.sku == "TEST-BEARING-608" })

            val reorderPo = ErpBridge.generateReorderPo(item.id)
            assertNotNull(reorderPo)
            assertTrue(reorderPo.poNumber.startsWith("PO-REORDER-"))
            assertTrue(reorderPo.lines.isNotEmpty())
        } else {
            val alerts = ErpBridge.getLowStockAlerts()
            assertNotNull(alerts)
            val reorderPo = ErpBridge.generateReorderPo("item-1")
            assertNotNull(reorderPo)
        }
    }

    @Test
    fun testWorkOrderKanbanAndGenealogy() {
        val engine = ErpBridge.getNativeEngine()
        if (engine != null) {
            val wh = engine.createLocation("WH-TEST", "Main Bay", null, "Warehouse", 0.0, 0.0)
            val raw = engine.createItem("RAW-BEARING", "Ball Bearing Raw", null, "pcs", false, true, "10", null)
            val fg = engine.createItem("FG-MOTOR", "Drive Motor Unit", null, "pcs", false, true, "5", null)

            engine.moveStock(raw.id, null, wh.id, "50", "Receive", "LOT-RAW-001", "Initial stock")
            engine.createOrUpdateLot("LOT-RAW-001", raw.id, "2030-12-31", "released", "NSK-BATCH-99")

            val bom = engine.createBom(
                "BOM-MOTOR-01",
                fg.id,
                "Motor Assembly Recipe",
                "1",
                listOf(BomComponentDto("", raw.id, "2", "0.0", "Bearings")),
                "Press fit bearing into housing"
            )

            val wo = engine.createWorkOrder("WO-MOTOR-001", bom.id, "2", wh.id, wh.id)
            assertEquals("Draft", wo.status)

            val updatedWo = ErpBridge.updateWorkOrderStatus(wo.id, "In Progress")
            assertNotNull(updatedWo)
            assertEquals("In Progress", updatedWo.status)

            val qcWo = ErpBridge.updateWorkOrderStatus(wo.id, "Quality Check")
            assertNotNull(qcWo)
            assertEquals("Quality Check", qcWo.status)

            val doneWo = ErpBridge.updateWorkOrderStatus(wo.id, "Completed")
            assertNotNull(doneWo)
            assertEquals("Completed", doneWo.status)

            val genealogy = ErpBridge.getWorkOrderGenealogy(wo.id)
            assertNotNull(genealogy)
            assertTrue(genealogy.finishedLotNumber.startsWith("LOT-MFG-"))
            assertTrue(genealogy.components.isNotEmpty())
            assertEquals("RAW-BEARING", genealogy.components.first().componentSku)
        } else {
            val woList = ErpBridge.listWorkOrders()
            assertTrue(woList.isNotEmpty())
        }
    }

    @Test
    fun testProjectTaskKanbanAndUnbilledSummary() {
        val engine = ErpBridge.getNativeEngine()
        if (engine != null) {
            val prj = engine.createProject(
                "Automation Cell",
                "PRJ-AUTO-TEST",
                null,
                "120.00",
                "40"
            )

            val task1 = engine.createTask(prj.id, "Install Optical Sensor", "High", "8")
            val task2 = engine.createTask(prj.id, "Commission PLC", "Urgent", "12")

            val tasks = ErpBridge.listProjectTasks(prj.id)
            assertEquals(2, tasks.size)

            val movedTask = ErpBridge.updateProjectTaskStatus(task1.id, "In Progress")
            assertNotNull(movedTask)
            assertEquals("In Progress", movedTask.status)

            engine.recordTimesheet(
                prj.id,
                task2.id,
                "lead_engineer",
                "2026-10-31",
                "5.0",
                null,
                true,
                "Calibrated analog sensors"
            )

            val unbilled = ErpBridge.getUnbilledHoursSummary(prj.id)
            assertNotNull(unbilled)
            assertEquals("5.0", unbilled.totalUnbilledHours)
            assertEquals(1u, unbilled.unbilledEntriesCount)
        } else {
            val tasks = ErpBridge.listProjectTasks(null)
            assertTrue(tasks.isNotEmpty())
        }
    }
}
