package org.erp.ledger.scanner

import android.content.Context
import android.util.Size
import androidx.annotation.OptIn
import androidx.camera.core.*
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * Android CameraX + MLKit Barcode Scanning Configuration & Engine.
 * Configures ImageAnalysis with BarcodeScannerOptions for QR codes, EAN-13, and Code 128,
 * and streams recognized payloads to the Rapid Scan & Move HUD.
 */
object AndroidBarcodeScanner {
    const val SCANNER_RESOLUTION_WIDTH = 1280
    const val SCANNER_RESOLUTION_HEIGHT = 720

    private val executor: ExecutorService = Executors.newSingleThreadExecutor()

    val scannerOptions: BarcodeScannerOptions = BarcodeScannerOptions.Builder()
        .setBarcodeFormats(
            Barcode.FORMAT_QR_CODE,
            Barcode.FORMAT_EAN_13,
            Barcode.FORMAT_CODE_128,
            Barcode.FORMAT_DATA_MATRIX,
            Barcode.FORMAT_UPC_A,
            Barcode.FORMAT_UPC_E
        )
        .build()

    val barcodeScanner: BarcodeScanner = BarcodeScanning.getClient(scannerOptions)

    fun getSupportedBarcodeFormats(): List<String> {
        return listOf(
            "FORMAT_QR_CODE",
            "FORMAT_EAN_13",
            "FORMAT_CODE_128",
            "FORMAT_DATA_MATRIX",
            "FORMAT_UPC_A",
            "FORMAT_UPC_E"
        )
    }

    /**
     * CameraX ImageAnalysis analyzer that converts frame buffers to MLKit InputImage
     * and streams recognized barcodes.
     */
    class BarcodeAnalyzer(
        private val onBarcodeDetected: (String) -> Unit
    ) : ImageAnalysis.Analyzer {
        private var lastScanTimestamp = 0L
        private val throttleMs = 500L

        @OptIn(ExperimentalGetImage::class)
        override fun analyze(imageProxy: ImageProxy) {
            val mediaImage = imageProxy.image
            val now = System.currentTimeMillis()
            if (mediaImage != null && now - lastScanTimestamp > throttleMs) {
                val inputImage = InputImage.fromMediaImage(
                    mediaImage,
                    imageProxy.imageInfo.rotationDegrees
                )

                barcodeScanner.process(inputImage)
                    .addOnSuccessListener { barcodes ->
                        for (barcode in barcodes) {
                            val rawValue = barcode.rawValue
                            if (!rawValue.isNullOrBlank()) {
                                lastScanTimestamp = now
                                onBarcodeDetected(rawValue)
                                break
                            }
                        }
                    }
                    .addOnFailureListener {
                        // Frame skip on analysis failure
                    }
                    .addOnCompleteListener {
                        imageProxy.close()
                    }
            } else {
                imageProxy.close()
            }
        }
    }

    /**
     * Binds CameraX Preview and ImageAnalysis use cases to the Android LifecycleOwner.
     */
    fun startCamera(
        context: Context,
        lifecycleOwner: LifecycleOwner,
        surfaceProvider: Preview.SurfaceProvider,
        onBarcodeScanned: (String) -> Unit,
        onCameraBound: (Camera) -> Unit = {}
    ) {
        val cameraProviderFuture = ProcessCameraProvider.getInstance(context)
        cameraProviderFuture.addListener({
            val cameraProvider = cameraProviderFuture.get()

            val preview = Preview.Builder()
                .setTargetResolution(Size(SCANNER_RESOLUTION_WIDTH, SCANNER_RESOLUTION_HEIGHT))
                .build()
                .also {
                    it.setSurfaceProvider(surfaceProvider)
                }

            val imageAnalysis = ImageAnalysis.Builder()
                .setTargetResolution(Size(SCANNER_RESOLUTION_WIDTH, SCANNER_RESOLUTION_HEIGHT))
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .build()
                .also {
                    it.setAnalyzer(executor, BarcodeAnalyzer(onBarcodeScanned))
                }

            val cameraSelector = CameraSelector.DEFAULT_BACK_CAMERA

            try {
                cameraProvider.unbindAll()
                val camera = cameraProvider.bindToLifecycle(
                    lifecycleOwner,
                    cameraSelector,
                    preview,
                    imageAnalysis
                )
                onCameraBound(camera)
            } catch (exc: Exception) {
                exc.printStackTrace()
            }
        }, ContextCompat.getMainExecutor(context))
    }
}
