package dev.daftar.daftar

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews

/** The "Record" home-screen widget: opens the app straight into recording (§8.1). */
class RecordWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        val open = Intent(context, MainActivity::class.java).apply {
            action = MainActivity.ACTION_RECORD
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val pending = PendingIntent.getActivity(
            context, 0, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val views = RemoteViews(context.packageName, R.layout.record_widget).apply {
            setOnClickPendingIntent(R.id.record_widget, pending)
            setContentDescription(R.id.record_widget, context.getString(R.string.record_widget_label))
        }
        ids.forEach { manager.updateAppWidget(it, views) }
    }
}
