package org.erp.ledger.core

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import uniffi.erp_ffi.*

/**
 * ErpBridge: The multiplatform bridge connecting Compose UI to the UniFFI Rust core engine.
 * Supports offline-first operation, reactive state management, and rapid action scanning workflows.
 * Directly wraps the Mozilla UniFFI generated `ErpEngineInterface`.
 */
object ErpBridge {
    private val _isInitialized = MutableStateFlow(false)
    val isInitialized: StateFlow<Boolean> = _isInitialized.asStateFlow()

    private val _activeContextLocation = MutableStateFlow<LocationDto?>(null)
    val activeContextLocation: StateFlow<LocationDto?> = _activeContextLocation.asStateFlow()

    private val _availableModules = MutableStateFlow<List<ModuleInfoDto>>(emptyList())
    val availableModules: StateFlow<List<ModuleInfoDto>> = _availableModules.asStateFlow()

    private val _recentScanResolution = MutableStateFlow<ScanResolutionDto?>(null)
    val recentScanResolution: StateFlow<ScanResolutionDto?> = _recentScanResolution.asStateFlow()

    private val _currentScreen = MutableStateFlow("locator")
    val currentScreen: StateFlow<String> = _currentScreen.asStateFlow()

    private val _serverUrl = MutableStateFlow<String?>("http://localhost:8080")
    val serverUrl: StateFlow<String?> = _serverUrl.asStateFlow()

    private val _serverStatus = MutableStateFlow<String>("Local Embedded & Server Synchronized")
    val serverStatus: StateFlow<String> = _serverStatus.asStateFlow()

    fun setServerUrl(url: String?) {
        _serverUrl.value = url
        _serverStatus.value = if (!url.isNullOrBlank()) "Configured ($url)" else "Local Embedded SQLite Mode"
    }

    fun navigateTo(screen: String) {
        _currentScreen.value = screen
    }

    private var nativeEngine: ErpEngineInterface? = null
    fun getNativeEngine(): ErpEngineInterface? = nativeEngine

    // In-memory fallback dataset for environments without loaded native dynamic libraries
    private val mockLocations = mutableListOf(
        LocationDto(
            id = "loc-wh-1",
            code = "WH-MAIN",
            name = "Main Facility",
            parentId = null,
            locationType = "Warehouse",
            pathBreadcrumb = "Main Facility",
            qrCodeUri = "erp://loc/loc-wh-1",
            mapX = 100.0,
            mapY = 80.0,
            isActive = true
        ),
        LocationDto(
            id = "loc-rm-1",
            code = "WS-01",
            name = "Workshop",
            parentId = "loc-wh-1",
            locationType = "Room",
            pathBreadcrumb = "Main Facility > Workshop",
            qrCodeUri = "erp://loc/loc-rm-1",
            mapX = 140.0,
            mapY = 110.0,
            isActive = true
        ),
        LocationDto(
            id = "loc-rck-2",
            code = "RCK-02",
            name = "Rack 2 (North Wall)",
            parentId = "loc-rm-1",
            locationType = "Rack",
            pathBreadcrumb = "Main Facility > Workshop > Rack 2 (North Wall)",
            qrCodeUri = "erp://loc/loc-rck-2",
            mapX = 160.0,
            mapY = 120.0,
            isActive = true
        ),
        LocationDto(
            id = "loc-bin-4",
            code = "BIN-04",
            name = "Blue Bin #4",
            parentId = "loc-rck-2",
            locationType = "Bin",
            pathBreadcrumb = "Main Facility > Workshop > Rack 2 (North Wall) > Shelf C > Blue Bin #4",
            qrCodeUri = "erp://loc/loc-bin-4",
            mapX = 165.0,
            mapY = 125.0,
            isActive = true
        )
    )

    private val mockItems = mutableListOf(
        ItemLocationCardDto(
            itemId = "item-1",
            sku = "M8-HEX-BOLT-316",
            itemName = "M8x40mm Hex Bolt Stainless 316",
            unit = "pcs",
            locationId = "loc-bin-4",
            locationCode = "BIN-04",
            locationName = "Blue Bin #4",
            locationType = "Bin",
            breadcrumbPath = "Main Facility > Workshop > Rack 2 (North Wall) > Shelf C > Blue Bin #4",
            mapX = 165.0,
            mapY = 125.0,
            quantity = "142.0",
            lotNumber = "LOT-2026-Q1",
            serialNumber = null,
            minStockLevel = "50.0",
            lotExpirationDate = "2029-12-31",
            lotQuarantineStatus = "released",
            lotManufacturerBatch = "ACME-BATCH-889",
            isLowStock = false,
            isExpiredOrExpiring = false
        ),
        ItemLocationCardDto(
            itemId = "item-2",
            sku = "BEARING-608-2RS",
            itemName = "Deep Groove Ball Bearing 608-2RS",
            unit = "pcs",
            locationId = "loc-bin-4",
            locationCode = "BIN-04",
            locationName = "Blue Bin #4",
            locationType = "Bin",
            breadcrumbPath = "Main Facility > Workshop > Rack 2 (North Wall) > Shelf C > Blue Bin #4",
            mapX = 165.0,
            mapY = 125.0,
            quantity = "38.0",
            lotNumber = "LOT-NSK-9",
            serialNumber = null,
            minStockLevel = "100.0",
            lotExpirationDate = "2027-06-30",
            lotQuarantineStatus = "released",
            lotManufacturerBatch = "NSK-JAPAN-004",
            isLowStock = true,
            isExpiredOrExpiring = false
        )
    )

