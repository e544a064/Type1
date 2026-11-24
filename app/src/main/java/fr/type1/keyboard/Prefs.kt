package fr.type1.keyboard

import android.content.Context
import android.content.SharedPreferences
import fr.type1.keyboard.model.KeyboardProfile
import org.json.JSONArray

/** What a long press on the backspace key does. */
enum class BackspaceHold { None, Repeat, Select }

/** Small preference store shared by the IME and the setup screen. */
object Prefs {
	private const val PREFS = "type1"
	private const val SPACE_CURSOR = "space_cursor"
	private const val BACKSPACE_HOLD = "backspace_hold"
	private const val SWIPE_SENSITIVITY = "swipe_sensitivity"
	private const val UTILITY_ROW_HEIGHT = "utility_row_height"
	private const val RECENT_EMOJIS = "recent_emojis"
	const val PROFILES = "profiles"
	const val ACTIVE_PROFILE = "active_profile"
	private const val MAX_RECENT_EMOJIS = 30

	fun preferences(context: Context): SharedPreferences {
		return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
	}

	private inline fun edit(context: Context, block: SharedPreferences.Editor.() -> Unit) {
		preferences(context).edit().apply(block).apply()
	}

	/** Drag horizontally on the spacebar to move the caret. Enabled by default. */
	fun spaceCursor(context: Context): Boolean = preferences(context).getBoolean(SPACE_CURSOR, true)

	fun setSpaceCursor(context: Context, enabled: Boolean) {
		edit(context) { putBoolean(SPACE_CURSOR, enabled) }
	}

	/** Long-press behaviour of the backspace key. Selects text by default. */
	fun backspaceHold(context: Context): BackspaceHold {
		val stored = (
			preferences(context)
			. getString(BACKSPACE_HOLD, null)
		)
		return when (stored) {
			BackspaceHold.None.name -> BackspaceHold.None
			BackspaceHold.Repeat.name -> BackspaceHold.Repeat
			else -> BackspaceHold.Select
		}
	}

	fun setBackspaceHold(context: Context, mode: BackspaceHold) {
		edit(context) { putString(BACKSPACE_HOLD, mode.name) }
	}

	/**
	 * Horizontal travel, in dp, needed to advance one caret/selection step.
	 * Lower values make both swipe gestures faster. Default 20 dp.
	 */
	fun swipeSensitivity(context: Context): Int {
		return preferences(context).getInt(SWIPE_SENSITIVITY, 20).coerceIn(8, 40)
	}

	fun setSwipeSensitivity(context: Context, dpPerStep: Int) {
		edit(context) { putInt(SWIPE_SENSITIVITY, dpPerStep.coerceIn(8, 40)) }
	}

	/** Height in dp of the bottom row holding the spacebar. Default 48 dp. */
	fun utilityRowHeight(context: Context): Int {
		return preferences(context).getInt(UTILITY_ROW_HEIGHT, 48).coerceIn(36, 72)
	}

	fun setUtilityRowHeight(context: Context, heightDp: Int) {
		edit(context) { putInt(UTILITY_ROW_HEIGHT, heightDp.coerceIn(36, 72)) }
	}

	/** User-created keyboard profiles, newest last. */
	fun profiles(context: Context): List<KeyboardProfile> {
		return KeyboardProfile.parseList(preferences(context).getString(PROFILES, null))
	}

	fun setProfiles(context: Context, profiles: List<KeyboardProfile>) {
		edit(context) { putString(PROFILES, KeyboardProfile.listJson(profiles)) }
	}

	/** Id of the active profile, or null when none is configured. */
	fun activeProfileId(context: Context): String? {
		return preferences(context).getString(ACTIVE_PROFILE, null)
	}

	fun setActiveProfileId(context: Context, profileId: String?) {
		edit(context) {
			if (profileId == null) remove(ACTIVE_PROFILE) else putString(ACTIVE_PROFILE, profileId)
		}
	}

	/** The active profile, falling back to the first configured one. */
	fun activeProfile(context: Context): KeyboardProfile? {
		val profiles = profiles(context)
		val id = activeProfileId(context)
		return profiles.firstOrNull { it.id == id } ?: profiles.firstOrNull()
	}

	/** Recently inserted emoji, newest first. */
	fun recentEmojis(context: Context): List<String> {
		val raw = preferences(context).getString(RECENT_EMOJIS, null) ?: return emptyList()
		return try {
			val array = JSONArray(raw)
			List(array.length()) { array.getString(it) }
		} catch (_: Exception) {
			emptyList()
		}
	}

	fun addRecentEmoji(context: Context, emoji: String) {
		val recent = buildList {
			add(emoji)
			addAll(recentEmojis(context).filterNot { it == emoji })
		}
		edit(context) {
			putString(RECENT_EMOJIS, JSONArray(recent.take(MAX_RECENT_EMOJIS)).toString())
		}
	}
}
