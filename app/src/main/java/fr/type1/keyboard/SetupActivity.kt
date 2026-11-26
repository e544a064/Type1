package fr.type1.keyboard

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import android.widget.Button

/** Setup page: enable the IME and pick it. */
class SetupActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_setup
	override val headerTitle: Int = R.string.setup_title

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		findViewById<Button>(R.id.enableButton).setOnClickListener {
			startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
		}

		findViewById<Button>(R.id.selectButton).setOnClickListener {
			val imm = getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager
			imm.showInputMethodPicker()
		}
	}
}
