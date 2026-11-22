package fr.type1.keyboard

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.view.inputmethod.InputMethodManager
import android.widget.AdapterView
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.SeekBar
import android.widget.Spinner
import android.widget.Switch
import android.widget.TextView

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
				Prefs.setBackspaceHold(this@SetupActivity, BackspaceHold.entries[position])
			}

			override fun onNothingSelected(parent: AdapterView<*>?) = Unit
		}

		val swipeSeek = findViewById<SeekBar>(R.id.swipeSensitivitySeek)
		val swipeValue = findViewById<TextView>(R.id.swipeSensitivityValue)
		swipeSeek.max = 32
		val initialSensitivity = Prefs.swipeSensitivity(this)
		swipeSeek.progress = initialSensitivity - 8
		swipeValue.text = getString(R.string.setup_swipe_value, initialSensitivity)
		swipeSeek.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
			override fun onProgressChanged(bar: SeekBar?, progress: Int, fromUser: Boolean) {
				val dp = progress + 8
				swipeValue.text = getString(R.string.setup_swipe_value, dp)
				if (fromUser) Prefs.setSwipeSensitivity(this@SetupActivity, dp)
			}

			override fun onStartTrackingTouch(bar: SeekBar?) = Unit

			override fun onStopTrackingTouch(bar: SeekBar?) = Unit
		})

		val heightSeek = findViewById<SeekBar>(R.id.utilityHeightSeek)
		val heightValue = findViewById<TextView>(R.id.utilityHeightValue)
		heightSeek.max = 36
		val initialHeight = Prefs.utilityRowHeight(this)
		heightSeek.progress = initialHeight - 36
		heightValue.text = getString(R.string.setup_height_value, initialHeight)
		heightSeek.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
			override fun onProgressChanged(bar: SeekBar?, progress: Int, fromUser: Boolean) {
				val dp = progress + 36
				heightValue.text = getString(R.string.setup_height_value, dp)
				if (fromUser) Prefs.setUtilityRowHeight(this@SetupActivity, dp)
			}

			override fun onStartTrackingTouch(bar: SeekBar?) = Unit

			override fun onStopTrackingTouch(bar: SeekBar?) = Unit
		})
	}
}
