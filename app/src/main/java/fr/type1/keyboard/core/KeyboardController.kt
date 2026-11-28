package fr.type1.keyboard.core

import android.text.InputType
import android.view.inputmethod.EditorInfo
import fr.type1.keyboard.model.CatalogEntry
import fr.type1.keyboard.model.KeyAction
import fr.type1.keyboard.model.RowModel

/** Owns the native engine handle and decodes its JSON payloads. */
class KeyboardController {

	private val handle: Long = NativeBridge.nativeCreate()

	fun onStartInput(info: EditorInfo?) {
		// No editor info means "unknown text field": text class, no caps flag.
		NativeBridge.nativeSetInputType(
			handle,
			effectiveInputType(info),
			info?.imeOptions ?: 0,
		)
	}

	/**
	 * Many editors want sentence capitalisation but never request it. Default to
	 * it for plain text, and never for numeric, password, email, URI, name,
	 * address or filter fields.
	 */
	private fun effectiveInputType(info: EditorInfo?): Int {
		val type = info?.inputType ?: return InputType.TYPE_CLASS_TEXT
		val cls = type and InputType.TYPE_MASK_CLASS
		val caps = (
			InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS
			or InputType.TYPE_TEXT_FLAG_CAP_WORDS
			or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
		)
		if (cls != InputType.TYPE_CLASS_TEXT || type and caps != 0) return type
		val variation = type and InputType.TYPE_MASK_VARIATION
		val blocked = when (variation) {
			InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
			InputType.TYPE_TEXT_VARIATION_EMAIL_SUBJECT,
			InputType.TYPE_TEXT_VARIATION_URI,
			InputType.TYPE_TEXT_VARIATION_PASSWORD,
			InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
			InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
			InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
			InputType.TYPE_TEXT_VARIATION_PERSON_NAME,
			InputType.TYPE_TEXT_VARIATION_POSTAL_ADDRESS,
			InputType.TYPE_TEXT_VARIATION_FILTER -> true
			else -> false
		}
		return if (blocked) type else type or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
	}

	fun reset() = NativeBridge.nativeReset(handle)

	fun prepare() = NativeBridge.nativePrepare(handle)

	fun setImeOptions(imeOptions: Int) = NativeBridge.nativeSetImeOptions(handle, imeOptions)

	internal fun setPersonalWords(entries: List<PersonalDictionaryEntry>) {
		NativeBridge.nativeSetPersonalWords(
			handle,
			entries.map { it.word }.toTypedArray(),
			entries.map { it.locale ?: "" }.toTypedArray(),
			entries.map { it.frequency }.toIntArray(),
		)
	}

	fun setProfile(layoutId: String, languageIds: List<String>): Boolean =
		NativeBridge.nativeSetProfile(handle, layoutId, languageIds.joinToString(","))

	fun setDefaultProfile() = NativeBridge.nativeSetDefaultProfile(handle)

	fun selectionChanged() = NativeBridge.nativeSelectionChanged(handle)

	fun syncContext(before: String, after: String) = NativeBridge.nativeSyncContext(handle, before, after)

	fun render(): RowModel = RowModel.parse(NativeBridge.nativeRender(handle))

	fun press(keyId: String): List<KeyAction> {
		return KeyAction.parseList(NativeBridge.nativeKey(handle, keyId))
	}

	fun close() = NativeBridge.nativeDestroy(handle)

	companion object {
		fun layouts(): List<CatalogEntry> = CatalogEntry.parseList(NativeBridge.nativeLayouts())

		fun languages(): List<CatalogEntry> = CatalogEntry.parseList(NativeBridge.nativeLanguages())
	}
}
