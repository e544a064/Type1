package fr.type1.keyboard.model

import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

/**
 * A user-created keyboard profile: one layout plus the languages whose
 * dictionaries and correction rules the engine should use.
 */
data class KeyboardProfile(
	val id: String,
	val layoutId: String,
	val languageIds: List<String>,
) {
	/** Human label built from the core catalogs, e.g. `AZERTY · Français`. */
	fun label(layouts: List<CatalogEntry>, languages: List<CatalogEntry>): String {
		val layout = layouts.firstOrNull { it.id == layoutId }?.label ?: layoutId
		val names = languageIds.mapNotNull { id ->
			languages.firstOrNull { it.id == id }?.label
		}
		return if (names.isEmpty()) layout else "$layout · ${names.joinToString(" + ")}"
	}

	companion object {
		fun create(layoutId: String, languageIds: List<String>): KeyboardProfile =
			KeyboardProfile(UUID.randomUUID().toString(), layoutId, languageIds)

		fun parseList(json: String?): List<KeyboardProfile> {
			if (json.isNullOrBlank()) return emptyList()
			val array = JSONArray(json)
			return List(array.length()) { index ->
				val item = array.getJSONObject(index)
				KeyboardProfile(
					id = item.getString("id"),
					layoutId = item.getString("layout"),
					languageIds = item.stringList("languages"),
				)
			}
		}

		fun listJson(profiles: List<KeyboardProfile>): String {
			val array = JSONArray()
			for (profile in profiles) {
				val item = JSONObject()
				item.put("id", profile.id)
				item.put("layout", profile.layoutId)
				item.put("languages", JSONArray(profile.languageIds))
				array.put(item)
			}
			return array.toString()
		}
	}
}
