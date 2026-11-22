# Feature inventory & status

Baseline of what mainstream Android keyboards offer, and where Type1 stands.
`[x]` MVP, `[ ]` roadmap, `[~]` partially there.

## 1. Core typing & layout
- [x] Letters, digits, punctuation, space, enter, backspace
- [x] Shift, caps lock (tap twice)
- [x] Multi-touch rollover: overlapping presses are committed in down order
- [x] AZERTY, all 26 letters on one line (column order, row-based vertical offsets)
- [x] Alternates via long-press (accents, `'`, `-`, `«»`, `!`, `?`, `:`, `0`–`9` on the page key)
- [x] Period key shows its `!`/`?`/`:` alternates greyed at the top
- [x] Page key long-press popup for a single digit without leaving the page
- [x] Symbols / numbers replace the letter line
- [x] Contextual enter key (`⏎`, `→`, `✓`, `🔍`, `➤`)
- [x] Shift and backspace pinned to the suggestion strip
- [x] Hold-to-repeat backspace
- [x] Comma left of the spacebar, period on its right with `!`/`?` on long-press
- [ ] Key preview bubble
- [ ] Other layouts (QWERTY, QWERTZ, BÉPO)
- [ ] Landscape-specific tuning

## 2. Modes & switching (critical for a one-row keyboard)
- [x] Page key: letters → numbers → symbols → letters
- [x] Shift and backspace occupy the suggestion strip's ends; the bottom row is page, comma, space, period, enter
- [ ] Language/layout switching key
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
- [x] Accent-insensitive matching (`ecole` → `école`) and one-tap replacement
- [x] Frequency-ranked 20k French dictionary with a layout-aware touch model
- [x] Autocorrect at word boundaries with revert (`jai` → `j'ai`)
- [x] Elision (`jai`, `taime`, `lhomme`, `daccord`, `quil`, `jusqua`) with
      typos inside the elided word (`taimx` → `t'aime`) and grammar-aware
      validation (`j'aime`, not `t'est`)
- [x] Narrow-key rescue for fully garbled tokens (`geqnvx` → `beauté`),
      elisions with a mistyped clitic (`vqjmd` → `t'aime`), two-key slips
      (`bknjpur` → `bonjour`), spelling variants (`commant` → `comment`) and
      a last-resort closest-word suggestion (`bodjmur` → `bonjour`)
- [x] Re-suggest corrections when the caret moves back into an existing word
- [ ] Next-word prediction
- [x] Auto-capitalisation (sentences / words / characters), including plain-text
      fields that do not request a cap flag, and after caret moves or backspace
- [x] Automatic spacing around French punctuation, including removing a stray
      space before `.`/`,` (also after a suggestion tap)
- [ ] Spell checking
- [ ] Personal dictionary
- [ ] Multilingual typing
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
- [ ] Physical keyboard passthrough
- [ ] Settings screen beyond onboarding
