package fr.type1.keyboard

import android.app.AlertDialog
import android.content.DialogInterface
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.model.CatalogEntry
import fr.type1.keyboard.model.KeyboardProfile

/** Create, edit and select the user's keyboard profiles. */
class ProfilesActivity : SettingsPageActivity() {

	override val contentLayout: Int = R.layout.activity_profiles
	override val headerTitle: Int = R.string.profiles_title

	private lateinit var layouts: List<CatalogEntry>
	private lateinit var languages: List<CatalogEntry>
	private lateinit var list: LinearLayout
	private lateinit var empty: TextView

	override fun onCreate(savedInstanceState: Bundle?) {
		super.onCreate(savedInstanceState)
		layouts = KeyboardController.layouts()
		languages = KeyboardController.languages()
		list = findViewById(R.id.profileList)
		empty = findViewById(R.id.profileEmpty)
		findViewById<Button>(R.id.addProfileButton).setOnClickListener {
			showEditor(null)
		}
		render()
	}

	private fun render() {
		val profiles = Prefs.profiles(this)
		var active = Prefs.activeProfileId(this)
		if (profiles.isEmpty()) {
			if (active != null) {
				Prefs.setActiveProfileId(this, null)
				active = null
			}
		} else if (profiles.none { it.id == active }) {
			active = profiles.first().id
			Prefs.setActiveProfileId(this, active)
		}
		list.removeAllViews()
		empty.visibility = if (profiles.isEmpty()) View.VISIBLE else View.GONE
		for ((index, profile) in profiles.withIndex()) {
			list.addView(profileRow(profile, profile.id == active))
			if (index < profiles.lastIndex) {
				val divider = View(this)
				divider.setBackgroundColor(0x1A11131A)
				list.addView(divider, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(1)))
			}
		}
	}

	private fun profileRow(profile: KeyboardProfile, active: Boolean): View {
		val row = LinearLayout(this)
		row.orientation = LinearLayout.HORIZONTAL
		row.gravity = Gravity.CENTER_VERTICAL
		row.setPadding(0, dp(10), 0, dp(10))
		row.isClickable = true
		row.setOnClickListener { select(profile) }

		val label = text(profile.label(layouts, languages), 16f, 0xFF11131A.toInt())
		row.addView(label, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))

		if (active) {
			val marker = text(getString(R.string.profiles_active), 12f, 0xFF4C6FFF.toInt())
			marker.setPadding(dp(8), 0, dp(4), 0)
			row.addView(marker)
		}

		row.addView(action(R.string.profiles_edit) { showEditor(profile) })
		row.addView(action(R.string.profiles_delete) { confirmDelete(profile) })
		return row
	}

	private fun action(label: Int, onClick: () -> Unit): TextView {
		return text(getString(label), 14f, 0xFF4C6FFF.toInt()).apply {
			setPadding(dp(10), dp(8), 0, dp(8))
			isClickable = true
			setOnClickListener { onClick() }
		}
	}

	private fun text(value: CharSequence, size: Float, color: Int): TextView {
		return TextView(this).apply {
			text = value
			textSize = size
			setTextColor(color)
		}
	}

	private fun select(profile: KeyboardProfile) {
		Prefs.setActiveProfileId(this, profile.id)
		render()
	}

	private fun showEditor(existing: KeyboardProfile?) {
		if (layouts.isEmpty() || languages.isEmpty()) return

		val container = LinearLayout(this)
		container.orientation = LinearLayout.VERTICAL
		container.setPadding(dp(20), dp(8), dp(20), 0)

		val layoutTitle = text(getString(R.string.profile_layout), 14f, 0xFF6B7285.toInt())
		container.addView(layoutTitle)

		val spinner = Spinner(this)
		spinner.adapter = ArrayAdapter(
			this,
			android.R.layout.simple_spinner_item,
			layouts.map { it.label },
		).apply {
			setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
		}
		spinner.setSelection(
			layouts.indexOfFirst { it.id == existing?.layoutId }.coerceAtLeast(0),
			false,
		)
		container.addView(spinner)

		val languageTitle = text(getString(R.string.profile_languages), 14f, 0xFF6B7285.toInt())
		languageTitle.setPadding(0, dp(16), 0, 0)
		container.addView(languageTitle)

		val toggles = LinkedHashMap<String, Switch>()
		for (language in languages) {
			val toggle = Switch(this)
			toggle.text = language.label
			toggle.textSize = 16f
			toggle.isChecked = existing?.languageIds?.contains(language.id)
				?: (language.id == languages.first().id)
			toggles[language.id] = toggle
			container.addView(toggle)
		}

		val dialog = (
			AlertDialog.Builder(this)
			. setTitle(if (existing == null) R.string.profile_new else R.string.profile_edit)
			. setView(container)
			. setPositiveButton(R.string.profile_save, null)
			. setNegativeButton(R.string.profile_cancel, null)
			. create()
		)

		dialog.setOnShowListener {
			dialog.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener {
				save(existing, spinner, toggles, dialog)
			}
		}
		dialog.show()
	}

	private fun save(
		existing: KeyboardProfile?,
		spinner: Spinner,
		toggles: Map<String, Switch>,
		dialog: AlertDialog,
	) {
		val languageIds = toggles.filterValues { it.isChecked }.keys.toList()
		if (languageIds.isEmpty()) {
			Toast.makeText(this, R.string.profile_need_language, Toast.LENGTH_SHORT).show()
			return
		}
		val layoutId = layouts[spinner.selectedItemPosition.coerceIn(0, layouts.lastIndex)].id
		val profiles = Prefs.profiles(this).toMutableList()
		val saved = if (existing == null) {
			val profile = KeyboardProfile.create(layoutId, languageIds)
			profiles.add(profile)
			profile
		} else {
			val index = profiles.indexOfFirst { it.id == existing.id }
			if (index < 0) return
			profiles[index] = existing.copy(layoutId = layoutId, languageIds = languageIds)
			profiles[index]
		}
		Prefs.setProfiles(this, profiles)
		if (Prefs.activeProfileId(this) == null) {
			Prefs.setActiveProfileId(this, saved.id)
		}
		dialog.dismiss()
		render()
	}

	private fun confirmDelete(profile: KeyboardProfile) {
		AlertDialog.Builder(this)
		. setTitle(R.string.profile_delete_title)
		. setMessage(getString(R.string.profile_delete_message, profile.label(layouts, languages)))
		. setPositiveButton(R.string.profile_delete_confirm) { _, _ -> delete(profile) }
		. setNegativeButton(R.string.profile_cancel, null)
		. show()
	}

	private fun delete(profile: KeyboardProfile) {
		val profiles = (
			Prefs.profiles(this)
			. filterNot { it.id == profile.id }
		)
		if (Prefs.activeProfileId(this) == profile.id) {
			Prefs.setActiveProfileId(this, profiles.firstOrNull()?.id)
		}
		Prefs.setProfiles(this, profiles)
		render()
	}

	private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
