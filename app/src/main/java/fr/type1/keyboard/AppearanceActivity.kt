package fr.type1.keyboard

import android.os.Bundle

/** Appearance settings: utility row height. */
class AppearanceActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_appearance
	override val headerTitle: Int = R.string.settings_appearance

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		bindSeekBar(
			R.id.utilityHeightSeek,
			R.id.utilityHeightValue,
			R.string.setup_height_value,
			36..72,
			Prefs.utilityRowHeight(this),
		) { Prefs.setUtilityRowHeight(this, it) }
	}
}
