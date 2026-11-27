package fr.type1.keyboard

import android.os.Bundle

/** Appearance settings: utility row height. */
class AppearanceActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_appearance
	override val headerTitle: Int = R.string.settings_appearance

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		bindSeekBar(
			R.id.suggestionHeightSeek,
			R.id.suggestionHeightValue,
			R.string.setup_height_value,
			28..56,
			Prefs.suggestionRowHeight(this),
		) { Prefs.setSuggestionRowHeight(this, it) }

		bindSeekBar(
			R.id.letterHeightSeek,
			R.id.letterHeightValue,
			R.string.setup_height_value,
			44..80,
			Prefs.letterRowHeight(this),
		) { Prefs.setLetterRowHeight(this, it) }

		bindSeekBar(
			R.id.utilityHeightSeek,
			R.id.utilityHeightValue,
			R.string.setup_height_value,
			36..72,
			Prefs.utilityRowHeight(this),
		) { Prefs.setUtilityRowHeight(this, it) }
	}
}
