# Type1 — a one-line AZERTY Android keyboard

Every letter sits on **one horizontal line**, between a suggestion strip and a
row of utility keys: ~154 dp of screen instead of the usual ~250 dp. The typing
logic is written in **Rust** and shared through a tiny JNI bridge; Android only
draws the three bands and applies editor actions. That split is deliberate: an
iOS port later only needs a new UI + FFI shim.

```
┌──────────────────────────────────────────────────────────────┐
│ ⇧        best     very     beat     nest            ⌫        │  38 dp
├──────────────────────────────────────────────────────────────┤
│ A q w Z s x E d c R f v T g b Y h n U j I k O l P m          │  56 dp
├──────────────────────────────────────────────────────────────┤
│ 123          espace                    ,              ⏎      │  48 dp
└──────────────────────────────────────────────────────────────┘
```

The middle band has **no key boxes and no groups**: 26 evenly spaced letters in
AZERTY column order (`A Q W`, `Z S X`, ...). All glyphs share one size and one
case. Each letter is shifted vertically according to its physical row — top-row
letters higher, bottom-row letters lower — so the original layout stays legible.
The row is lowercase normally and uppercase while shift (or auto-capitalisation)
is active. Taps resolve by horizontal position; you can drag along the strip and
release on the letter you want.

## Status: MVP

- All 26 letters on one line in AZERTY column order, row-based vertical offsets,
  uniform size and case (uppercase while shift/auto-caps is active).
- Numbers and symbols take over the letter line (`123`, `#+=`).
- Suggestion strip with accent-insensitive prefix lookup from an embedded French
  lexicon: type `ecole`, get `École`, tap to replace the word in one action.
- French accents on long-press (`é è ê ë à â ç ù û î ï ô ö œ ÿ ñ`), plus `’`.
- Shift (tap once = one capital, twice = caps lock), auto-capitalisation from
  `inputType` (`CAP_SENTENCES`, `CAP_WORDS`, `CAP_CHARACTERS`).
- French spacing: space before `? ! : ;`, automatic space after `. , ; : ! ?`
  when a new word starts.
- Backspace with hold-to-repeat; enter adapts to the editor action
  (`→`, `✓`, `🔍`, `➤`) and inserts a newline in multiline fields.
- Drag along the letter line to correct a press before releasing.
- Shift and backspace sit at the ends of the suggestion strip; the bottom row is
  just page, space, comma and enter.
- Sounds, haptics, pressed-letter highlight.

Not in the MVP: autocorrect, next-word prediction, glide typing, emoji panel,
clipboard manager, voice input, themes, settings screen, physical keyboard.

## Why one line?

26 letters on a 360 dp phone give each letter ~11 dp of width. That is far below
the 48 dp accessibility guideline, and it is the central trade-off of the
design: one line saves ~100 dp of height, and the suggestion strip is what makes
it workable. Three things compensate:

- nearest-letter hit testing (the whole strip is one target area),
- drag-to-correct: press, slide, release on the right letter,
- suggestions that fix whole words at once (`ecole` → `École`).

Layout is data, not code: `core/src/layout.rs` lists the letters in AZERTY
column order with a `row` index (0 top, 1 home, 2 bottom). Change the order or
the offsets there and the UI follows.

## Sizing

The three bands are 38 dp (suggestions), 56 dp (letters) and 48 dp (utilities),
plus 3 dp gaps: **154 dp** total. On a 411 dp-wide phone a letter is ~12.5 dp
wide; on 360 dp it is ~11 dp. Tuning knobs:

- `suggestionHeight`, `letterHeight`, `utilityHeight` in `KeyboardView.kt`,
- the `rowOffset` values (±7 dp) in `KeyboardView.kt`,
- letter order, row indices and utility weights in `core/src/layout.rs`,
- the `WORDS` list in `core/src/lexicon.rs`.

## Repository layout

```
core/                     Rust crate (no Android dependency)
  src/layout.rs           letters, row indices, utility keys
  src/lexicon.rs          French word list, accent folding, suggestions
  src/engine.rs           shift, caps, French spacing, word buffer, suggestions
  src/action.rs           actions emitted to the platform (commit/delete/ime)
  src/ffi.rs              JNI surface (create/render/key/inputType/reset)
  tests/engine_tests.rs   behaviour tests (cargo test)
app/                      Android app (Kotlin, no AndroidX needed)
  src/main/java/fr/type1/keyboard/
    Type1Ime.kt           InputMethodService
    KeyboardView.kt       three bands + touch handling
    SetupActivity.kt      enable/pick the keyboard
    core/NativeBridge.kt  external fun declarations
    core/KeyboardController.kt
    model/RowModel.kt     JSON model (suggestions, letters, keys)
    model/KeyAction.kt    JSON action model
scripts/build-rust.ps1    cross-compile the core into jniLibs
tools/jni-smoke/          JVM smoke test for the JNI contract (no device)
```

