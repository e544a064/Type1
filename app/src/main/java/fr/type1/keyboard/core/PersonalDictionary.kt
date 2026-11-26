package fr.type1.keyboard.core

import android.content.Context
import android.provider.UserDictionary

internal data class PersonalDictionaryEntry(
	val word: String,
	val locale: String?,
	val frequency: Int,
)

/**
 * Read the system personal dictionary. Android only exposes this provider to
 * enabled input methods and spell checkers, so failures are treated as “no
 * personal words” rather than surfacing an error to the user.
 *
 * Personal words containing whitespace or hyphens are skipped for now: the
 * core tokenizer treats those as separate tokens.
 */
internal object PersonalDictionary {

	fun load(context: Context): List<PersonalDictionaryEntry> = runCatching {
		val projection = arrayOf(
			UserDictionary.Words.WORD,
			UserDictionary.Words.LOCALE,
			UserDictionary.Words.FREQUENCY,
		)
		val entries = mutableListOf<PersonalDictionaryEntry>()
		context
		. contentResolver
		. query(
			UserDictionary.Words.CONTENT_URI,
			projection,
			null,
			null,
			null,
		)
		?. use { cursor ->
			val wordIndex = cursor.getColumnIndexOrThrow(UserDictionary.Words.WORD)
			val localeIndex = cursor.getColumnIndexOrThrow(UserDictionary.Words.LOCALE)
			val frequencyIndex = cursor.getColumnIndexOrThrow(UserDictionary.Words.FREQUENCY)
			while (cursor.moveToNext()) {
				val word = cursor.getString(wordIndex)?.trim().orEmpty()
				if (word.isEmpty() || word.any { it.isWhitespace() || it == '-' }) {
					continue
				}
				val locale = cursor.getString(localeIndex)?.trim().orEmpty().ifEmpty { null }
				val frequency = cursor.getInt(frequencyIndex).coerceIn(0, 255)
				entries.add(PersonalDictionaryEntry(word, locale, frequency))
			}
		}
		entries
	}.getOrDefault(emptyList())
}
