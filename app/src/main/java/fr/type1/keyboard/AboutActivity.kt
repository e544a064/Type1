package fr.type1.keyboard

import android.app.AlertDialog
import android.os.Bundle
import android.widget.TextView

/** About page: dictionary credits and clickable licence notices. */
class AboutActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_about
	override val headerTitle: Int = R.string.settings_about

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		findViewById<TextView>(R.id.licenseLexique).setOnClickListener {
			showLicense(R.string.settings_license_lexique, R.raw.license_lexique)
		}
		findViewById<TextView>(R.id.licenseWordfreq).setOnClickListener {
			showLicense(R.string.settings_license_wordfreq, R.raw.license_wordfreq)
		}
		findViewById<TextView>(R.id.licenseKoreanConjugations).setOnClickListener {
			showLicense(
				R.string.settings_license_korean_conjugations,
				R.raw.license_korean_conjugations,
			)
		}
		findViewById<TextView>(R.id.licenseType1).setOnClickListener {
			showLicense(R.string.settings_license_app, R.raw.license_type1)
		}
	}

	private fun showLicense(title: Int, raw: Int) {
		val text = (
			resources
			. openRawResource(raw)
			. bufferedReader()
			. use { it.readText() }
		)
		AlertDialog.Builder(this)
		. setTitle(title)
		. setMessage(text)
		. setPositiveButton(android.R.string.ok, null)
		. show()
	}
}