The contract between the two halves is JSON over JNI:

```jsonc
// render(handle) -> the three bands
{ "page": "letters", "shift": 0, "caps": false,
  "suggestions": ["Écouter", "Écrire", "École"],
  "letters": [ { "id": "a", "label": "A", "letter": true,
                 "row": 0, "alts": ["à","â","ä"] }, ... ],
  "strip":   [ { "id": "shift", "label": "⇧", "kind": "shift", "on": false },
               { "id": "backspace", "label": "⌫", "kind": "backspace", "on": false } ],
  "keys":    [ { "id": "page", "label": "123", "kind": "page",
                 "w": 1.0, "on": false }, ... ] }

// key(handle, "a") -> actions to apply
[ { "t": "commit", "v": "A" } ]

// key(handle, "sug:0") -> replace the word being typed
[ { "t": "delete", "n": 2 }, { "t": "commit", "v": "Écouter " } ]
```

## Build

### 1. Rust core

```bash
cargo test                 # core logic, runs on the host
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
export ANDROID_NDK_HOME=/path/to/ndk      # PowerShell: $env:ANDROID_NDK_HOME
./scripts/build-rust.ps1                  # writes app/src/main/jniLibs/<abi>/*.so
```

`./gradlew buildRust` runs the same step and is wired to `preBuild`. It only
warns when the toolchain is missing, so Kotlin errors stay readable.

### 2. APK

Requirements: **JDK 17 or 21**. JDK 24+ breaks Gradle 8.11 (its embedded Kotlin
parser rejects version strings such as `27.0.1.0.3`, which is all the error
message says). `gradle/gradle-daemon-jvm.properties` pins the daemon to a JDK 21
toolchain, so the wrapper works even when `JAVA_HOME` points elsewhere.

Open the project in Android Studio (JDK 17 or 21) or:

```bash
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

If toolchain auto-detection does not find a JDK 21, run Gradle through the
helper, which searches `JAVA_HOME`, Android Studio's JBR, `~/.jdks` and
`Program Files\Java` and picks the newest 17-21:

```powershell
./scripts/gradle.ps1 assembleDebug
```

Then launch **Type1**, enable it in the system settings, and pick it with the
keyboard switcher.

### 3. Test without a device

```powershell
# core logic
cd core; cargo test

# JNI contract: real JVM -> Rust -> JSON (uses the host build of the core)
./tools/jni-smoke/run.ps1
```

`run.ps1` prints the row model and the actions for a few key presses. The Java
class mirrors the Kotlin `NativeBridge` so a broken symbol name or signature
fails immediately, without installing an APK.

## Verified

This snapshot was built and linted on Windows with:

- Rust 1.101 (nightly), `cargo test`: 32 tests passing.
- JDK 21 (Temurin), Gradle 8.11.1, AGP 8.7.2, Kotlin 2.0.21.
- NDK 27.0.12077973 + cargo-ndk 4.1.2: `arm64-v8a`, `armeabi-v7a`, `x86_64`.
- `./gradlew assembleDebug`: APK 1.9 MB, contains the three `.so` files.
- `./gradlew lintDebug`: 0 errors (3 typo false positives on "Type1").
- JNI smoke test: suggestions (`ec` → `Écouter`/`Écrire`/`École`) and
  `⇧ ⌫ ⏎ é` round-trip correctly through the host JVM.

No device was attached, so on-device behaviour (touch, IME lifecycle, sound) is
untested.

## Interaction notes

| Gesture | Result |
| --- | --- |
| Tap a letter | Type it (respects shift / auto-caps) |
| Drag along the letter line | Move the highlight, release to type |
| Long-press a letter | Accent strip replaces the letter line; slide and release |
| Tap a suggestion | Delete the typed word and insert the suggestion plus a space |
| Hold ⌫ | Repeat delete |
| Page key (`123` / `#+=` / `ABC`) | Letters → numbers → symbols → letters |
| Tap ⇧ once / twice | One capital / caps lock |
| `▾` / `⌨` | Hide the keyboard / switch keyboard |

## Roadmap

1. **Prediction** — frequency-ordered lexicon, next-word model, autocorrect with
   undo, so the strip earns its space.
2. **Typing comfort** — magnification popup above the strip, per-letter touch
   model, haptic strength setting.
3. **French details** — smart accents on double-tap, elision (`l'`, `d'`, `j'`),
   proper-noun capitalisation.
4. **Text tools** — clipboard panel, cursor arrows, word delete on long-press
   ⌫, double-space period.
5. **Personalisation** — dictionary additions, themes, per-app page memory.
6. **Later** — emoji/kaomoji panel, voice input hand-off, glide typing adapted
   to one line, iOS UI on the same core.

## Licence

GPL-3.0 — see [LICENSE](LICENSE).
