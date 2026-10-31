package org.erp.ledger

import kotlin.test.Test
import kotlin.test.assertTrue
import kotlin.test.assertNotNull
import org.erp.ledger.core.ErpBridge

class NativeLoadTest {
    @Test
    fun testNativeEngineLoadsWithoutFallback() {
        ErpBridge.initialize(":memory:")
        val modules = ErpBridge.availableModules.value
        println("Loaded modules: ${modules.map { it.id }}")
        assertTrue(modules.isNotEmpty(), "Modules should not be empty")
        val storageLocations = ErpBridge.getStorageLocations()
        assertNotNull(storageLocations)
    }
}
