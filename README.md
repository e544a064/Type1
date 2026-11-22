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
│ 123      ,        espace        .        ⏎                 │  48 dp
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
- Autocorrection at word boundaries from a 20k frequency-ranked French
  dictionary with a layout-aware touch model: neighbouring-key slips,
  transpositions, missing accents (`ecole` → `école`) and elision
  (`jai` → `j'ai`, `taimx` → `t'aime`, `taipe` → `t'aime`).
- Narrow-key rescue: when every typed key is one or two keys away from a
  word, the most frequent such word is offered (`geqnvx` → `beauté`,
  `vqjmd` → `t'aime`, `bknjpur` → `bonjour`). One-key candidates always rank
  first: `emps` shows `temps` before the two-key `elle`.
- One-letter spelling variants are considered even when the letter is far from
  the key pressed: `commant` offers `comment` (frequency) and `collant`
  (two-key slip), both in the strip.
- Very last resort: when nothing is one or two keys away and no spelling
  variant fits, the closest same-length dictionary word is offered
  (`bodjmur` → `bonjour`). That tier is suggestion-only, never auto-applied.
- The suggestion strip ranks by word frequency as well as key distance, so
  `ne` comes before `eu` for `ue`, and `t'aime` before the rare `tapie`.
- Short words are corrected too, to the most likely candidate: `pq` → `la`
  (with `ma` offered next in the strip). Rare dictionary words are corrected
  the same way (`pa` → `la`), while common words stay put.
- Overlapping fast taps are supported: keys are sent in the order they went
  down, even when several fingers are still held (`x` then `d` → `xd`).
- The suggestion strip shows correction candidates while typing and the
  original + alternatives after a correction; the first backspace undoes the
  correction, keeps the boundary you typed and reopens the word.
- Move the caret back into an existing word and the strip re-suggests
  corrections for it; tapping one replaces the whole word in place
  (`Roppxjf` → `Comment`). Backspacing over a finished word's boundary brings
  its corrections back too, without moving the caret.
- Conservative application: a confidence margin (stricter for two-letter
  tokens), no all-caps/mixed-case words, no suggestions in password/email/URI/
  name/address/number fields or when the editor asks for none.
- French accents on long-press (`é è ê ë à â ç ù û î ï ô ö œ ÿ ñ`), plus `’`.
- Shift (tap once = one capital, twice = caps lock), auto-capitalisation from
  `inputType` (`CAP_SENTENCES`, `CAP_WORDS`, `CAP_CHARACTERS`).
- French spacing: space before `? ! : ;`, automatic space after `. , ; : ! ?`
  when a new word starts.
- Backspace with a configurable long-press (repeat, swipe to size a selection,
  or nothing); enter adapts to the editor action
  (`→`, `✓`, `🔍`, `➤`) and inserts a newline in multiline fields.
- Drag along the letter line to correct a press before releasing.
- Drag horizontally on the spacebar to move the caret; the setup screen has a
  switch to turn this off.
- Swipe sensitivity (8–40 dp per character) is adjustable with a slider and
  shared by the spacebar caret drag and the backspace selection.
- Spacebar row height is adjustable (36–72 dp, 48 dp by default).
- Shift and backspace sit at the ends of the suggestion strip; the bottom row is
  page, comma, space, period and enter. Long-press the period for `!` and `?`.
- Vector icons for shift, backspace and enter (the enter icon follows the editor
  action: return, arrow, search, send, check).
- Sounds, haptics, pressed-letter highlight.

Not in the MVP: next-word prediction (context model),
personal dictionary, dictionary imports, glide typing, emoji panel, clipboard
manager, voice input, themes, physical keyboard.

## Why one line?

26 letters on a 360 dp phone give each letter ~11 dp of width. That is far below
the 48 dp accessibility guideline, and it is the central trade-off of the
design: one line saves ~100 dp of height, and the suggestion strip is what makes
it workable. Three things compensate:

- nearest-letter hit testing (the whole strip is one target area),
- drag-to-correct: press, slide, release on the right letter,
- autocorrection with layout-aware candidates (`dkre` → `dire`,
  `ecole` → `école`).

Layout is data, not code: `core/src/layout.rs` lists the letters in AZERTY
column order with a `row` index (0 top, 1 home, 2 bottom). Change the order or
the offsets there and the UI follows.

## Sizing

The three bands are 38 dp (suggestions), 56 dp (letters) and 36–72 dp
(utilities, 48 dp by default),
plus 3 dp gaps: **154 dp** total by default. On a 411 dp-wide phone a letter is
~12.5 dp wide; on 360 dp it is ~11 dp. Tuning knobs:

- `suggestionHeight`, `letterHeight` in `KeyboardView.kt`,
- the utility row height is a setting (36–72 dp, default 48),
- the `rowOffset` values (±7 dp) in `KeyboardView.kt`,
- letter order, row indices and utility weights in `core/src/layout.rs`,
- `MIN_SCORE`, `TWO_KEY_MIN_SCORE`, `AGGRESSIVE_WEIGHT`, `LENGTH_PENALTY` and
  `KNOWN_WORD_MARGIN` in `core/src/correction.rs`,
