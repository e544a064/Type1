package fr.type1.keyboard.core;

/**
 * Host-side smoke test for the JNI contract, no Android device needed.
 *
 * It mirrors `app/src/main/java/fr/type1/keyboard/core/NativeBridge.kt` with
 * static methods, loads the host build of the core, and prints the payloads.
 *
 *   cd tools/jni-smoke
 *   ./run.ps1
 */
public class NativeBridge {

	public static native long nativeCreate();

	public static native void nativeDestroy(long handle);

	public static native void nativeReset(long handle);

	public static native void nativeSyncContext(long handle, String before, String after);

	public static native void nativeSetInputType(long handle, int inputType, int imeOptions);

	public static native void nativeSetImeOptions(long handle, int imeOptions);

	public static native String nativeLayouts();

	public static native String nativeLanguages();

	public static native boolean nativeSetProfile(long handle, String layoutId, String languages);

	public static native void nativeSetDefaultProfile(long handle);

	public static native String nativeRender(long handle);

	public static native String nativeKey(long handle, String key);

	public static void main(String[] args) {
		if (args.length != 1) {
			System.err.println("usage: NativeBridge <path-to-type1_core.dll|.so|.dylib>");
			System.exit(2);
		}
		System.load(args[0]);

		long handle = nativeCreate();
		// TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_CAP_SENTENCES
		nativeSetInputType(handle, 0x0000_4001, 0);

		System.out.println("initial row: " + nativeRender(handle));
		System.out.println("e        -> " + nativeKey(handle, "e"));
		System.out.println("c        -> " + nativeKey(handle, "c"));
		System.out.println("with suggestions: " + nativeRender(handle));
		System.out.println("sug:0    -> " + nativeKey(handle, "sug:0"));
		System.out.println("a        -> " + nativeKey(handle, "a"));
		System.out.println("shift    -> " + nativeKey(handle, "shift"));
		System.out.println("a        -> " + nativeKey(handle, "a"));
		System.out.println("space    -> " + nativeKey(handle, "space"));
		System.out.println("enter    -> " + nativeKey(handle, "enter"));
		System.out.println("backspace-> " + nativeKey(handle, "backspace"));
		System.out.println("alt:é    -> " + nativeKey(handle, "alt:é"));
		System.out.println("numbers  -> " + nativeKey(handle, "page"));
		System.out.println("next row : " + nativeRender(handle));

		nativeReset(handle);
		nativeSetInputType(handle, 0x0000_4001, 0);
		System.out.println("j        -> " + nativeKey(handle, "j"));
		System.out.println("a        -> " + nativeKey(handle, "a"));
		System.out.println("i        -> " + nativeKey(handle, "i"));
		System.out.println("space    -> " + nativeKey(handle, "space"));
		System.out.println("corrected: " + nativeRender(handle));
		System.out.println("backspace-> " + nativeKey(handle, "backspace"));
		System.out.println("reverted : " + nativeRender(handle));

		nativeReset(handle);
		nativeSetInputType(handle, 0x0000_4001, 0);
		nativeSyncContext(handle, "Ropp", "xjf");
		System.out.println("recorrection: " + nativeRender(handle));
		System.out.println("sug:0    -> " + nativeKey(handle, "sug:0"));

		nativeReset(handle);
		nativeSetInputType(handle, 0x0000_4001, 0);
		nativeSyncContext(handle, "Bonjour. ", "");
		System.out.println("caps     -> " + nativeKey(handle, "a"));

		nativeReset(handle);
		nativeSetInputType(handle, 0x0000_0001, 0);
		nativeSetImeOptions(handle, 0x0000_0003);
		System.out.println("search   -> " + nativeKey(handle, "enter"));
		System.out.println("layouts  -> " + nativeLayouts());
		System.out.println("langs    -> " + nativeLanguages());
		System.out.println("profile  -> " + nativeSetProfile(handle, "azerty", "fr"));
		nativeSetDefaultProfile(handle);
		System.out.println("default  -> " + nativeRender(handle));

		nativeDestroy(handle);
		System.out.println("OK");
	}
}
