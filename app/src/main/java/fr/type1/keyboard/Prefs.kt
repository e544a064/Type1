package fr.type1.keyboard

import android.content.Context

/** Small preference store shared by the IME and the setup screen. */
object Prefs {
	private const val PREFS = "type1"
	private const val SPACE_CURSOR = "space_cursor"

	/** Drag horizontally on the spacebar to move the caret. Enabled by default. */
	fun spaceCursor(context: Context): Boolean = (
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. getBoolean(SPACE_CURSOR, true)
	)

	fun setSpaceCursor(context: Context, enabled: Boolean) {
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. edit()
		. putBoolean(SPACE_CURSOR, enabled)
		. apply()
	}
}
