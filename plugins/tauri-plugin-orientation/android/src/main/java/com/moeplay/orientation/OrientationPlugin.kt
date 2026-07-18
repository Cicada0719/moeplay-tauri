package com.moeplay.orientation

import android.app.Activity
import android.content.pm.ActivityInfo
import android.content.res.Configuration
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class SetOrientationArgs {
    lateinit var mode: String
}

@TauriPlugin
class OrientationPlugin(private val activity: Activity) : Plugin(activity) {
    private val ORIENTATION_EVENT = "orientation-change"
    private var requestedMode: String = "auto"

    @Command
    fun setOrientation(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SetOrientationArgs::class.java)
            val orientation = when (args.mode) {
                "portrait" -> ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT
                "landscape" -> ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                "auto" -> ActivityInfo.SCREEN_ORIENTATION_FULL_USER
                else -> throw IllegalArgumentException("Unsupported orientation mode: ${args.mode}")
            }
            requestedMode = args.mode
            activity.runOnUiThread {
                activity.requestedOrientation = orientation
                invoke.resolve(response())
            }
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to set orientation")
        }
    }

    @Command
    fun getOrientation(invoke: Invoke) {
        invoke.resolve(response())
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        trigger(ORIENTATION_EVENT, response(newConfig.orientation))
    }

    override fun onResume() {
        trigger(ORIENTATION_EVENT, response())
    }

    private fun response(configurationOrientation: Int = activity.resources.configuration.orientation): JSObject {
        val current = if (configurationOrientation == Configuration.ORIENTATION_LANDSCAPE) {
            "landscape"
        } else {
            "portrait"
        }
        return JSObject().apply {
            put("mode", requestedMode)
            put("orientation", current)
        }
    }
}