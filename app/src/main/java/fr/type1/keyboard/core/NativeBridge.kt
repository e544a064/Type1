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

	/** Start loading the active dictionaries in the background. */
	external fun nativePrepare(handle: Long)

	external fun nativeDestroy(handle: Long)

	external fun nativeReset(handle: Long)

	external fun nativeSelectionChanged(handle: Long)

	external fun nativeSyncContext(handle: Long, before: String, after: String)

	external fun nativeSetInputType(handle: Long, inputType: Int, imeOptions: Int)

	external fun nativeSetImeOptions(handle: Long, imeOptions: Int)

	external fun nativeSetPersonalWords(
		handle: Long,
		words: Array<String>,
		locales: Array<String>,
		frequencies: IntArray,
	)

	/** JSON catalog of selectable layouts. */
	external fun nativeLayouts(): String

	/** JSON catalog of selectable languages. */
	external fun nativeLanguages(): String

	/** Layout id plus comma-separated language ids. */
	external fun nativeSetProfile(handle: Long, layoutId: String, languages: String): Boolean

	external fun nativeSetDefaultProfile(handle: Long)

	/** JSON row model: keys, weights, page, shift state. */
	external fun nativeRender(handle: Long): String

	/** JSON action list for one key press. */
	external fun nativeKey(handle: Long, keyId: String): String
}
