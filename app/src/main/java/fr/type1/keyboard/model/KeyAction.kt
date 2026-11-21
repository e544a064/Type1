package fr.type1.keyboard.model

import org.json.JSONArray

/** Something to do with the editor, decided by the Rust core. */
sealed class KeyAction {

	data class Commit(val text: String) : KeyAction()

	data class DeleteWord(val word: Boolean) : KeyAction()

	data class DeleteChars(val count: Int) : KeyAction()

	data class Ime(val action: String) : KeyAction()

	companion object {
		fun parseList(json: String): List<KeyAction> {
			if (json.isBlank()) return emptyList()
			val array = JSONArray(json)
			val out = ArrayList<KeyAction>(array.length())
			for (i in 0 until array.length()) {
				val item = array.getJSONObject(i)
				when (item.optString("t")) {
					"commit" -> out += Commit(item.optString("v"))
					"delete" -> {
						if (item.has("n")) {
							out += DeleteChars(item.optInt("n", 0))
						} else {
							out += DeleteWord(item.optBoolean("word", false))
						}
					}
					"ime" -> out += Ime(item.optString("v", "none"))
				}
			}
			return out
		}
	}
}
