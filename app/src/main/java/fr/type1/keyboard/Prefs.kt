package fr.type1.keyboard

import android.content.Context

/** What a long press on the backspace key does. */
enum class BackspaceHold { None, Repeat, Select }

/** Small preference store shared by the IME and the setup screen. */
object Prefs {
	private const val PREFS = "type1"
	private const val SPACE_CURSOR = "space_cursor"
	private const val BACKSPACE_HOLD = "backspace_hold"
	private const val SWIPE_SENSITIVITY = "swipe_sensitivity"

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

	/** Long-press behaviour of the backspace key. Selects text by default. */
	fun backspaceHold(context: Context): BackspaceHold {
		val stored = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. getString(BACKSPACE_HOLD, null)
		return when (stored) {
			BackspaceHold.None.name -> BackspaceHold.None
			BackspaceHold.Repeat.name -> BackspaceHold.Repeat
			else -> BackspaceHold.Select
		}
	}

	fun setBackspaceHold(context: Context, mode: BackspaceHold) {
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. edit()
		. putString(BACKSPACE_HOLD, mode.name)
		. apply()
	}

	/**
	 * Horizontal travel, in dp, needed to advance one caret/selection step.
	 * Lower values make both swipe gestures faster. Default 20 dp.
	 */
	fun swipeSensitivity(context: Context): Int = (
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. getInt(SWIPE_SENSITIVITY, 20)
		. coerceIn(8, 40)
	)

	fun setSwipeSensitivity(context: Context, dpPerStep: Int) {
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
		. edit()
		. putInt(SWIPE_SENSITIVITY, dpPerStep.coerceIn(8, 40))
		. apply()
	}
}
