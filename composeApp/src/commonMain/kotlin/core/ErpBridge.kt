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
            serialNumber = null
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
            serialNumber = null
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
}
