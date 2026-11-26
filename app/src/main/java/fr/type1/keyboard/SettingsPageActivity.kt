package fr.type1.keyboard

import android.app.Activity
import android.os.Bundle
import android.view.WindowManager
import android.widget.SeekBar
import android.widget.TextView

/**
 * Shared scaffold for the settings pages: a sticky title + test field header,
 * followed by a page-specific scrollable body.
 */
abstract class SettingsPageActivity : Activity() {

	protected abstract val contentLayout: Int
	protected abstract val headerTitle: Int

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)
		setContentView(contentLayout)
		window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN)
		findViewById<TextView>(R.id.settingsTitle).setText(headerTitle)
	}

	protected fun bindSeekBar(
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
}
