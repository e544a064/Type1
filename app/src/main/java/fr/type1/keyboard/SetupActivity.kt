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
import fr.type1.keyboard.core.KeyboardController

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

		findViewById<Button>(R.id.profilesButton).setOnClickListener {
			startActivity(Intent(this, ProfilesActivity::class.java))
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

		bindSeekBar(
			R.id.swipeSensitivitySeek,
			R.id.swipeSensitivityValue,
			R.string.setup_swipe_value,
			8..40,
			Prefs.swipeSensitivity(this),
		) { Prefs.setSwipeSensitivity(this, it) }

		bindSeekBar(
			R.id.utilityHeightSeek,
			R.id.utilityHeightValue,
			R.string.setup_height_value,
			36..72,
			Prefs.utilityRowHeight(this),
		) { Prefs.setUtilityRowHeight(this, it) }
	}

	private fun bindSeekBar(
		seekId: Int,
		valueId: Int,
		labelId: Int,
		range: IntRange,
		initial: Int,
		onValue: (Int) -> Unit,
	) {
		val seek = findViewById<SeekBar>(seekId)
		val value = findViewById<TextView>(valueId)
		val offset = range.first
		seek.max = range.last - offset
		seek.progress = initial - offset
		value.text = getString(labelId, initial)
		seek.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
			override fun onProgressChanged(bar: SeekBar?, progress: Int, fromUser: Boolean) {
				val current = progress + offset
				value.text = getString(labelId, current)
				if (fromUser) onValue(current)
			}

			override fun onStartTrackingTouch(bar: SeekBar?) = Unit

			override fun onStopTrackingTouch(bar: SeekBar?) = Unit
		})
	}

	override fun onResume() {
		super.onResume()
		val profile = Prefs.activeProfile(this)
		val summary = findViewById<TextView>(R.id.activeProfileValue)
		summary.text = if (profile == null) {
			getString(R.string.setup_profiles_none)
		} else {
			profile.label(
				KeyboardController.layouts(),
				KeyboardController.languages(),
			)
		}
	}
}
