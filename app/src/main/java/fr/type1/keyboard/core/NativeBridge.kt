package fr.type1.keyboard.core

/**
 * Thin JNI façade over the Rust core (`core/src/ffi.rs`).
 *
 * The core owns layout, pages, shift and French spacing rules. This class only
 * marshals strings: the row is rendered as JSON, key presses come back as a
 * JSON action list.
 */
internal object NativeBridge {

	init {
		System.loadLibrary("type1_core")
	}

	external fun nativeCreate(): Long

	external fun nativeDestroy(handle: Long)

	external fun nativeReset(handle: Long)

	external fun nativeSelectionChanged(handle: Long)

	external fun nativeSetInputType(handle: Long, inputType: Int, imeOptions: Int)

	/** JSON row model: keys, weights, page, shift state. */
	external fun nativeRender(handle: Long): String

	/** JSON action list for one key press. */
	external fun nativeKey(handle: Long, keyId: String): String
}
