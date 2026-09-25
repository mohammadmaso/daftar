package dev.daftar.daftar

import android.content.Intent
import android.net.Uri
import android.os.Build
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
                stream(intent)?.let { image(it) }
            }
            Intent.ACTION_SEND_MULTIPLE -> streams(intent).forEach { image(it) }
            ACTION_RECORD -> pending.add(mapOf("record" to true))
        }
        // Handled once: a configuration change must not file the same share again.
        if (pending.size > before) intent.action = Intent.ACTION_MAIN
        return pending.size > before
    }

    private fun image(uri: Uri) {
        val type = contentResolver.getType(uri) ?: return
        if (!type.startsWith("image/")) return
        val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return
        pending.add(mapOf("image" to bytes))
    }

    @Suppress("DEPRECATION")
    private fun stream(intent: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableExtra(Intent.EXTRA_STREAM)

    @Suppress("DEPRECATION")
    private fun streams(intent: Intent): List<Uri> =
        (if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)) ?: emptyList()
}
