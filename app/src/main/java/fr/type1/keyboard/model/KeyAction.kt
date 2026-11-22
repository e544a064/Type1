package fr.type1.keyboard.model

import org.json.JSONArray

/** Something to do with the editor, decided by the Rust core. */
sealed class KeyAction {

	data class Commit(val text: String) : KeyAction()

	/** Commit a suggestion plus its convenience space, avoiding a duplicate space. */
	data class CommitSuggestion(val text: String) : KeyAction()

	data class DeleteWord(val word: Boolean) : KeyAction()

	data class DeleteChars(val count: Int) : KeyAction()

	/** Replace the word around the caret, with [before]/[after] characters on each side. */
	data class ReplaceSurrounding(val before: Int, val after: Int, val text: String) : KeyAction()

	/** Move the caret by [delta] characters; negative moves left. */
	data class Cursor(val delta: Int) : KeyAction()

	/** Extend the selection by [delta] characters; negative extends left. */
	data class ExtendSelection(val delta: Int) : KeyAction()

	object StartSelection : KeyAction()

	object DeleteSelection : KeyAction()

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
					"suggest" -> out += CommitSuggestion(item.optString("v"))
					"delete" -> {
						if (item.has("n")) {
							out += DeleteChars(item.optInt("n", 0))
						} else {
							out += DeleteWord(item.optBoolean("word", false))
						}
					}
					"cursor" -> out += Cursor(item.optInt("v", 0))
					"replace" -> out += ReplaceSurrounding(
						item.optInt("b", 0),
						item.optInt("a", 0),
						item.optString("v", ""),
					)
					"select" -> out += ExtendSelection(item.optInt("v", 0))
					"startSelection" -> out += StartSelection
					"deleteSelection" -> out += DeleteSelection
					"ime" -> out += Ime(item.optString("v", "none"))
				}
			}
			return out
		}
	}
}