- the dictionary in `core/data/fr.tsv` (see `core/data/NOTICE.md`).

## Repository layout

```
core/                     Rust crate (no Android dependency)
  src/layout.rs           letters, row indices, utility keys
  src/dictionary.rs       frequency-ranked French dictionary (embedded data)
  src/correction.rs       layout-aware candidates and scoring
  src/lexicon.rs          accent folding
  src/engine.rs           shift, caps, spacing, word buffer, correction, undo
  src/action.rs           actions emitted to the platform (commit/delete/ime)
  src/ffi.rs              JNI surface (create/render/key/inputType/selection)
  data/fr.tsv             derived Lexique 3.83 list (20k forms)
  data/NOTICE.md          dictionary licence and attribution
  examples/evaluate.rs    dev-only correction evaluation
  examples/make_autocorrect_cases.rs  regenerate the bulk fixture
  tests/engine_tests.rs   keyboard-engine behaviour tests
  tests/autocorrect_tests.rs  bulk autocorrect regressions
  tests/data/autocorrect_cases.tsv  2,500 mixed typo cases
  tests/data/two_key_cases.tsv  2,000 two-key-slip cases
  tests/data/spelling_cases.tsv  2,000 spelling-variant cases
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
scripts/build-dictionary.ps1  regenerate core/data/fr.tsv from Lexique 3.83
tools/jni-smoke/          JVM smoke test for the JNI contract (no device)
```

## Dictionary

`core/data/fr.tsv` is derived from [Lexique 3.83](http://www.lexique.org)
(CC BY-SA 4.0): the 20,000 most frequent lowercase forms, film and book
frequencies summed per form, stored as hundredths of `ln(freq + 1)` plus a
part-of-speech flag used to validate elisions (`j'aime` but not `t'est`). The
setup screen shows the attribution and `core/data/NOTICE.md` carries the full
notice. `cargo run --release --example evaluate` reports top-1 accuracy on
synthetic typos and the random-string false-positive rate.
`scripts/build-dictionary.ps1` regenerates the list from an extracted
`Lexique383.tsv`.

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
# core logic, 6,500 autocorrect cases and held-out accuracy floors
cd core; cargo test

# JNI contract: real JVM -> Rust -> JSON (uses the host build of the core)
./tools/jni-smoke/run.ps1
```

The bulk fixtures are regenerated after tuning the corrector:

```powershell
cd core; cargo run --release --example make_autocorrect_cases
```

`run.ps1` prints the row model and the actions for a few key presses. The Java
class mirrors the Kotlin `NativeBridge` so a broken symbol name or signature
fails immediately, without installing an APK.

## Verified

This snapshot was built and linted on Windows with:

- Rust 1.101 (nightly), `cargo test`: 99 tests plus 6,500 fixture cases.
- JDK 21 (Temurin), Gradle 8.11.1, AGP 8.7.2, Kotlin 2.0.21.
- NDK 27.0.12077973 + cargo-ndk 4.1.2: `arm64-v8a`, `armeabi-v7a`, `x86_64`.
- `./gradlew assembleDebug`: APK 1.9 MB, contains the three `.so` files.
- `./gradlew lintDebug`: 0 errors (3 typo false positives on "Type1").
- JNI smoke test: autocorrect and undo round-trip correctly through the host JVM.

No device was attached, so on-device behaviour (touch, IME lifecycle, sound) is
untested.

## Interaction notes

| Gesture | Result |
| --- | --- |
| Tap a letter | Type it (respects shift / auto-caps) |
| Drag along the letter line | Move the highlight, release to type |
| Drag on the spacebar | Move the cursor left/right (disable in the app) |
| Move the caret into an existing word | Re-suggests corrections for that word; tap one to replace it |
| Long-press a letter | Popup above the key with its own and neighbouring accents; slide and release |
| Long-press `.` | Popup with `!` and `?`; slide and release |
| Tap a suggestion | Replace the typed or corrected word with the candidate |
| ⌫ right after a correction | Undo the correction, restore the typed word and keep the space or punctuation |
| Hold ⌫ | Depends on the setting: repeat delete, swipe to size a selection then delete, or nothing |
| Page key (`123` / `#+=` / `ABC`) | Letters → numbers → symbols → letters |
| Tap ⇧ once / twice | One capital / caps lock |

## Roadmap

1. **Prediction** — next-word model and context-aware ranking, so the strip
   earns its space.
2. **Typing comfort** — magnification popup above the strip, per-letter touch
   model, haptic strength setting.
3. **French details** — smart accents on double-tap, proper-noun
   capitalisation.
4. **Text tools** — clipboard panel, cursor arrows, word delete on long-press
   ⌫, double-space period.
5. **Personalisation** — dictionary additions, themes, per-app page memory.
6. **Later** — emoji/kaomoji panel, voice input hand-off, glide typing adapted
   to one line, iOS UI on the same core.

## Licence

GPL-3.0 — see [LICENSE](LICENSE).