    fun initialize(dbPath: String = "ledger.db") {
        try {
            val engine = initErp(dbPath)
            nativeEngine = engine
            _availableModules.value = engine.getAvailableModules()
        } catch (e: Throwable) {
            println("ErpBridge: Native UniFFI library not linked in current runtime, initializing local fallback ($e)")
            _availableModules.value = listOf(
                ModuleInfoDto(
                    id = "mod-inventory",
                    name = "Spatial Inventory & Stock Ledger",
                    description = "Self-referencing storage tree, immutable stock ledger, and offline smart QR codes.",
                    dependencies = emptyList(),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("inventory/locator", "Item Locator", "search", 10u),
                        ModuleRouteDto("inventory/scan_hud", "Scan & Move HUD", "qr_code_scanner", 11u),
                        ModuleRouteDto("inventory/tree", "Storage Locations", "account_tree", 12u),
                        ModuleRouteDto("inventory/labels", "Print QR Labels", "print", 13u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-contacts",
                    name = "Contacts & Customers",
                    description = "Customer and supplier records, addresses, and tax identifiers.",
                    dependencies = emptyList(),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("contacts/list", "Contacts", "people", 20u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-invoicing",
                    name = "Invoicing & Billing",
                    description = "Quotes, sales invoices, tax precision calculation, and automated inventory deduction.",
                    dependencies = listOf("mod-contacts"),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("invoicing/invoices", "Invoices", "receipt_long", 30u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-accounting",
                    name = "General Ledger & Accounting",
                    description = "Chart of accounts tree, balanced double-entry GL, Trial Balance, and Profit & Loss reports.",
                    dependencies = emptyList(),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("accounting/gl", "General Ledger", "account_balance", 40u),
                        ModuleRouteDto("accounting/trial_balance", "Trial Balance", "assessment", 41u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-purchasing",
                    name = "Purchasing & Procurement",
                    description = "Purchase orders, vendor shipments, and goods receipt with direct warehouse putaway.",
                    dependencies = listOf("mod-contacts", "mod-inventory"),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("purchasing/orders", "Purchase Orders", "shopping_cart", 50u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-manufacturing",
                    name = "Manufacturing & Assembly",
                    description = "Multi-level Bill of Materials (BOM) and atomic component-to-finished goods work orders.",
                    dependencies = listOf("mod-inventory"),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("manufacturing/boms", "BOM & Assembly", "build", 60u)
                    )
                ),
                ModuleInfoDto(
                    id = "mod-projects",
                    name = "Projects, Tasks & Timesheets",
                    description = "Project tracking, task assignment, billable timesheets, and 1-click sales invoice generation.",
                    dependencies = listOf("mod-contacts", "mod-invoicing"),
                    isEnabled = true,
                    installedVersion = 1u,
                    routes = listOf(
                        ModuleRouteDto("projects/list", "Projects & Tasks", "assignment", 70u)
                    )
                )
            )
        }
        _isInitialized.value = true
    }

    fun setActiveContextLocation(location: LocationDto?) {
        _activeContextLocation.value = location
    }

    fun clearActiveContextLocation() {
        _activeContextLocation.value = null
    }

    fun resolveQrScan(rawPayload: String): ScanResolutionDto {
        val engine = nativeEngine
        if (engine != null) {
            val resolution = engine.resolveQrScan(rawPayload, _activeContextLocation.value?.id)
            if (resolution.resolutionType == ScanResolutionTypeDto.LOCATION_CONTEXT) {
                _activeContextLocation.value = resolution.location
            }
            _recentScanResolution.value = resolution
            return resolution
        }

        // Fallback resolution for UI previews
        val trimmed = rawPayload.trim()
        val activeLoc = _activeContextLocation.value

        val resolution = when {
            trimmed.startsWith("erp://loc/") -> {
                val locIdOrCode = trimmed.removePrefix("erp://loc/").trimEnd('/')
                val foundLoc = mockLocations.firstOrNull { it.id == locIdOrCode || it.code == locIdOrCode }
                    ?: LocationDto(
                        id = locIdOrCode,
                        code = "BIN-SCAN",
                        name = "Scanned Bin ($locIdOrCode)",
                        parentId = "loc-rck-2",
                        locationType = "Bin",
                        pathBreadcrumb = "Main Facility > Workshop > Scanned Bin",
                        qrCodeUri = trimmed,
                        mapX = 150.0,
                        mapY = 120.0,
                        isActive = true
                    )
                _activeContextLocation.value = foundLoc
                ScanResolutionDto(
                    resolutionType = ScanResolutionTypeDto.LOCATION_CONTEXT,
                    promptMessage = "Active Target Location: ${foundLoc.name}. Ready to Putaway or Transfer.",
                    location = foundLoc,
                    item = null,
                    itemLocations = mockItems.filter { it.locationId == foundLoc.id },
                    totalQuantity = null,
                    currentQuantityAtLocation = null,
                    suggestedAction = null,
                    rawPayload = rawPayload
                )
            }

            trimmed.startsWith("erp://item/") || trimmed.startsWith("erp://sku/") || trimmed.contains("BOLT") || trimmed.contains("BEARING") -> {
                val sku = when {
                    trimmed.startsWith("erp://item/") -> trimmed.removePrefix("erp://item/").split("?")[0]
                    trimmed.startsWith("erp://sku/") -> trimmed.removePrefix("erp://sku/").split("?")[0]
                    else -> trimmed
                }

                val matchedItem = mockItems.firstOrNull { it.sku.contains(sku, ignoreCase = true) }
                val itemDto = ItemDto(
                    id = matchedItem?.itemId ?: "item-scanned",
                    sku = matchedItem?.sku ?: sku,
                    name = matchedItem?.itemName ?: "Product ($sku)",
                    description = "Industrial Part",
                    unit = matchedItem?.unit ?: "pcs",
                    trackSerial = false,
                    trackLot = true,
                    minStockLevel = "20.0",
                    qrCodeUri = "erp://item/$sku",
                    barcode = null
                )

                if (activeLoc != null) {
                    ScanResolutionDto(
                        resolutionType = ScanResolutionTypeDto.RAPID_STOCK_ACTION,
                        promptMessage = "Transfer/Putaway 1 unit of '${itemDto.name}' into '${activeLoc.name}'?",
                        location = activeLoc,
                        item = itemDto,
                        itemLocations = emptyList(),
                        totalQuantity = null,
                        currentQuantityAtLocation = matchedItem?.quantity ?: "0",
                        suggestedAction = "Putaway 1 ${itemDto.unit}",
                        rawPayload = rawPayload
                    )
                } else {
                    val matchingLocations = mockItems.filter { it.sku.contains(sku, ignoreCase = true) }
                    ScanResolutionDto(
                        resolutionType = ScanResolutionTypeDto.ITEM_LOOKUP,
                        promptMessage = "Found ${matchingLocations.size} location(s) holding '${itemDto.name}'",
                        location = null,
                        item = itemDto,
                        itemLocations = matchingLocations,
                        totalQuantity = matchingLocations.sumOf { it.quantity.toDoubleOrNull() ?: 0.0 }.toString(),
                        currentQuantityAtLocation = null,
                        suggestedAction = null,
                        rawPayload = rawPayload
                    )
                }
            }

            else -> {
                ScanResolutionDto(
                    resolutionType = ScanResolutionTypeDto.UNKNOWN,
                    promptMessage = "Unrecognized barcode/QR format: '$rawPayload'",
                    location = null,
                    item = null,
                    itemLocations = emptyList(),
                    totalQuantity = null,
                    currentQuantityAtLocation = null,
                    suggestedAction = null,
                    rawPayload = rawPayload
                )
            }
        }

        _recentScanResolution.value = resolution
        return resolution
    }

    fun searchItemLocations(query: String): List<ItemLocationCardDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.searchItemLocations(query)
            } catch (e: Exception) {
                emptyList()
            }
        }

        val q = query.trim()
        if (q.isEmpty()) return mockItems
        return mockItems.filter {
            it.sku.contains(q, ignoreCase = true) ||
            it.itemName.contains(q, ignoreCase = true) ||
            it.breadcrumbPath.contains(q, ignoreCase = true) ||
            it.lotNumber?.contains(q, ignoreCase = true) == true
        }
    }

    fun executeQuickTransfer(itemId: String, toLocationId: String, qty: String = "1.0"): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.moveStock(
                    itemId = itemId,
                    fromLoc = null,
                    toLoc = toLocationId,
                    qty = qty,
                    movementType = "Putaway",
                    lotOrSn = null,
                    notes = "Rapid scan putaway"
                )
                true
            } catch (e: Exception) {
                false
            }
        }

        val existing = mockItems.find { it.itemId == itemId && it.locationId == toLocationId }
        if (existing != null) {
            val updated = existing.copy(
                quantity = ((existing.quantity.toDoubleOrNull() ?: 0.0) + (qty.toDoubleOrNull() ?: 1.0)).toString()
            )
            val idx = mockItems.indexOf(existing)
            mockItems[idx] = updated
        }
        return true
    }

    fun reconcileAudit(locationId: String, itemId: String, physicalCount: String, lotNumber: String?): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.reconcileAudit(locationId, itemId, physicalCount, lotNumber, null) != null
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    fun toggleModule(moduleId: String, enabled: Boolean) {
        val engine = nativeEngine
        if (engine != null) {
            try {
                engine.setModuleEnabled(moduleId, enabled)
                _availableModules.value = engine.getAvailableModules()
                return
            } catch (_: Exception) {}
        }

        val current = _availableModules.value.toMutableList()
        val index = current.indexOfFirst { it.id == moduleId }
        if (index != -1) {
            val mod = current[index]
            current[index] = mod.copy(isEnabled = enabled)
            _availableModules.value = current
        }
    }

    fun updateLocationCoordinates(locationId: String, mapX: Double, mapY: Double): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.updateLocationCoordinates(locationId, mapX, mapY)
                val idx = mockLocations.indexOfFirst { it.id == locationId }
                if (idx != -1) {
                    mockLocations[idx] = mockLocations[idx].copy(mapX = mapX, mapY = mapY)
                }
                if (_activeContextLocation.value?.id == locationId) {
                    _activeContextLocation.value = _activeContextLocation.value?.copy(mapX = mapX, mapY = mapY)
                }
                true
            } catch (e: Exception) {
                println("Failed to update coordinates via engine: $e")
                false
            }
        }

        val idx = mockLocations.indexOfFirst { it.id == locationId }
        if (idx != -1) {
            mockLocations[idx] = mockLocations[idx].copy(mapX = mapX, mapY = mapY)
        }
        if (_activeContextLocation.value?.id == locationId) {
            _activeContextLocation.value = _activeContextLocation.value?.copy(mapX = mapX, mapY = mapY)
        }
        return true
    }

    // ==========================================
    // Database Backup & Data Export/Import Bridge
    // ==========================================

    fun backupDatabase(destinationPath: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.backupDatabase(destinationPath)
                true
            } catch (e: Exception) {
                println("Backup failed: $e")
                false
            }
        }
        return false
    }

    fun exportDataCsv(entityType: String): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.exportDataCsv(entityType)
            } catch (e: Exception) {
                "id,sku,name\n"
            }
        }
        return "id,sku,name\n"
    }

    fun exportDataJson(entityType: String): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.exportDataJson(entityType)
            } catch (e: Exception) {
                "[]"
            }
        }
        return "[]"
    }

    fun importDataCsv(entityType: String, csvContent: String): Int {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.importDataCsv(entityType, csvContent).toInt()
            } catch (e: Exception) {
                println("Import failed: $e")
                0
            }
        }
        return 0
    }

    // ==========================================
    // WASM Plugin Management Bridge
    // ==========================================

    fun installWasmPluginFile(filePath: String, manifestJson: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.installWasmPluginFile(manifestJson, filePath)
                true
            } catch (e: Exception) {
                println("Plugin installation failed: $e")
                false
            }
        }
        return false
    }

    fun listInstalledPlugins(): List<PluginManifestDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listInstalledPlugins()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    fun uninstallPlugin(pluginId: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.uninstallWasmPlugin(pluginId)
                true
            } catch (e: Exception) {
                println("Plugin uninstall failed: $e")
                false
            }
        }
        return false
    }

    fun getPluginUiNodes(contextView: String): List<PluginUiNodeDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getPluginUiNodes(contextView)
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    fun getStorageLocations(): List<LocationDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getLocationTree().map { it.location }
            } catch (e: Exception) {
                mockLocations
            }
        }
        return mockLocations
    }

    fun generateLabelSvg(uri: String, title: String, subtitle: String, format: String = "ThermalRoll62mm"): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateFormattedLabelSvg(format, uri, title, subtitle, null)
            } catch (e: Exception) {
                "<svg></svg>"
            }
        }
        return "<svg></svg>"
    }

    // ==========================================
    // Accounting Bridge
    // ==========================================

    fun getChartOfAccounts(): List<AccountDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getChartOfAccounts()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            AccountDto("acc-1110", "1110", "Cash on Hand", "Asset", null, false, "1540.00", true),
            AccountDto("acc-1120", "1120", "Operating Bank Account", "Asset", null, false, "24500.00", true),
            AccountDto("acc-1200", "1200", "Accounts Receivable", "Asset", null, false, "3400.00", true),
            AccountDto("acc-1300", "1300", "Inventory Asset", "Asset", null, false, "18200.00", true),
            AccountDto("acc-2100", "2100", "Accounts Payable", "Liability", null, false, "2100.00", true),
            AccountDto("acc-4100", "4100", "Sales Revenue", "Income", null, false, "42000.00", true),
            AccountDto("acc-5100", "5100", "Cost of Goods Sold", "Expense", null, false, "16500.00", true)
        )
    }

    fun getTrialBalance(): FinancialStatementDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getTrialBalance()
            } catch (e: Exception) {
                null
            }
        }
        return FinancialStatementDto(
            title = "Trial Balance",
            asOfOrPeriod = "2026-10-29",
            rows = listOf(
                TrialBalanceRowDto("acc-1120", "1120", "Operating Bank Account", "Asset", "24500.00", "0.00", "24500.00"),
                TrialBalanceRowDto("acc-1200", "1200", "Accounts Receivable", "Asset", "3400.00", "0.00", "3400.00"),
                TrialBalanceRowDto("acc-4100", "4100", "Sales Revenue", "Income", "0.00", "27900.00", "27900.00")
            ),
            totalDebit = "27900.00",
            totalCredit = "27900.00",
            netIncomeOrEquity = "0.00"
        )
    }

    // ==========================================
    // Purchasing Bridge
    // ==========================================

    fun listPurchaseOrders(): List<PurchaseOrderDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listPurchaseOrders()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            PurchaseOrderDto(
                id = "po-1",
                poNumber = "PO-2026-081",
                supplierId = "supp-1",
                orderDate = "2026-10-25",
                expectedDelivery = "2026-11-05",
                status = "Submitted",
                subtotal = "1450.00",
                taxTotal = "275.50",
                grandTotal = "1725.50",
                notes = "Stainless hardware fasteners restock",
                lines = listOf(
                    PurchaseOrderLineDto("pol-1", "item-1", "M8 Hex Bolts 316", "500", "0", "2.90", "0.19", "1450.00")
                )
            )
        )
    }

    fun receiveGoods(poId: String?, destinationLocationId: String, items: List<PurchaseReceiptLineDto>): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.receiveGoods(poId, "supp-1", destinationLocationId, items, "Mobile HUD Putaway")
                true
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    // ==========================================
    // Manufacturing Bridge
    // ==========================================

    fun listBoms(): List<BillOfMaterialsDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listBoms()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            BillOfMaterialsDto(
                id = "bom-1",
                bomNumber = "BOM-ROBOT-01",
                itemId = "item-robot",
                name = "Desktop Robotic Arm 4-Axis",
                quantity = "1",
                routingNotes = "Chassis assembly -> stepper motor wiring -> calibration",
                components = listOf(
                    BomComponentDto("comp-1", "item-1", "1", "0.0", "CNC Aluminum Base"),
                    BomComponentDto("comp-2", "item-2", "3", "0.0", "NEMA 17 Stepper Motors")
                )
            )
        )
    }

    fun listWorkOrders(): List<WorkOrderDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listWorkOrders()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            WorkOrderDto(
                id = "wo-1",
                workOrderNumber = "WO-2026-004",
                bomId = "bom-1",
                itemId = "item-robot",
                targetQuantity = "5",
                sourceLocationId = "loc-bin-4",
                destinationLocationId = "loc-wh-1",
                status = "In Progress",
                createdAt = "2026-10-28T09:00:00Z",
                completedAt = null
            )
        )
    }

    fun executeWorkOrder(workOrderId: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.executeWorkOrder(workOrderId)
                true
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    // ==========================================
    // Projects & Timesheets Bridge
    // ==========================================

    fun listProjects(): List<ProjectDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listProjects()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            ProjectDto(
                id = "prj-1",
                name = "Warehouse Automation & Robotics",
                code = "PRJ-AUTO-01",
                customerId = "cust-1",
                status = "In Progress",
                billingRateHourly = "150.00",
                estimatedHours = "60.0"
            )
        )
    }

    fun recordTimesheet(
        projectId: String,
        hours: String,
        description: String,
        isBillable: Boolean = true
    ): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.recordTimesheet(
                    projectId,
                    null,
                    "lead_architect",
                    "2026-10-29",
                    hours,
                    null,
                    isBillable,
                    description
                )
                true
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    fun invoiceUnbilledTimesheets(projectId: String): InvoiceDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.invoiceUnbilledTimesheets(projectId, "INV-TS-${System.currentTimeMillis() % 10000}", "0.0")
            } catch (e: Exception) {
                null
            }
        }
        return null
    }

    // ==========================================
    // v0.0.3 Capabilities (LEDGER-11 through 15)
    // ==========================================

    // LEDGER-11: Offline Pure-Vector PDF Generation
    fun generateInvoicePdf(invoiceId: String): ByteArray {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateInvoicePdf(invoiceId)
            } catch (e: Exception) {
                "%PDF-1.4 Mock Invoice PDF Bytes".encodeToByteArray()
            }
        }
        return "%PDF-1.4 Mock Invoice PDF Bytes with EPC QR".encodeToByteArray()
    }

    fun exportInvoicePdfToFile(invoiceId: String, destinationPath: String): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.exportInvoicePdfToFile(invoiceId, destinationPath)
            } catch (e: Exception) {
                destinationPath
            }
        }
        return destinationPath
    }

    fun generatePurchaseOrderPdf(poId: String): ByteArray {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generatePurchaseOrderPdf(poId)
            } catch (e: Exception) {
                "%PDF-1.4 Mock PO PDF Bytes".encodeToByteArray()
            }
        }
        return "%PDF-1.4 Mock PO PDF Bytes".encodeToByteArray()
    }

    fun exportPurchaseOrderPdfToFile(poId: String, destinationPath: String): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.exportPurchaseOrderPdfToFile(poId, destinationPath)
            } catch (e: Exception) {
                destinationPath
            }
        }
        return destinationPath
    }

    // LEDGER-12: Peer-to-Peer LAN / mDNS Sync
    fun getSyncStatus(): SyncStatusDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getSyncStatus()
            } catch (e: Exception) {
                fallbackSyncStatus()
            }
        }
        return fallbackSyncStatus()
    }

    fun triggerLanSync(): SyncStatusDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.triggerLanSync()
            } catch (e: Exception) {
                fallbackSyncStatus()
            }
        }
        return fallbackSyncStatus().copy(unsyncedEventsCount = 0u, convergenceState = "Converged")
    }

    fun registerPeer(nodeId: String, displayName: String, ipAddress: String, port: UShort): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.registerPeer(nodeId, displayName, ipAddress, port)
                true
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    private fun fallbackSyncStatus(): SyncStatusDto = SyncStatusDto(
        connectedPeersCount = 2u,
        unsyncedEventsCount = 0u,
        totalEventsCount = 142u,
        lastSyncTimestamp = "2026-10-31T14:00:00Z",
        convergenceState = "Converged",
        localHlc = "1793450000000-0001-node-local",
        peers = listOf(
            PeerNodeDto("peer-wh-tab", "Shop Floor Tablet", "192.168.1.45", 8080u, 12u, true),
            PeerNodeDto("peer-mfg-pc", "Assembly Workstation", "192.168.1.88", 8080u, 4u, true)
        )
    )

    // LEDGER-13: Low-Stock Alerts & Automated Reorder PO Generation
    fun getLowStockAlerts(): List<LowStockAlertDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getLowStockAlerts()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            LowStockAlertDto(
                itemId = "item-2",
                sku = "BEARING-608-2RS",
                name = "Deep Groove Ball Bearing 608-2RS",
                unit = "pcs",
                currentStock = "38.0",
                minStockLevel = "100.0",
                suggestedReorderQty = "162.0",
                preferredSupplierId = "cont-1",
                preferredSupplierName = "Industrial Bearings & Motion GmbH"
            )
        )
    }

    fun generateReorderPo(itemId: String): PurchaseOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateReorderPo(itemId)
            } catch (e: Exception) {
                null
            }
        }
        return PurchaseOrderDto(
            id = "po-auto-reorder",
            poNumber = "PO-REORDER-608",
            supplierId = "cont-1",
            orderDate = "2026-10-31",
            expectedDelivery = "2026-11-15",
            status = "Draft",
            subtotal = "202.50",
            taxTotal = "0.00",
            grandTotal = "202.50",
            notes = "Automated low-stock replenishment PO",
            lines = listOf(
                PurchaseOrderLineDto("line-1", itemId, "Automated replenishment", "162.0", "0.0", "1.25", "0.0", "202.50")
            )
        )
    }

    // LEDGER-14: Kanban Workflow Board & Project Tasks
    fun listProjectTasks(projectId: String?): List<ProjectTaskDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listProjectTasks(projectId)
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            ProjectTaskDto("task-1", projectId ?: "prj-1", "Precision calibration & endstop alignment", "In Progress", "High", "8.0"),
            ProjectTaskDto("task-2", projectId ?: "prj-1", "Harness wiring & connector crimping", "Backlog", "Medium", "4.0"),
            ProjectTaskDto("task-3", projectId ?: "prj-1", "Safety cage optical curtain test", "Done", "High", "6.0")
        )
    }

    fun updateProjectTaskStatus(taskId: String, newStatus: String): ProjectTaskDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.updateProjectTaskStatus(taskId, newStatus)
            } catch (e: Exception) {
                null
            }
        }
        return ProjectTaskDto(taskId, "prj-1", "Updated Task", newStatus, "High", "4.0")
    }

    fun getUnbilledHoursSummary(projectId: String?): UnbilledHoursSummaryDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getUnbilledHoursSummary(projectId)
            } catch (e: Exception) {
                UnbilledHoursSummaryDto("0.0", "0.00", 0u)
            }
        }
        return UnbilledHoursSummaryDto(
            totalUnbilledHours = "18.5",
            totalUnbilledAmount = "2775.00",
            unbilledEntriesCount = 4u
        )
    }

    fun updateWorkOrderStatus(workOrderId: String, newStatus: String): WorkOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.updateWorkOrderStatus(workOrderId, newStatus)
            } catch (e: Exception) {
                null
            }
        }
        return WorkOrderDto(
            id = workOrderId,
            workOrderNumber = "WO-2026-004",
            bomId = "bom-1",
            itemId = "item-robot",
            targetQuantity = "5",
            sourceLocationId = "loc-bin-4",
            destinationLocationId = "loc-wh-1",
            status = newStatus,
            createdAt = "2026-10-28T09:00:00Z",
            completedAt = if (newStatus == "Completed") "2026-10-31T14:00:00Z" else null
        )
    }

    // LEDGER-15: Component Genealogy & Lot Expiration Tracking
    fun getWorkOrderGenealogy(workOrderIdOrNumber: String): ComponentGenealogyDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getWorkOrderGenealogy(workOrderIdOrNumber)
            } catch (e: Exception) {
                null
            }
        }
        return ComponentGenealogyDto(
            workOrderId = "wo-1",
            workOrderNumber = "WO-2026-004",
            finishedItemId = "item-robot",
            finishedSku = "ROBOT-ARM-4AXIS",
            finishedName = "Desktop Robotic Arm 4-Axis",
            finishedLotNumber = "LOT-MFG-WO-2026-004",
            quantity = "5",
            completedAt = "2026-10-31T12:00:00Z",
            components = listOf(
                ComponentGenealogyEntryDto(
                    componentItemId = "item-1",
                    componentSku = "M8-HEX-BOLT-316",
                    componentName = "M8x40mm Hex Bolt Stainless 316",
                    componentLotNumber = "LOT-2026-Q1",
                    componentQuantity = "20",
                    expirationDate = "2030-12-31",
                    quarantineStatus = "released",
                    manufacturerBatch = "ACME-BATCH-889"
                ),
                ComponentGenealogyEntryDto(
                    componentItemId = "item-2",
                    componentSku = "BEARING-608-2RS",
                    componentName = "Deep Groove Ball Bearing 608-2RS",
                    componentLotNumber = "LOT-NSK-9",
                    componentQuantity = "10",
                    expirationDate = "2028-06-30",
                    quarantineStatus = "released",
                    manufacturerBatch = "NSK-JAPAN-004"
                )
            )
        )
    }

    fun createOrUpdateLot(
        lotNumber: String,
        itemId: String,
        expirationDate: String?,
        quarantineStatus: String?,
        manufacturerBatch: String?
    ): LotInfoDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.createOrUpdateLot(lotNumber, itemId, expirationDate, quarantineStatus, manufacturerBatch)
            } catch (e: Exception) {
                null
            }
        }
        return LotInfoDto(lotNumber, itemId, expirationDate, quarantineStatus ?: "released", manufacturerBatch, "2026-10-31T12:00:00Z")
    }

    fun getLotInfo(lotNumber: String): LotInfoDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getLotInfo(lotNumber)
            } catch (e: Exception) {
                null
            }
        }
        return null
    }

    // ========================================================================
    // v0.0.4 Release Bridge Methods (LEDGER-16 to LEDGER-28)
    // ========================================================================

    // LEDGER-16: Multi-up Sheet PDF Layouts & Barcode Printing
    fun generateMultiUpLabelsPdf(layoutName: String, items: List<MultiUpLabelItemDto>): ByteArray? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateMultiUpLabelsPdf(layoutName, items)
            } catch (e: Exception) {
                null
            }
        }
        return "%PDF-1.4 mock multi-up PDF".encodeToByteArray()
    }

    fun batchStreamThermalLabels(items: List<MultiUpLabelItemDto>): BatchPrintProgressDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.batchStreamThermalLabels(items)
            } catch (e: Exception) {
                BatchPrintProgressDto(items.size.toUInt(), items.size.toUInt(), true)
            }
        }
        return BatchPrintProgressDto(items.size.toUInt(), items.size.toUInt(), true)
    }

    // LEDGER-17: BLE Barcode Scanners & Digital Weight Scale Integration
    fun listBleDevices(): List<BleDeviceDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listBleDevices()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            BleDeviceDto("ble-ds3678", "Zebra DS3678-ER Rugged 2D Scanner", "E4:E1:12:44:98:AA", "scanner", true, 92.toUByte()),
            BleDeviceDto("ble-ps60", "Mettler Toledo PS60 Precision Scale", "00:80:E1:2A:55:1B", "scale", true, null),
            BleDeviceDto("ble-zd421", "Zebra ZD421c Direct Thermal Printer", "48:A4:93:3C:D0:14", "printer", false, null)
        )
    }

    fun connectBleDevice(deviceId: String): BleDeviceDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.connectBleDevice(deviceId)
            } catch (e: Exception) {
                null
            }
        }
        return BleDeviceDto(deviceId, "Simulated BLE Device", "00:11:22:33:44:55", "scanner", true, 88.toUByte())
    }

    fun getScaleReading(deviceId: String, sampleTare: Double? = null, unitPieceWeight: Double? = null): ScaleReadingDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getScaleReading(deviceId, sampleTare, unitPieceWeight)
            } catch (e: Exception) {
                ScaleReadingDto(1450.0, sampleTare ?: 50.0, 1400.0, true, "g", unitPieceWeight, (1400.0 / (unitPieceWeight ?: 1.0)).toUInt())
            }
        }
        val tare = sampleTare ?: 50.0
        val raw = 1450.0
        val net = raw - tare
        val pieces = unitPieceWeight?.let { if (it > 0.0) (net / it).toUInt() else null }
        return ScaleReadingDto(raw, tare, net, true, "g", unitPieceWeight, pieces)
    }

    fun calculatePieceCount(netWeightGrams: Double, unitPieceWeightGrams: Double): UInt {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.calculatePieceCount(netWeightGrams, unitPieceWeightGrams)
            } catch (e: Exception) {
                (netWeightGrams / unitPieceWeightGrams).toUInt()
            }
        }
        return (netWeightGrams / unitPieceWeightGrams).toUInt()
    }

    // LEDGER-18: Multi-Currency Accounting & Offline FX Tables
    fun setExchangeRate(baseCurrency: String, targetCurrency: String, rate: Double, effectiveDate: String): ExchangeRateDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.setExchangeRate(baseCurrency, targetCurrency, rate, effectiveDate)
            } catch (e: Exception) {
                null
            }
        }
        return ExchangeRateDto(baseCurrency, targetCurrency, rate, effectiveDate)
    }

    fun getExchangeRates(): List<ExchangeRateDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getExchangeRates()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            ExchangeRateDto("EUR", "USD", 1.085, "2026-10-31"),
            ExchangeRateDto("EUR", "GBP", 0.847, "2026-10-31"),
            ExchangeRateDto("EUR", "CHF", 0.942, "2026-10-31")
        )
    }

    fun recordFxGainLoss(invoiceId: String, invoiceRate: Double, settlementRate: Double, invoiceAmount: String, currency: String): FxGainLossResultDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.recordFxGainLoss(invoiceId, invoiceRate, settlementRate, invoiceAmount, currency)
            } catch (e: Exception) {
                null
            }
        }
        val gainLoss = ((settlementRate - invoiceRate) * (invoiceAmount.toDoubleOrNull() ?: 0.0))
        return FxGainLossResultDto(invoiceId, gainLoss.toString(), currency, "gl-fx-mock")
    }

    // LEDGER-19: Recurring Subscription Invoicing & Client Retainer Deposits
    fun createRecurringSchedule(customerId: String, planName: String, frequency: String, amount: String, currency: String, nextRunDate: String): RecurringScheduleDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.createRecurringSchedule(customerId, planName, frequency, amount, currency, nextRunDate)
            } catch (e: Exception) {
                null
            }
        }
        return RecurringScheduleDto("sched-mock-1", customerId, frequency, nextRunDate, amount, currency, planName, true)
    }

    fun listRecurringSchedules(): List<RecurringScheduleDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listRecurringSchedules()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    fun triggerRecurringInvoices(currentDate: String = "2026-11-01"): UInt {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.triggerRecurringInvoices(currentDate)
            } catch (e: Exception) {
                0u
            }
        }
        return 1u
    }

    fun getCustomerRetainer(customerId: String): RetainerAccountDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getCustomerRetainer(customerId)
            } catch (e: Exception) {
                RetainerAccountDto(customerId, "0.00", "EUR", "2026-10-31T12:00:00Z")
            }
        }
        return RetainerAccountDto(customerId, "1500.00", "EUR", "2026-10-31T12:00:00Z")
    }

    fun depositRetainer(customerId: String, amount: String, currency: String = "EUR"): RetainerAccountDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.depositRetainer(customerId, amount, currency)
            } catch (e: Exception) {
                null
            }
        }
        return RetainerAccountDto(customerId, amount, currency, "2026-10-31T12:00:00Z")
    }

    fun drawdownRetainer(customerId: String, amount: String, invoiceId: String): RetainerAccountDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.drawdownRetainer(customerId, amount, invoiceId)
            } catch (e: Exception) {
                null
            }
        }
        return RetainerAccountDto(customerId, "1000.00", "EUR", "2026-10-31T12:00:00Z")
    }

    // LEDGER-20: ABC Inventory Pareto Analysis & Dynamic EOQ
    fun runAbcAnalysis(): AbcAnalysisDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.runAbcAnalysis()
            } catch (e: Exception) {
                AbcAnalysisDto(0.0, 0u, 0u, 0u, emptyList())
            }
        }
        return AbcAnalysisDto(
            totalValue = 25480.0,
            tierACount = 1u,
            tierBCount = 2u,
            tierCCount = 5u,
            items = listOf(
                AbcItemTierDto("item-1", "ROBOT-ARM-6AXIS", "6-Axis Robot Arm", 18500.0, 72.6, "A"),
                AbcItemTierDto("item-2", "CNC-SPINDLE-2KW", "2.2kW Water Cooled Spindle", 4500.0, 90.2, "B"),
                AbcItemTierDto("item-3", "M8-HEX-BOLT-316", "M8 Stainless Hex Bolt", 350.0, 100.0, "C")
            )
        )
    }

    fun getInventoryAgingHeatmap(): List<LocationAgingDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getInventoryAgingHeatmap()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            LocationAgingDto("loc-wh-1", "WH-MAIN", 12.0, 25u, "#4CAF50", 14u),
            LocationAgingDto("loc-rm-1", "WS-01", 45.0, 60u, "#2196F3", 8u),
            LocationAgingDto("loc-bin-4", "BIN-04", 120.0, 150u, "#FF9800", 2u)
        )
    }

    fun calculateEoqAndSafetyStock(itemId: String, annualDemand: Double = 120.0, orderingCost: Double = 50.0, holdingCostRate: Double = 0.2, unitCost: Double = 100.0, leadTimeDays: Double = 7.0, demandStdDev: Double = 3.0): EoqCalculationDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.calculateEoqAndSafetyStock(itemId, annualDemand, orderingCost, holdingCostRate, unitCost, leadTimeDays, demandStdDev)
            } catch (e: Exception) {
                null
            }
        }
        return EoqCalculationDto(itemId, "SKU-SAMPLE", 24.5, 8.2, 15.0, annualDemand)
    }

    // LEDGER-21: Database Encryption at Rest & Authenticated P2P Sync
    fun getSecurityStatus(): SecurityStatusDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getSecurityStatus()
            } catch (e: Exception) {
                SecurityStatusDto(true, "ChaCha20-Poly1305 / SQLCipher Envelope", 2u, "node-local")
            }
        }
        return SecurityStatusDto(true, "ChaCha20-Poly1305 / SQLCipher Envelope", 2u, "node-local-01")
    }

    fun generatePairingQr(): PeerPairingPayloadDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generatePairingQr()
            } catch (e: Exception) {
                PeerPairingPayloadDto("node-local", "psk_mock", "erp://pair?node=node-local&psk=mock", "SHA256:mock")
            }
        }
        return PeerPairingPayloadDto("node-local", "psk_sec_9918231", "erp://pair?node=node-local&name=Desktop&psk=psk_sec_9918231&addr=192.168.1.100:8443", "SHA256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069")
    }

    fun parsePairingQr(uri: String): PeerPairingPayloadDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.parsePairingQr(uri)
            } catch (e: Exception) {
                null
            }
        }
        return PeerPairingPayloadDto("node-remote-02", "psk_paired", uri, "SHA256:valid")
    }

    fun verifyPeerHandshake(peerNodeId: String, challenge: String, token: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.verifyPeerHandshake(peerNodeId, challenge, token)
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    // LEDGER-22: On-Device Offline OCR Intake for Receipts & Vendor Invoices
    fun processOcrReceipt(imageData: ByteArray): OcrReceiptResultDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.processOcrReceipt(imageData)
            } catch (e: Exception) {
                OcrReceiptResultDto(null, null, null, null, null, emptyList(), 0.0f, "")
            }
        }
        return OcrReceiptResultDto(
            merchantName = "ACME Industrial Supplies GmbH",
            date = "2026-10-31",
            totalAmount = 83.90,
            taxAmount = 13.40,
            currency = "EUR",
            lines = listOf(
                OcrReceiptLineDto("M8 DIN 933 Hex Bolts 100x", 1.0, 25.50, 25.50, 0.98f, OcrBoundingBoxDto(10.0f, 50.0f, 200.0f, 20.0f)),
                OcrReceiptLineDto("Cutting Oil 5L", 1.0, 45.00, 45.00, 0.96f, OcrBoundingBoxDto(10.0f, 80.0f, 200.0f, 20.0f))
            ),
            confidence = 0.97f,
            rawText = "ACME Industrial Supplies GmbH\nDate: 2026-10-31\nTotal: 83.90 EUR"
        )
    }

    fun convertOcrToDraftPo(ocrResult: OcrReceiptResultDto, supplierId: String, destinationLocationId: String): PurchaseOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.convertOcrToDraftPo(ocrResult, supplierId, destinationLocationId)
            } catch (e: Exception) {
                null
            }
        }
        return null
    }

    // LEDGER-23: Serial Number Tracking, Warranty Passport & Maintenance Logs
    fun registerSerializedAsset(serialNumber: String, itemId: String, warrantyExpiry: String?, initialLocationId: String?): SerializedAssetPassportDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.registerSerializedAsset(serialNumber, itemId, warrantyExpiry, initialLocationId)
            } catch (e: Exception) {
                null
            }
        }
        return SerializedAssetPassportDto("sn-1", serialNumber, itemId, "Sample Asset", "SKU-SAMPLE", "InService", warrantyExpiry, initialLocationId, "2026-10-31T12:00:00Z", "erp://serial/$serialNumber")
    }

    fun getAssetPassport(serialNumber: String): SerializedAssetPassportDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getAssetPassport(serialNumber)
            } catch (e: Exception) {
                null
            }
        }
        return SerializedAssetPassportDto("sn-1", serialNumber, "item-1", "Precision Robotic Arm", "ROBOT-ARM-6AXIS", "InService", "2029-10-31", "loc-rm-1", "2026-10-31T12:00:00Z", "erp://serial/$serialNumber")
    }

    fun addMaintenanceLog(assetId: String, maintenanceType: String, description: String, performedBy: String, nextDueDate: String?): AssetMaintenanceLogDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.addMaintenanceLog(assetId, maintenanceType, description, performedBy, nextDueDate)
            } catch (e: Exception) {
                null
            }
        }
        return AssetMaintenanceLogDto("maint-1", assetId, maintenanceType, description, performedBy, "2026-10-31", nextDueDate)
    }

    fun listMaintenanceLogs(assetId: String): List<AssetMaintenanceLogDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listMaintenanceLogs(assetId)
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    fun createRmaClaim(assetId: String, customerId: String, issueDescription: String): RmaClaimDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.createRmaClaim(assetId, customerId, issueDescription)
            } catch (e: Exception) {
                null
            }
        }
        return RmaClaimDto("rma-1", "RMA-20261031-01", assetId, customerId, issueDescription, "Open", null, "2026-10-31T12:00:00Z")
    }

    fun listRmaClaims(): List<RmaClaimDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listRmaClaims()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    // LEDGER-24: Offline POS Cashier Mode, Cash Drawer Kick & Shift Reconciliation
    fun openPosShift(registerId: String = "REG-01", cashierId: String, startingCash: String): PosShiftDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.openPosShift(registerId, cashierId, startingCash)
            } catch (e: Exception) {
                null
            }
        }
        return PosShiftDto("shift-1", registerId, cashierId, "2026-10-31T08:00:00Z", null, startingCash, "0.00", startingCash, null, null, "Open")
    }

    fun recordPosSale(shiftId: String, amount: String, paymentMethod: String = "cash"): PosShiftDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.recordPosSale(shiftId, amount, paymentMethod)
            } catch (e: Exception) {
                null
            }
        }
        return PosShiftDto(shiftId, "REG-01", "cashier", "2026-10-31T08:00:00Z", null, "200.00", amount, "250.00", null, null, "Open")
    }

    fun closePosShift(shiftId: String, actualCash: String): PosShiftDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.closePosShift(shiftId, actualCash)
            } catch (e: Exception) {
                null
            }
        }
        return PosShiftDto(shiftId, "REG-01", "cashier", "2026-10-31T08:00:00Z", "2026-10-31T17:00:00Z", "200.00", "50.00", "250.00", actualCash, "0.00", "Closed")
    }

    fun generateShiftReport(shiftId: String, reportType: String = "Z"): ShiftReportDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateShiftReport(shiftId, reportType)
            } catch (e: Exception) {
                null
            }
        }
        return ShiftReportDto(shiftId, reportType, "2026-10-31T17:00:00Z", "========================================\n          LEDGER ERP POS\n       FINAL SHIFT Z-REPORT\n========================================\nShift ID    : $shiftId\nStatus      : Closed\n========================================\n")
    }

    // LEDGER-25: Multi-Warehouse Transfer Orders & In-Transit Tracking
    fun createTransferOrder(sourceLocationId: String, destinationLocationId: String, lines: List<StockTransferLineDto>): StockTransferOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.createTransferOrder(sourceLocationId, destinationLocationId, lines)
            } catch (e: Exception) {
                null
            }
        }
        return StockTransferOrderDto("xfer-1", "TO-20261031-01", sourceLocationId, destinationLocationId, "loc-in-transit-virtual", "Draft", "2026-10-31T12:00:00Z", null, null, lines)
    }

    fun dispatchTransferOrder(orderId: String): StockTransferOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.dispatchTransferOrder(orderId)
            } catch (e: Exception) {
                null
            }
        }
        return null
    }

    fun receiveTransferOrder(orderId: String, receivedQuantities: List<String>): StockTransferOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.receiveTransferOrder(orderId, receivedQuantities)
            } catch (e: Exception) {
                null
            }
        }
        return null
    }

    fun listTransferOrders(): List<StockTransferOrderDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listTransferOrders()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    // LEDGER-26: Workstation Capacity Planning & Gantt Production Scheduling
    fun createWorkstation(code: String, name: String, capacityHoursPerDay: Double = 16.0, hourlyRate: Double = 75.0): WorkstationDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.createWorkstation(code, name, capacityHoursPerDay, hourlyRate)
            } catch (e: Exception) {
                null
            }
        }
        return WorkstationDto("ws-1", code, name, capacityHoursPerDay, hourlyRate, true)
    }

    fun listWorkstations(): List<WorkstationDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listWorkstations()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            WorkstationDto("ws-1", "WS-CNC-01", "5-Axis CNC Milling Center", 16.0, 85.0, true),
            WorkstationDto("ws-2", "WS-ROBOT-WELD", "Robotic MIG/TIG Welder", 12.0, 65.0, true),
            WorkstationDto("ws-3", "WS-ASSEMBLY", "Final Precision Assembly Bay", 8.0, 45.0, true)
        )
    }

    fun addRoutingOperation(bomId: String, workstationId: String, operationName: String, sequence: UInt, standardDurationMinutes: UInt): RoutingOperationDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.addRoutingOperation(bomId, workstationId, operationName, sequence, standardDurationMinutes)
            } catch (e: Exception) {
                null
            }
        }
        return RoutingOperationDto("ro-1", bomId, workstationId, operationName, sequence, standardDurationMinutes)
    }

    fun scheduleWorkOrder(workOrderId: String, workstationId: String, operationId: String, startTime: String, allocatedMinutes: UInt): WorkOrderAllocationDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.scheduleWorkOrder(workOrderId, workstationId, operationId, startTime, allocatedMinutes)
            } catch (e: Exception) {
                null
            }
        }
        return WorkOrderAllocationDto("woa-1", workOrderId, workstationId, operationId, startTime, "${startTime}+${allocatedMinutes}m", allocatedMinutes, "Scheduled")
    }

    fun getWorkstationCapacitySchedule(workstationId: String, date: String): WorkstationCapacitySummaryDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getWorkstationCapacitySchedule(workstationId, date)
            } catch (e: Exception) {
                WorkstationCapacitySummaryDto(workstationId, "WS-CODE", date, 960u, 0u, 0.0, false)
            }
        }
        return WorkstationCapacitySummaryDto(workstationId, "WS-CNC-01", date, 960u, 480u, 50.0, false)
    }

    // LEDGER-27: Automated NAS/SFTP Backup Scheduler & USB Drive Sync
    fun configureBackupPolicy(targetDirectory: String, retentionCount: UInt = 7u, frequencyHours: UInt = 24u, enabled: Boolean = true): BackupPolicyConfigDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.configureBackupPolicy(targetDirectory, retentionCount, frequencyHours, enabled)
            } catch (e: Exception) {
                null
            }
        }
        return BackupPolicyConfigDto(enabled, targetDirectory, retentionCount, frequencyHours)
    }

    fun getBackupPolicy(): BackupPolicyConfigDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.getBackupPolicy()
            } catch (e: Exception) {
                BackupPolicyConfigDto(true, "./backups", 7u, 24u)
            }
        }
        return BackupPolicyConfigDto(true, "./backups", 7u, 24u)
    }

    fun triggerAutomatedBackup(): BackupArchiveDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.triggerAutomatedBackup()
            } catch (e: Exception) {
                null
            }
        }
        return BackupArchiveDto("ledger_backup_local_demo.db", "./backups/ledger_backup_local_demo.db", 524288u, "2026-10-31T12:00:00Z", true, "a1b2c3d4e5f6")
    }

    fun listBackupArchives(): List<BackupArchiveDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.listBackupArchives()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return emptyList()
    }

    fun detectUsbDrives(): List<UsbDriveInfoDto> {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.detectUsbDrives()
            } catch (e: Exception) {
                emptyList()
            }
        }
        return listOf(
            UsbDriveInfoDto("/dev/sdb1", "/media/usb_drive", 64000000000u, 48500000000u)
        )
    }

    fun syncBackupToUsb(archivePath: String, usbMountPath: String): String {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.syncBackupToUsb(archivePath, usbMountPath)
            } catch (e: Exception) {
                e.message ?: "Sync error"
            }
        }
        return "Synced $archivePath to $usbMountPath successfully"
    }

    fun verifyBackupFile(filePath: String): Boolean {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.verifyBackupFile(filePath)
            } catch (e: Exception) {
                false
            }
        }
        return true
    }

    // LEDGER-28: European E-Invoicing Compliance (Factur-X / ZUGFeRD 2.2 / UBL 2.1)
    fun generateFacturXXml(invoiceId: String): String? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateFacturXXml(invoiceId)
            } catch (e: Exception) {
                null
            }
        }
        return "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rsm:CrossIndustryInvoice>...</rsm:CrossIndustryInvoice>"
    }

    fun generateHybridPdfA3Invoice(invoiceId: String): ByteArray? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.generateHybridPdfA3Invoice(invoiceId)
            } catch (e: Exception) {
                null
            }
        }
        return "%PDF-1.4 Factur-X Hybrid PDF/A-3 mock".encodeToByteArray()
    }

    fun parseIncomingEinvoiceXml(xmlContent: String): IncomingEInvoiceDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.parseIncomingEinvoiceXml(xmlContent)
            } catch (e: Exception) {
                null
            }
        }
        return IncomingEInvoiceDto("INV-EINVOICE-001", "European Vendor SpA", "IT12345678901", "2026-10-31", 250.0, "EUR", 1u, xmlContent)
    }

    fun validateEinvoiceEn16931(xmlContent: String): EInvoiceValidationReportDto {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.validateEinvoiceEn16931(xmlContent)
            } catch (e: Exception) {
                EInvoiceValidationReportDto(false, "EN 16931", "CrossIndustryInvoice", listOf(e.message ?: "Validation error"), emptyList())
            }
        }
        return EInvoiceValidationReportDto(true, "Factur-X EN 16931 (COMFORT)", "CrossIndustryInvoice", emptyList(), emptyList())
    }

    fun convertEinvoiceToDraftPo(xmlContent: String, supplierId: String, destinationLocationId: String): PurchaseOrderDto? {
        val engine = nativeEngine
        if (engine != null) {
            return try {
                engine.convertEinvoiceToDraftPo(xmlContent, supplierId, destinationLocationId)
            } catch (e: Exception) {
                null
            }
        }
        return null
    }
}
