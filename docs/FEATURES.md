# Feature inventory & status

Baseline of what mainstream Android keyboards offer, and where Type1 stands.
`[x]` MVP, `[ ]` roadmap, `[~]` partially there.

## 1. Core typing & layout
- [x] Letters, digits, punctuation, space, enter, backspace
- [x] Shift, caps lock (tap twice)
- [x] Multi-touch rollover: overlapping presses are committed in down order
- [x] AZERTY and QWERTY, all 26 letters on one line (column order, row-based vertical offsets)
- [x] Alternates via long-press (accents, `'`, `-`, `«»`, `!`, `?`, `:`, `0`–`9` on the page key)
- [x] Period key shows its `!`/`?`/`:` alternates greyed at the top
- [x] Page key long-press popup for a single digit without leaving the page
- [x] Symbols / numbers replace the letter line
- [x] Contextual enter key (`⏎`, `→`, `✓`, `🔍`, `➤`); an explicit action
      wins over multiline, newline only without an action or with
      `IME_FLAG_NO_ENTER_ACTION`
- [x] Shift and backspace pinned to the suggestion strip
- [x] Hold-to-repeat backspace
- [x] Comma left of the spacebar, period on its right with `!`/`?` on long-press
- [ ] Key preview bubble
- [~] Other layouts: AZERTY and QWERTY data; Sebeolsik/Korean pending
- [ ] Landscape-specific tuning

## 2. Modes & switching (critical for a one-row keyboard)
- [x] Page key: letters → numbers → symbols → letters
- [x] Shift and backspace occupy the suggestion strip's ends; the bottom row is page, comma, space, period, enter
- [x] Profiles page: create, edit, delete and select profiles from a layout
      dropdown and language toggles
- [x] Long-press comma to cycle to the next profile (globe popup, wraps around)
- [x] Spacebar shows the active language codes (`FR`, `FR-EN`, ...)
- [ ] Remember last page per app
- [ ] IME picker key (globe)

## 3. Text editing
- [x] Backspace with a configurable long-press (repeat, swipe-to-size selection, nothing)
- [x] Word delete in the core (not bound to a gesture yet)
- [x] Swipe on the spacebar to move the cursor (switch in the setup screen)
- [x] Configurable swipe sensitivity for the spacebar cursor and backspace selection
- [x] Configurable spacebar row height (36–72 dp)
- [ ] Select word / line / all
- [ ] Clipboard history panel
- [ ] Undo / redo

## 4. Suggestions & correction
- [x] Suggestion strip above the letters showing correction candidates
- [x] Short tokens show more candidates (up to six for two or three letters)
- [x] Single-letter suggestions: one-letter words and accents first, then the
      neighbouring keys (one and two keys away). `q` → `à`, `a`, `w`, `z`; an
      English neighbour of `I` → `I`, plus the raw letter `i` and further keys
- [x] Per-field correction memory: moving back into a changed word re-opens
      the original suggestions, capped and cleared on a new or sensitive field
- [x] Accent-insensitive matching (`ecole` → `école`) and one-tap replacement
- [x] Per-language dictionaries (French and English UK) with a profile-scoped
      corrector
- [x] Autocorrect at word boundaries with revert (`jai` → `j'ai`)
- [x] Autocorrect with caps lock: all-caps words keep their capitals
      (`LANGAUGE` → `LANGUAGE`, `JAI` → `J'AI`)
- [x] British English autocorrect with British spelling preference
      (`langauge` → `language`, `colour`, `centre`, `organise`)
- [x] English contractions restored (`dont` → `don't`, `cant` → `can't`,
      `isnt` → `isn't`, `youre` → `you're`, `oclock` → `o'clock`), with nearby
      key slips corrected too (`douv` → `don't`)
- [x] Elision (`jai`, `taime`, `lhomme`, `daccord`, `quil`, `jusqua`) with
      typos inside the elided word (`taimx` → `t'aime`) and grammar-aware
      validation (`j'aime`, not `t'est`)
- [x] Narrow-key rescue for fully garbled tokens (`geqnvx` → `beauté`),
      elisions with a mistyped clitic (`vqjmd` → `t'aime`), two-key slips
      (`bknjpur` → `bonjour`), spelling variants (`commant` → `comment`) and
      a last-resort closest-word suggestion (`bodjmur` → `bonjour`)
- [x] Re-suggest corrections when the caret moves back into an existing word
- [x] Full-word suggestions and replacement when typing inside a word
- [ ] Next-word prediction
- [x] Auto-capitalisation (sentences / words / characters), including plain-text
      fields that do not request a cap flag, and after caret moves or backspace
- [x] Automatic spacing around French punctuation, including removing a stray
      space before `.`/`,` (also after a suggestion tap)
- [ ] Spell checking
- [ ] Personal dictionary
- [~] Multilingual typing: profiles can list several languages and combine
      their dictionaries; mixed-language ranking and Korean are still pending
- [ ] Double-space period
- [ ] Grammar/style hints

## 5. Gestures
- [x] Drag along the letter line to pick a letter before releasing
- [x] Long-press accent popup above the key, merging the pressed key and its neighbours (slide to pick)
- [ ] Glide typing adapted to one line
- [ ] Swipe-up utilities panel

## 6. Emoji & rich content
- [ ] Emoji panel, recents, skin tones
- [ ] Kaomoji
- [ ] GIF / sticker insertion
- [ ] Extended symbol pages

## 7. Voice & other input
- [ ] Voice dictation hand-off
- [ ] Handwriting

## 8. Appearance
- [x] Dark theme baseline
- [x] Haptics and key sounds
- [ ] Theme picker / dynamic colour
- [ ] Key height and font size settings
- [ ] Light theme

## 9. Productivity
- [ ] Text expansion / snippets
- [ ] Smart clipboard actions (OTP, URLs)
- [ ] Per-app behaviour
- [ ] Number row toggle

## 10. Privacy & security
- [x] Everything on device, no network code
- [ ] Incognito mode (disable learning)
- [ ] Sensitive-field handling (no logging today, but no learning either)
- [ ] Clear learned data

## 11. Accessibility & localisation
- [ ] TalkBack descriptions for keys
- [ ] Large-text / high-contrast mode
- [ ] Configurable touch timing
- [ ] RTL / complex scripts
- [x] French UI strings

## 12. System integration
- [x] Correct `inputType` handling (caps, numeric, multiline)
- [x] Editor actions from `imeOptions`
- [x] No fullscreen extraction
- [x] Predictive back gestures in the setup and profiles activities
- [ ] Physical keyboard passthrough
- [~] Settings: profiles page plus the onboarding screen; other options are
      still inline in onboarding
