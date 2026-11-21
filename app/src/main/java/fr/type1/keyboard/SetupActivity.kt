package fr.type1.keyboard

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import android.widget.Button

/** Onboarding: enable the IME, then pick it. */
class SetupActivity : Activity() {

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)
		setContentView(R.layout.activity_setup)

		findViewById<Button>(R.id.enableButton).setOnClickListener {
			startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
		}

		findViewById<Button>(R.id.selectButton).setOnClickListener {
			val imm = getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager
			imm.showInputMethodPicker()
		}
	}
}
