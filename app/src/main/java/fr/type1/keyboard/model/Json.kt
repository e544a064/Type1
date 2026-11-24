package fr.type1.keyboard.model

import org.json.JSONObject

/** Read a JSON array of strings, treating a missing array as empty. */
internal fun JSONObject.stringList(name: String): List<String> {
	val array = optJSONArray(name) ?: return emptyList()
	return List(array.length()) { array.getString(it) }
}
