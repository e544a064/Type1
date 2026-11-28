package fr.type1.keyboard

import android.os.Bundle

/** Appearance settings: suggestion, letter and space row heights. */
class AppearanceActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_appearance
	override val headerTitle: Int = R.string.settings_appearance

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		bindSeekBar(
			R.id.suggestionHeightSeek,
			R.id.suggestionHeightValue,
			R.string.setup_height_value,
			Prefs.SUGGESTION_HEIGHT_RANGE,
			Prefs.suggestionRowHeight(this),
		) { Prefs.setSuggestionRowHeight(this, it) }

		bindSeekBar(
			R.id.letterHeightSeek,
			R.id.letterHeightValue,
			R.string.setup_height_value,
			Prefs.LETTER_HEIGHT_RANGE,
			Prefs.letterRowHeight(this),
		) { Prefs.setLetterRowHeight(this, it) }

		bindSeekBar(
			R.id.utilityHeightSeek,
			R.id.utilityHeightValue,
			R.string.setup_height_value,
			Prefs.UTILITY_HEIGHT_RANGE,
			Prefs.utilityRowHeight(this),
		) { Prefs.setUtilityRowHeight(this, it) }
	}
}
