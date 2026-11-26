package fr.type1.keyboard

import android.os.Bundle
import android.view.View
import android.widget.AdapterView
import android.widget.ArrayAdapter
import android.widget.Spinner
import android.widget.Switch

/** Behaviour settings: spacebar drag, backspace hold and swipe sensitivity. */
class BehaviourActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_behaviour
	override val headerTitle: Int = R.string.settings_behaviour

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)

		val spaceCursor = findViewById<Switch>(R.id.spaceCursorSwitch)
		spaceCursor.isChecked = Prefs.spaceCursor(this)
		spaceCursor.setOnCheckedChangeListener { _, enabled ->
			Prefs.setSpaceCursor(this, enabled)
		}

		val backspaceHold = findViewById<Spinner>(R.id.backspaceHoldSpinner)
		backspaceHold.adapter = ArrayAdapter(
			this,
			android.R.layout.simple_spinner_item,
			listOf(
				getString(R.string.backspace_hold_none),
				getString(R.string.backspace_hold_repeat),
				getString(R.string.backspace_hold_select),
			),
		).apply {
			setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
		}
		backspaceHold.setSelection(Prefs.backspaceHold(this).ordinal, false)
		backspaceHold.onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
			override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) {
				Prefs.setBackspaceHold(this@BehaviourActivity, BackspaceHold.entries[position])
			}

			override fun onNothingSelected(parent: AdapterView<*>?) = Unit
		}

		bindSeekBar(
			R.id.swipeSensitivitySeek,
			R.id.swipeSensitivityValue,
			R.string.setup_swipe_value,
			8..40,
			Prefs.swipeSensitivity(this),
		) { Prefs.setSwipeSensitivity(this, it) }
	}
}
