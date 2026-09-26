package dev.daftar.daftar

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import java.io.File
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * Receives what other apps share into Daftar (§8.1: sharing creates a raw capture). Shared items
 * wait here until the Dart side takes them, so a share that cold-starts the app is not lost.
 */
class MainActivity : FlutterActivity() {
    companion object {
        /** Sent by the home-screen widget: start recording as soon as Today is up. */
        const val ACTION_RECORD = "dev.daftar.daftar.RECORD"
    }

    private val pending = mutableListOf<Map<String, Any>>()
    private var channel: MethodChannel? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "daftar/share").apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "take" -> {
                        result.success(pending.toList())
                        pending.clear()
                    }
                    else -> result.notImplemented()
                }
            }
        }
        collect(intent)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "daftar/voice").setMethodCallHandler { call, result ->
            when (call.method) {
                "start" -> {
                    VoiceService.start(
                        this,
                        call.argument<String>("title") ?: "",
                        call.argument<String>("body") ?: "",
                        call.argument<String>("channel") ?: "",
                    )
                    result.success(null)
                }
                "stop" -> {
                    VoiceService.stop(this)
                    result.success(null)
                }
                else -> result.notImplemented()
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (collect(intent)) channel?.invokeMethod("arrived", null)
    }

    /** Queues the intent's text, images or record request; returns whether anything arrived. */
    private fun collect(intent: Intent?): Boolean {
        if (intent == null) return false
        val before = pending.size
        when (intent.action) {
            Intent.ACTION_SEND -> {
                intent.getStringExtra(Intent.EXTRA_TEXT)?.takeIf { it.isNotBlank() }?.let {
                    val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT)
                    val text = if (subject.isNullOrBlank() || it.contains(subject)) it else "$subject\n$it"
                    pending.add(mapOf("text" to text))
                }
                stream(intent)?.let { attachment(it) }
            }
            Intent.ACTION_SEND_MULTIPLE -> streams(intent).forEach { attachment(it) }
            ACTION_RECORD -> pending.add(mapOf("record" to true))
        }
        // Handled once: a configuration change must not file the same share again.
        if (pending.size > before) intent.action = Intent.ACTION_MAIN
        return pending.size > before
    }

    /** An image becomes a photo capture; any other file (a document, a recording) is copied
     *  here and opened in the import preview. */
    private fun attachment(uri: Uri) {
        val type = contentResolver.getType(uri) ?: ""
        if (type.startsWith("image/")) {
            val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return
            pending.add(mapOf("image" to bytes))
            return
        }
        val dir = File(cacheDir, "shared").apply { mkdirs() }
        val name = displayName(uri)?.replace('/', '_')?.takeIf { it.isNotBlank() } ?: "shared"
        val out = File(dir, "${System.currentTimeMillis()}-$name")
        contentResolver.openInputStream(uri)?.use { input ->
            out.outputStream().use { input.copyTo(it) }
        } ?: return
        pending.add(mapOf("file" to out.absolutePath))
    }

    private fun displayName(uri: Uri): String? =
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
            if (it.moveToFirst()) it.getString(0) else null
        } ?: uri.lastPathSegment

    @Suppress("DEPRECATION")
    private fun stream(intent: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableExtra(Intent.EXTRA_STREAM)

    @Suppress("DEPRECATION")
    private fun streams(intent: Intent): List<Uri> =
        (if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)) ?: emptyList()
}
