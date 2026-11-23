package fr.type1.keyboard

import android.content.Context
import android.content.SharedPreferences
import fr.type1.keyboard.model.KeyboardProfile

/** What a long press on the backspace key does. */
enum class BackspaceHold { None, Repeat, Select }

/** Small preference store shared by the IME and the setup screen. */
object Prefs {
	private const val PREFS = "type1"
	private const val SPACE_CURSOR = "space_cursor"
	private const val BACKSPACE_HOLD = "backspace_hold"
	private const val SWIPE_SENSITIVITY = "swipe_sensitivity"
	private const val UTILITY_ROW_HEIGHT = "utility_row_height"
	const val PROFILES = "profiles"
	const val ACTIVE_PROFILE = "active_profile"

	fun preferences(context: Context): SharedPreferences =
		context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

	/** Drag horizontally on the spacebar to move the caret. Enabled by default. */
	fun spaceCursor(context: Context): Boolean = (
		preferences(context)
		. getBoolean(SPACE_CURSOR, true)
	)

	fun setSpaceCursor(context: Context, enabled: Boolean) {
		preferences(context)
		. edit()
		. putBoolean(SPACE_CURSOR, enabled)
		. apply()
	}

	/** Long-press behaviour of the backspace key. Selects text by default. */
	fun backspaceHold(context: Context): BackspaceHold {
		val stored = preferences(context)
		. getString(BACKSPACE_HOLD, null)
		return when (stored) {
			BackspaceHold.None.name -> BackspaceHold.None
			BackspaceHold.Repeat.name -> BackspaceHold.Repeat
			else -> BackspaceHold.Select
		}
	}

	fun setBackspaceHold(context: Context, mode: BackspaceHold) {
		preferences(context)
		. edit()
		. putString(BACKSPACE_HOLD, mode.name)
		. apply()
	}

	/**
	 * Horizontal travel, in dp, needed to advance one caret/selection step.
	 * Lower values make both swipe gestures faster. Default 20 dp.
	 */
	fun swipeSensitivity(context: Context): Int = (
		preferences(context)
		. getInt(SWIPE_SENSITIVITY, 20)
		. coerceIn(8, 40)
	)

	fun setSwipeSensitivity(context: Context, dpPerStep: Int) {
		preferences(context)
		. edit()
		. putInt(SWIPE_SENSITIVITY, dpPerStep.coerceIn(8, 40))
		. apply()
	}

	/** Height in dp of the bottom row holding the spacebar. Default 48 dp. */
	fun utilityRowHeight(context: Context): Int = (
		preferences(context)
		. getInt(UTILITY_ROW_HEIGHT, 48)
		. coerceIn(36, 72)
	)

	fun setUtilityRowHeight(context: Context, heightDp: Int) {
		preferences(context)
		. edit()
		. putInt(UTILITY_ROW_HEIGHT, heightDp.coerceIn(36, 72))
		. apply()
	}

	/** User-created keyboard profiles, newest last. */
	fun profiles(context: Context): List<KeyboardProfile> = (
		KeyboardProfile.parseList(
			preferences(context)
			. getString(PROFILES, null),
		)
	)

	fun setProfiles(context: Context, profiles: List<KeyboardProfile>) {
		preferences(context)
		. edit()
		. putString(PROFILES, KeyboardProfile.listJson(profiles))
		. apply()
	}

	/** Id of the active profile, or null when none is configured. */
	fun activeProfileId(context: Context): String? = (
		preferences(context)
		. getString(ACTIVE_PROFILE, null)
	)

	fun setActiveProfileId(context: Context, profileId: String?) {
		val editor = preferences(context)
		. edit()
		if (profileId == null) {
			editor.remove(ACTIVE_PROFILE)
		} else {
			editor.putString(ACTIVE_PROFILE, profileId)
		}
		editor.apply()
	}

	/** The active profile, falling back to the first configured one. */
	fun activeProfile(context: Context): KeyboardProfile? {
		val profiles = profiles(context)
		val id = activeProfileId(context)
		return profiles.firstOrNull { it.id == id } ?: profiles.firstOrNull()
	}
}
