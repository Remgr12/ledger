package org.erp.ledger

import java.io.File
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import org.erp.ledger.core.ErpBridge

class BackupAndPluginTest {

    @Test
    fun testDatabaseBackupAndWalFlush() {
        val testDb = "build/test_backup_source.db"
        val backupDest = "build/test_backup_dest.db"
        File(testDb).delete()
        File(backupDest).delete()

        ErpBridge.initialize(testDb)

        val success = ErpBridge.backupDatabase(backupDest)
        assertTrue(success, "backupDatabase should return true")
        assertTrue(File(backupDest).exists(), "Backup destination file should exist")
        assertTrue(File(backupDest).length() > 0, "Backup file should be non-empty")
    }

    @Test
    fun testDataExportCsvAndJson() {
        ErpBridge.initialize(":memory:")

        val itemsCsv = ErpBridge.exportDataCsv("items")
        assertTrue(itemsCsv.startsWith("id,sku,name"), "Items CSV should have header")

        val contactsCsv = ErpBridge.exportDataCsv("contacts")
        assertTrue(contactsCsv.startsWith("id,name,tax_id"), "Contacts CSV should have header")

        val itemsJson = ErpBridge.exportDataJson("items")
        assertTrue(itemsJson.startsWith("["), "Items JSON should be a JSON array")

        val contactsJson = ErpBridge.exportDataJson("contacts")
        assertTrue(contactsJson.startsWith("["), "Contacts JSON should be a JSON array")
    }

    @Test
    fun testDataImportCsv() {
        ErpBridge.initialize(":memory:")

        val sampleCsv = """
            sku,name,unit,description,min_stock
            IMPORT-TEST-1,Imported Item 1,pcs,Test Desc,10
            IMPORT-TEST-2,Imported Item 2,kg,Test Desc 2,25
        """.trimIndent()

        val importedCount = ErpBridge.importDataCsv("items", sampleCsv)
        assertTrue(importedCount >= 2, "Should import at least 2 items")

        val exportCsvAfter = ErpBridge.exportDataCsv("items")
        assertTrue(exportCsvAfter.contains("IMPORT-TEST-1"), "Exported CSV should contain imported item")
    }

    @Test
    fun testWasmPluginInstallationAndUiNodes() {
        ErpBridge.initialize(":memory:")

        val wasmPath = "../plugins/sample_supplier_plugin.wasm"
        val altPath = "plugins/sample_supplier_plugin.wasm"
        val resolvedPath = if (File(wasmPath).exists()) wasmPath else altPath

        if (File(resolvedPath).exists()) {
            val manifest = """
            {
                "id": "sample-supplier-plugin",
                "name": "Acme Supplier GS1 Plugin",
                "version": "1.0.0",
                "description": "Supplier decoder and QA badges",
                "permissions": ["read:inventory", "ui:custom_field", "hook:on_qr_scanned"]
            }
            """.trimIndent()

            val installed = ErpBridge.installWasmPluginFile(resolvedPath, manifest)
            assertTrue(installed, "installWasmPluginFile should succeed")

            val plugins = ErpBridge.listInstalledPlugins()
            assertTrue(plugins.any { it.id == "sample-supplier-plugin" }, "Installed plugins should contain sample-supplier-plugin")

            val uiNodes = ErpBridge.getPluginUiNodes("inventory_detail")
            assertTrue(uiNodes.isNotEmpty(), "UI nodes for inventory_detail should not be empty")

            val uninstalled = ErpBridge.uninstallPlugin("sample-supplier-plugin")
            assertTrue(uninstalled, "uninstallPlugin should succeed")
        }
    }
}
