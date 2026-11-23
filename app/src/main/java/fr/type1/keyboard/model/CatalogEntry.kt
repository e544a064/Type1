package fr.type1.keyboard.model

import org.json.JSONArray

/** A selectable layout or language entry from the core catalog. */
data class CatalogEntry(val id: String, val label: String) {
	companion object {
		fun parseList(json: String): List<CatalogEntry> {
			if (json.isBlank()) return emptyList()
			val array = JSONArray(json)
			return List(array.length()) { index ->
				val item = array.getJSONObject(index)
				CatalogEntry(item.getString("id"), item.getString("label"))
			}
		}
	}
}
