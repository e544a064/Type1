# Feature inventory & status

Baseline of what mainstream Android keyboards offer, and where Type1 stands.
`[x]` MVP, `[ ]` roadmap, `[~]` partially there.

## 1. Core typing & layout
- [x] Letters, digits, punctuation, space, enter, backspace
- [x] Shift, caps lock (tap twice)
- [x] AZERTY, all 26 letters on one line (column order, row-based vertical offsets)
- [x] Alternates via long-press (accents, `’`, `–`, `«»`)
- [x] Symbols / numbers replace the letter line
- [x] Contextual enter key (`⏎`, `→`, `✓`, `🔍`, `➤`)
- [x] Shift and backspace pinned to the suggestion strip
- [x] Hold-to-repeat backspace
- [ ] Key preview bubble
- [ ] Other layouts (QWERTY, QWERTZ, BÉPO)
- [ ] Landscape-specific tuning

## 2. Modes & switching (critical for a one-row keyboard)
- [x] Page key: letters → numbers → symbols → letters
- [x] Shift and backspace occupy the suggestion strip's ends; the bottom row is page, space, comma, enter
- [ ] Language/layout switching key
- [ ] Remember last page per app
- [ ] IME picker key (globe)

## 3. Text editing
- [x] Backspace with repeat (suggestion strip, right end)
- [x] Word delete in the core (not bound to a gesture yet)
- [ ] Cursor arrows / swipe-on-space cursor control
- [ ] Select word / line / all
- [ ] Clipboard history panel
- [ ] Undo / redo

## 4. Suggestions & correction
- [x] Suggestion strip above the letters (prefix lookup, embedded French lexicon)
- [x] Accent-insensitive matching (`ecole` → `École`) and one-tap word replacement
- [ ] Autocorrect with revert
- [ ] Next-word prediction
- [x] Auto-capitalisation (sentences / words / characters)
- [x] Automatic spacing around French punctuation
- [ ] Spell checking
- [ ] Personal dictionary
- [ ] Multilingual typing
- [ ] Double-space period
- [ ] Grammar/style hints

## 5. Gestures
- [x] Drag along the letter line to pick a letter before releasing
- [x] Long-press alternates strip (slide to pick)
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
