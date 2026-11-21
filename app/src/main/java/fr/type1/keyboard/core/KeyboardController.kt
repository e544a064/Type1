package fr.type1.keyboard.core

import android.view.inputmethod.EditorInfo
import fr.type1.keyboard.model.KeyAction
import fr.type1.keyboard.model.RowModel

/** Owns the native engine handle and decodes its JSON payloads. */
class KeyboardController {

	private val handle: Long = NativeBridge.nativeCreate()

	fun onStartInput(info: EditorInfo?) {
		// No editor info means "unknown text field": text class, no caps flag.
		NativeBridge.nativeSetInputType(
			handle,
			info?.inputType ?: 0x0000_0001,
			info?.imeOptions ?: 0,
		)
	}

	fun reset() = NativeBridge.nativeReset(handle)

	fun render(): RowModel = RowModel.parse(NativeBridge.nativeRender(handle))

	fun press(keyId: String): List<KeyAction> =
	KeyAction.parseList(NativeBridge.nativeKey(handle, keyId))

	fun close() = NativeBridge.nativeDestroy(handle)
}
