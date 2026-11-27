package fr.type1.keyboard

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView

/** Main settings hub: a list of settings pages. */
class SettingsActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_settings
	override val headerTitle: Int = R.string.app_name
	override val headerSubtitle: Int = R.string.setup_subtitle

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)
		val rows = findViewById<LinearLayout>(R.id.settingsRows)
		addRow(
			rows,
			R.drawable.ic_settings_setup,
			R.string.setup_title,
			R.string.settings_setup_summary,
		) { startActivity(Intent(this, SetupActivity::class.java)) }
		addRow(
			rows,
			R.drawable.ic_settings_profiles,
			R.string.profiles_title,
			R.string.settings_profiles_summary,
		) { startActivity(Intent(this, ProfilesActivity::class.java)) }
		addSpacer(rows)
		addRow(
			rows,
			R.drawable.ic_settings_behaviour,
			R.string.settings_behaviour,
			R.string.settings_behaviour_summary,
		) { startActivity(Intent(this, BehaviourActivity::class.java)) }
		addRow(
			rows,
			R.drawable.ic_settings_appearance,
			R.string.settings_appearance,
			R.string.settings_appearance_summary,
		) { startActivity(Intent(this, AppearanceActivity::class.java)) }
		addRow(
			rows,
			R.drawable.ic_settings_about,
			R.string.settings_about,
			R.string.settings_about_summary,
		) { startActivity(Intent(this, AboutActivity::class.java)) }
		addSpacer(rows)
		addRow(
			rows,
			R.drawable.ic_settings_dictionary,
			R.string.settings_personal_dictionary,
			R.string.settings_personal_dictionary_summary,
		) {
			runCatching {
				startActivity(Intent(Settings.ACTION_USER_DICTIONARY_SETTINGS))
			}
		}
	}

	private fun addRow(
		container: LinearLayout,
		icon: Int,
		title: Int,
		summary: Int,
		onClick: () -> Unit,
	) {
		val row = layoutInflater.inflate(R.layout.settings_row, container, false)
		row.findViewById<ImageView>(R.id.rowIcon).setImageResource(icon)
		row.findViewById<TextView>(R.id.rowTitle).setText(title)
		row.findViewById<TextView>(R.id.rowSummary).setText(summary)
		row.setOnClickListener { onClick() }
		container.addView(row)
		addDivider(container)
	}

	private fun addDivider(container: LinearLayout) {
		val divider = View(this)
		divider.setBackgroundColor(getColor(R.color.settings_divider))
		val params = LinearLayout.LayoutParams(
			LinearLayout.LayoutParams.MATCH_PARENT,
			dp(1),
		)
		params.marginStart = dp(84)
		params.marginEnd = dp(24)
		container.addView(divider, params)
	}

	private fun addSpacer(container: LinearLayout) {
		container.addView(
			View(this),
			LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(12)),
		)
	}

	private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
