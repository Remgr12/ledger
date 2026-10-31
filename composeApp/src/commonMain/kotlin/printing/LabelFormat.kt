package org.erp.ledger.printing

/**
 * Standard label media formats supported by thermal, POS, and office printers.
 */
enum class LabelFormat(
    val displayName: String,
    val description: String,
    val widthMm: Double,
    val heightMm: Double,
    val isContinuous: Boolean
) {
    BROTHER_62MM(
        displayName = "Brother 62mm Thermal Roll",
        description = "DK-22205 continuous roll / DK-11209 62x29mm die-cut address labels",
        widthMm = 62.0,
        heightMm = 29.0,
        isContinuous = true
    ),
    AVERY_DIE_CUT(
        displayName = "Avery Die-Cut Sheets (Avery 5160 / 24-up)",
        description = "Standard letter/A4 sheet with 24-30 adhesive labels (63.5 x 33.9mm)",
        widthMm = 63.5,
        heightMm = 33.9,
        isContinuous = false
    ),
    ESCPOS_58MM(
        displayName = "ESC/POS 58mm / 80mm Roll",
        description = "Point-of-Sale thermal receipt paper for shelf tags & bin labels",
        widthMm = 58.0,
        heightMm = 40.0,
        isContinuous = true
    ),
    TSPL_4X6(
        displayName = "TSPL / Zebra 4x6\" Shipping",
        description = "Direct thermal industrial pallet and warehouse bin label (101.6 x 152.4mm)",
        widthMm = 101.6,
        heightMm = 152.4,
        isContinuous = false
    )
}
