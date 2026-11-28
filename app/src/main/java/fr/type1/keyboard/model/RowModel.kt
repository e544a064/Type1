package fr.type1.keyboard.model

import org.json.JSONObject

/** One character in the middle band. */
data class LetterKey(
	val id: String,
	val label: String,
	/** Text this key commits when tapped (use this for its popup). */
	val commit: String,
	val letter: Boolean,
	/** Physical keyboard row: 0 top, 1 home, 2 bottom. */
	val row: Int,
	val alts: List<String>,
)

/** One key of the bottom band. */
data class UtilityKey(
	val id: String,
	val label: String,
	val kind: String,
	val weight: Float,
	val active: Boolean,
	/** Vector icon name, or "" to draw [label] as text. */
	val icon: String,
	/** Extra symbols offered by a long press. */
	val alts: List<String> = emptyList(),
)

/** Suggestions are addressed as `sug:0`, `sug:1`, ... by the core. */
const val SUGGESTION_PREFIX = "sug:"

/** The whole keyboard model: suggestions, letters, utility keys. */
data class RowModel(
	val page: String,
	val shift: Int,
	/** True when the next letter will be uppercase (shift or auto-capitals). */
	val caps: Boolean,
	/** Whether a letter popup may also show neighbouring keys' alternates. */
	val mergeLetterAlts: Boolean,
	val suggestions: List<String>,
	val letters: List<LetterKey>,
	/** Shift (left) and backspace (right) pinned to the suggestion strip. */
	val strip: List<UtilityKey>,
	val keys: List<UtilityKey>,
) {
	companion object {
		fun parse(json: String): RowModel {
			val root = JSONObject(json)

			val lettersArray = root.getJSONArray("letters")
			val letters = List(lettersArray.length()) { index ->
				val letter = lettersArray.getJSONObject(index)
				LetterKey(
					id = letter.getString("id"),
					label = letter.optString("label", letter.getString("id")),
					commit = letter.optString("commit", letter.getString("id")),
					letter = letter.optBoolean("letter", false),
					row = letter.optInt("row", 1),
					alts = letter.stringList("alts"),
				)
			}

			return RowModel(
				page = root.optString("page", "letters"),
				shift = root.optInt("shift", 0),
				caps = root.optBoolean("caps", false),
				mergeLetterAlts = root.optBoolean("mergeLetterAlts", true),
				suggestions = root.stringList("suggestions"),
				letters = letters,
				strip = root.utilityList("strip"),
				keys = root.utilityList("keys"),
			)
		}

		private fun JSONObject.utilityList(name: String): List<UtilityKey> {
			val array = optJSONArray(name) ?: return emptyList()
			return List(array.length()) { index ->
				val key = array.getJSONObject(index)
				UtilityKey(
					id = key.getString("id"),
					label = key.optString("label"),
					kind = key.optString("kind", "char"),
					weight = key.optDouble("w", 1.0).toFloat(),
					active = key.optBoolean("on", false),
					icon = key.optString("icon", ""),
					alts = key.stringList("alts"),
				)
			}
		}
	}
}
