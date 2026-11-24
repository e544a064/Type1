package fr.type1.keyboard.model

import org.json.JSONObject

/** One character in the middle band. */
data class LetterKey(
	val id: String,
	val label: String,
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
	val suggestions: List<String>,
	val letters: List<LetterKey>,
	/** Shift (left) and backspace (right) pinned to the suggestion strip. */
	val strip: List<UtilityKey>,
	val keys: List<UtilityKey>,
) {
	companion object {
		fun parse(json: String): RowModel {
			val root = JSONObject(json)

			val suggestionsArray = root.optJSONArray("suggestions")
			val suggestions = if (suggestionsArray == null) {
				emptyList()
			} else {
				List(suggestionsArray.length()) { suggestionsArray.getString(it) }
			}

			val lettersArray = root.getJSONArray("letters")
			val letters = ArrayList<LetterKey>(lettersArray.length())
			for (i in 0 until lettersArray.length()) {
				val letter = lettersArray.getJSONObject(i)
				val altsArray = letter.optJSONArray("alts")
				val alts = if (altsArray == null) {
					emptyList()
				} else {
					List(altsArray.length()) { altsArray.getString(it) }
				}
				letters += LetterKey(
					id = letter.getString("id"),
					label = letter.optString("label", letter.getString("id")),
					letter = letter.optBoolean("letter", false),
					row = letter.optInt("row", 1),
					alts = alts,
				)
			}

			val keysArray = root.getJSONArray("keys")
			val keys = ArrayList<UtilityKey>(keysArray.length())
			for (i in 0 until keysArray.length()) {
				val key = keysArray.getJSONObject(i)
				val altsArray = key.optJSONArray("alts")
				val alts = if (altsArray == null) {
					emptyList()
				} else {
					List(altsArray.length()) { altsArray.getString(it) }
				}
				keys += UtilityKey(
					id = key.getString("id"),
					label = key.optString("label"),
					kind = key.optString("kind", "char"),
					weight = key.optDouble("w", 1.0).toFloat(),
					active = key.optBoolean("on", false),
					icon = key.optString("icon", ""),
					alts = alts,
				)
			}

			val stripArray = root.optJSONArray("strip")
			val strip = if (stripArray == null) {
				emptyList()
			} else {
				List(stripArray.length()) { i ->
					val key = stripArray.getJSONObject(i)
					UtilityKey(
						id = key.getString("id"),
						label = key.optString("label"),
						kind = key.optString("kind", "char"),
						weight = 1f,
						active = key.optBoolean("on", false),
						icon = key.optString("icon", ""),
					)
				}
			}

			return RowModel(
				page = root.optString("page", "letters"),
				shift = root.optInt("shift", 0),
				caps = root.optBoolean("caps", false),
				suggestions = suggestions,
				letters = letters,
				strip = strip,
				keys = keys,
			)
		}
	}
}
