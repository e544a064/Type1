//! Keyboard state machine: pages, shift, capitalisation, French spacing,
//! current-word tracking, correction and suggestions.

use std::collections::VecDeque;

use crate::action::{Action, ImeActionKind};
use crate::correction::Suggestion;
use crate::hangul;
use crate::json;
use crate::layout::{Letter, Page, ShiftState};
use crate::lexicon::fold;
use crate::personal::{PersonalLexicon, PersonalWord};
use crate::profile::Profile;

const STRIP_SUGGESTIONS: usize = 3;
const MAX_STRIP_SUGGESTIONS: usize = 6;

/// Per-field correction memory caps. Only words the keyboard actually changed
/// are stored, so this is generous for a normal text field.
const MAX_MEMORY_ENTRIES: usize = 1_000;
const MAX_MEMORY_BYTES: usize = 256 * 1024;
const MEMORY_ENTRY_OVERHEAD: usize = 32;

/// Short tokens leave room for more cells in the strip. The corrector
/// generates at most six candidates.
fn suggestion_limit(typed: &str) -> usize {
	match typed.chars().count() {
		0..=3 => MAX_STRIP_SUGGESTIONS,
		4 => 5,
		5 => 4,
		_ => STRIP_SUGGESTIONS,
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsMode {
	None,
	Characters,
	Words,
	Sentences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CasePattern {
	Lower,
	Capitalized,
	AllCaps,
	Mixed,
}

#[derive(Debug, Clone)]
struct CorrectionRecord {
	original: String,
	corrected: String,
	/// Exact text committed as the word boundary (may include a leading space).
	boundary: String,
	alternatives: Vec<String>,
}

/// Word around the caret, restored when the user taps back into existing text.
#[derive(Debug, Clone)]
struct ContextWord {
	before: String,
	after: String,
}

impl ContextWord {
	fn full(&self) -> String {
		format!("{}{}", self.before, self.after)
	}
}

/// A word the keyboard changed, paired with what the user actually typed.
#[derive(Debug)]
struct MemoryEntry {
	/// Lowercase visible word, used as the content address.
	visible: String,
	typed: String,
}

#[derive(Debug, Default)]
struct CorrectionMemory {
	entries: VecDeque<MemoryEntry>,
	bytes: usize,
}

impl CorrectionMemory {
	fn clear(&mut self) {
		self.entries.clear();
		self.bytes = 0;
	}

	fn remember(&mut self, visible: &str, typed: &str) {
		let key = visible.to_lowercase();
		if key.is_empty() || typed.is_empty() || key == typed.to_lowercase() {
			return;
		}
		if let Some(index) = (
			self
			. entries
			. iter()
			. position(|entry| entry.visible == key)
		) {
			if let Some(old) = self.entries.remove(index) {
				self.bytes = self.bytes.saturating_sub(entry_bytes(&old));
			}
		}
		let entry = MemoryEntry {
			visible: key,
			typed: typed.to_string(),
		};
		self.bytes += entry_bytes(&entry);
		self.entries.push_front(entry);
		while self.entries.len() > MAX_MEMORY_ENTRIES || self.bytes > MAX_MEMORY_BYTES {
			let Some(old) = self.entries.pop_back() else {
				break;
			};
			self.bytes = self.bytes.saturating_sub(entry_bytes(&old));
		}
	}

	fn lookup(&self, visible: &str) -> Option<&str> {
		let key = visible.to_lowercase();
		self.entries
		. iter()
		. find(|entry| entry.visible == key)
		. map(|entry| entry.typed.as_str())
	}
}

fn entry_bytes(entry: &MemoryEntry) -> usize {
	entry.visible.len() + entry.typed.len() + MEMORY_ENTRY_OVERHEAD
}

#[derive(Debug)]
pub struct Engine {
	profile: Profile,
	page: Page,
	shift: ShiftState,
	caps: CapsMode,
	/// True when the next letter starts a sentence (or a word in Words mode).
	at_start: bool,
	/// True when the current word began at the start of a sentence.
	word_starts_sentence: bool,
	last_char: Option<char>,
	/// Letters (and inner apostrophes) committed since the last boundary.
	word: String,
	/// Jamo keys behind the current Hangul word. Empty for other scripts.
	jamo: Vec<char>,
	ime_action: ImeActionKind,
	/// True when the editor asked the enter key to stay a newline.
	no_enter_action: bool,
	/// Whether the suggestion strip and suggestion taps are enabled.
	suggestions_enabled: bool,
	/// Whether word boundaries may replace the typed word automatically.
	autocorrect_enabled: bool,
	/// Host-provided personal words, usually Android's user dictionary.
	personal: PersonalLexicon,
	last_correction: Option<CorrectionRecord>,
	/// Words changed in this field, keyed by their visible form.
	correction_memory: CorrectionMemory,
	/// Word accepted from the suggestion strip, kept until the user moves on.
	suggestion_word: Option<String>,
	/// Boundary text left just after the caret when a correction is undone.
	pending_boundary: Option<String>,
	/// The current word was restored by an undo; do not correct it again until
	/// the user edits it.
	reverted: bool,
	suggestion_cache: Option<(String, Suggestion)>,
	/// Word around the caret when the user moves back into existing text.
	context: Option<ContextWord>,
	/// Word characters after the caret when editing inside existing text.
	context_suffix: Option<String>,
	/// Whether dictionary indexes load on a background thread.
	async_loading: bool,
}

impl Default for Engine {
	fn default() -> Self {
		Self::new()
	}
}

impl Engine {
	pub fn new() -> Self {
		Engine {
			profile: Profile::default_profile(),
			page: Page::Letters,
			shift: ShiftState::Off,
			caps: CapsMode::Sentences,
			at_start: true,
			word_starts_sentence: true,
			last_char: None,
			word: String::new(),
			jamo: Vec::new(),
			ime_action: ImeActionKind::None,
			no_enter_action: false,
			suggestions_enabled: true,
			autocorrect_enabled: true,
			personal: PersonalLexicon::default(),
			last_correction: None,
			correction_memory: CorrectionMemory::default(),
			suggestion_word: None,
			pending_boundary: None,
			reverted: false,
			suggestion_cache: None,
			context: None,
			context_suffix: None,
			async_loading: false,
		}
	}

	pub fn page(&self) -> Page {
		self.page
	}

	pub fn profile(&self) -> &Profile {
		&self.profile
	}

	fn page_letters(&self) -> &'static [Letter] {
		self.profile.layout.chars(self.page)
	}

	/// Switch the active layout/language profile and start from a clean state.
	pub fn set_profile(&mut self, profile: Profile) {
		let mut profile = profile;
		profile.set_async_loading(self.async_loading);
		if self.profile.same_as(&profile) {
			self.profile.start_loading();
			return;
		}
		self.profile = profile;
		self.page = Page::Letters;
		self.reset();
		self.profile.start_loading();
	}

	/// Start loading the active profile's dictionaries on a background thread.
	pub fn prepare(&mut self) {
		self.async_loading = true;
		self.profile.set_async_loading(true);
		self.profile.start_loading();
	}

	pub fn set_profile_ids(&mut self, layout_id: &str, language_ids: &[&str]) -> bool {
		let Some(profile) = Profile::from_ids(layout_id, language_ids) else {
			return false;
		};
		self.set_profile(profile);
		true
	}

	/// Replace the host-provided personal words. Suggestion results are cached
	/// per typed token, so a change invalidates the cache.
	pub fn set_personal_words(&mut self, words: Vec<PersonalWord>) {
		self.personal = PersonalLexicon::new(words);
		self.suggestion_cache = None;
	}

	/// Return to the built-in layout and languages when no user profile is
	/// configured.
	pub fn set_default_profile(&mut self) {
		self.set_profile(Profile::default_profile());
	}

	/// Forget a pending correction and the guards that belong to it.
	fn clear_correction_state(&mut self) {
		self.last_correction = None;
		self.suggestion_word = None;
		self.pending_boundary = None;
		self.reverted = false;
	}

	fn remember_correction(&mut self, visible: &str, typed: &str) {
		if self.suggestions_enabled {
			self.correction_memory.remember(visible, typed);
		}
	}

	fn remembered_typed(&self, visible: &str) -> Option<&str> {
		if self.suggestions_enabled {
			self.correction_memory.lookup(visible)
		} else {
			None
		}
	}

	pub fn reset(&mut self) {
		self.shift = ShiftState::Off;
		self.at_start = true;
		self.word_starts_sentence = true;
		self.last_char = None;
		self.word.clear();
		self.jamo.clear();
		self.clear_correction_state();
		self.correction_memory.clear();
		self.suggestion_cache = None;
		self.context = None;
		self.context_suffix = None;
	}

	/// Configure from `EditorInfo`. Flags mirror the Android constants so the
	/// core stays platform independent.
	pub fn set_input_type(&mut self, input_type: i32, ime_options: i32) {
		let class = input_type & 0x0000_000f;
		let variation = input_type & 0x0000_0ff0;
		// Password fields stay private: no suggestions and no correction.
		let password_variation = matches!(variation, 0x80 | 0x90 | 0xe0);
		// Prose fields where replacing a word would be unhelpful. Suggestions
		// still help the user type a URL, address or search query.
		let no_autocorrect_variation = matches!(
			variation,
			0x10 | 0x20 | 0x30 | 0x60 | 0x70 | 0xb0 | 0xd0
		);
		// Android's search views often use plain text plus the search action
		// or their own completion, and set the no-suggestions flag.
		let search_action = ime_options & 0x0000_00ff == 0x0000_0003;
		let no_suggestions = input_type & 0x0008_0000 != 0;
		let own_completion = input_type & 0x0001_0000 != 0 && no_suggestions;
		let search_field = no_autocorrect_variation || search_action || own_completion;

		let numeric = matches!(class, 0x2..=0x4);
		// Non-prose fields keep suggestions even when the editor sets the
		// no-suggestions flag (browsers do this for URL bars). Passwords never
		// do, and the flag still hides the strip for plain text.
		let suppress_suggestions = no_suggestions && !search_field;
		self.suggestions_enabled =
			class == 0x1 && !password_variation && !suppress_suggestions;
		self.autocorrect_enabled =
			self.suggestions_enabled && !search_field;
		self.caps = if input_type & 0x0000_1000 != 0 {
			CapsMode::Characters
		} else if input_type & 0x0000_2000 != 0 {
			CapsMode::Words
		} else if input_type & 0x0000_4000 != 0 {
			CapsMode::Sentences
		} else {
			CapsMode::None
		};
		self.apply_ime_options(ime_options);

		self.page = if numeric {
			Page::Numbers
		} else {
			Page::Letters
		};
		self.reset();
	}

	/// Refresh the editor action without resetting the current edit. The app
	/// may set `imeOptions` after `onStartInput`, just before the view appears.
	pub fn set_ime_options(&mut self, ime_options: i32) {
		self.apply_ime_options(ime_options);
	}

	fn apply_ime_options(&mut self, ime_options: i32) {
		self.ime_action = ImeActionKind::from_ime_options(ime_options);
		self.no_enter_action = ime_options & 0x4000_0000 != 0;
	}

	/// The editor selection moved: a pending correction can no longer be undone.
	pub fn selection_changed(&mut self) {
		self.clear_correction_state();
		self.context = None;
		self.context_suffix = None;
	}

	/// The caret moved into existing text. `before` and `after` are the text
	/// around the caret, so the word under the caret can be corrected as a
	/// whole even when the caret is inside it. The sentence position is read
	/// from the same text, so auto-capitalisation and French spacing stay
	/// correct after a caret move or a backspace.
	pub fn set_context_word(&mut self, before: &str, after: &str) {
		// A correction undo leaves the user's boundary just after the caret;
		// a suggestion tap leaves its accepted word there. The platform
		// re-reads the context after that caret move, so keep the related
		// guards while the text still matches.
		let suggestion = self.suggestion_word.take();
		let pending = self.pending_boundary.take();
		let was_reverted = self.reverted;
		self.selection_changed();
		let prefix = trailing_word(before);
		self.pending_boundary = pending.filter(|boundary| after.starts_with(boundary));
		self.reverted = (
			(self.pending_boundary.is_some() && was_reverted)
			|| suggestion.is_some_and(|word| prefix == word)
		);
		let suffix = leading_word(after);
		let total = prefix.chars().count() + suffix.chars().count();
		let sentence_start = starts_sentence(before);
		self.word.clear();
		self.word.push_str(&prefix);
		if self.profile.hangul() {
			self.jamo = hangul::decompose_keystrokes(&prefix);
		}
		self.context_suffix = if self.suggestions_enabled && !suffix.is_empty() {
			Some(suffix.clone())
		} else {
			None
		};
		self.context = if total >= 2 && self.suggestions_enabled {
			Some(ContextWord {
				before: prefix,
				after: suffix,
			})
		} else {
			None
		};
		self.suggestion_cache = None;
		// Keep the real character before the caret: it decides the automatic
		// space after punctuation even when no word sits there.
		self.last_char = before.chars().last().or_else(|| after.chars().last());
		let word_start = self.word.is_empty();
		self.word_starts_sentence = sentence_start;
		self.at_start = sentence_start || (self.caps == CapsMode::Words && word_start);
	}

	/// Handle a key press. Returns the editor actions to apply, in order.
	/// The UI always re-renders afterwards.
	pub fn on_key(&mut self, id: &str) -> Vec<Action> {
		if !id.starts_with("sug:") {
			self.context = None;
		}
		if let Some(symbol) = id.strip_prefix("sym:") {
			let space_before = self.profile.spacing.space_before(symbol);
			return self.finish_word(symbol, space_before);
		}
		if let Some(alt) = id.strip_prefix("alt:") {
			if (
				self.profile.hangul()
				&& !alt.is_empty()
				&& alt.chars().all(hangul::is_jamo)
			) {
				return self.commit_hangul(alt);
			}
			return self.commit(alt, true, false);
		}
		if let Some(index) = id.strip_prefix("sug:") {
			return self.apply_suggestion(index);
		}

		match id {
			"shift" => {
				self.shift = self.shift.cycle();
				Vec::new()
			}
			"page" => {
				self.cycle_page();
				Vec::new()
			}
			"space" => self.finish_word(" ", false),
			"comma" => self.finish_word(",", false),
			"dot" => self.finish_word(".", false),
			"backspace" => self.backspace(),
			"cursor_left" => {
				self.drop_word();
				vec![Action::MoveCursor(-1)]
			}
			"cursor_right" => {
				self.drop_word();
				vec![Action::MoveCursor(1)]
			}
			"select_left" => {
				self.drop_word();
				vec![Action::ExtendSelection(-1)]
			}
			"select_right" => {
				self.drop_word();
				vec![Action::ExtendSelection(1)]
			}
			"start_selection" => {
				self.drop_word();
				vec![Action::StartSelection]
			}
			"delete_selection" => {
				self.drop_word();
				vec![Action::DeleteSelection]
			}
			"enter" => self.enter(),
			_ => match self.page_letters().iter().find(|letter| letter.id == id) {
				Some(letter) if letter.id == "apos" => self.commit_apostrophe(),
				Some(letter) if !letter.letter && self.page == Page::Symbols => {
					let space_before = self.profile.spacing.space_before(letter.commit);
					self.finish_word(letter.commit, space_before)
				}
				Some(letter) if self.profile.hangul() && letter.letter => {
					let text = letter_text(letter, self.shift != ShiftState::Off);
					self.commit_hangul(text)
				}
				Some(letter) => {
					let text = letter_text(letter, self.shift != ShiftState::Off);
					self.commit(text, letter.letter, false)
				}
				None => Vec::new(),
			},
		}
	}

	/// Commit a character, applying shift, auto-capitalisation and French
	/// spacing (a space is required before `? ! : ;`, and after sentence
	/// punctuation when a new word starts).
	fn commit(&mut self, text: &str, is_letter: bool, space_before: bool) -> Vec<Action> {
		let mut out = Vec::new();
		self.suggestion_word = None;

		if is_letter {
			self.last_correction = None;
		}

		let upper = is_letter && (self.shift != ShiftState::Off || self.auto_caps());
		let rendered = if upper {
			text.to_uppercase()
		} else {
			text.to_string()
		};

		let mut leading_space = space_before && !matches!(self.last_char, Some(' ') | None);
		if is_letter
			&& matches!(self.last_char, Some('.' | ',' | ';' | ':' | '!' | '?'))
		{
			leading_space = true;
		}

		if leading_space {
			out.push(Action::Commit(" ".into()));
			self.note(' ');
		}
		out.push(Action::Commit(rendered.clone()));

		if is_letter {
			if self.word.is_empty() {
				self.word_starts_sentence = self.at_start;
			}
			self.word.push_str(&rendered);
			self.reverted = false;
			if self.shift == ShiftState::Once {
				self.shift = ShiftState::Off;
			}
		} else {
			self.word.clear();
			self.jamo.clear();
			self.last_correction = None;
			self.context_suffix = None;
		}
		self.suggestion_cache = None;
		if let Some(c) = rendered.chars().last() {
			self.note(c);
		}
		out
	}

	/// Commit a jamo, composing it into the previous syllable. The visible word
	/// may shrink as well as grow: `간` plus `ㅏ` becomes `가나`, so the editor
	/// is updated by deleting only the changed suffix.
	fn commit_hangul(&mut self, text: &str) -> Vec<Action> {
		let mut out = Vec::new();
		self.suggestion_word = None;
		self.last_correction = None;
		if self.word.is_empty() {
			self.word_starts_sentence = self.at_start;
		}
		if self.jamo.is_empty() && !self.word.is_empty() {
			// The caret sits in text that this layout cannot decompose.
			self.word.clear();
		}
		self.jamo
		. extend(text.chars().filter(|c| hangul::is_jamo(*c)));
		let next = hangul::compose_jamo(&self.jamo);
		let old = std::mem::take(&mut self.word);
		let common = common_prefix_chars(&old, &next);
		let delete = old.chars().count().saturating_sub(common);
		if delete > 0 {
			out.push(Action::DeleteChars(delete as u32));
		}
		let suffix: String = next.chars().skip(common).collect();
		if !suffix.is_empty() {
			out.push(Action::Commit(suffix));
		}
		self.word = next;
		self.reverted = false;
		self.suggestion_cache = None;
		if self.shift == ShiftState::Once {
			self.shift = ShiftState::Off;
		}
		if let Some(c) = self.word.chars().last() {
			self.note(c);
		}
		out
	}

	/// An apostrophe inside a word continues the token (`j'ai`).
	fn commit_apostrophe(&mut self) -> Vec<Action> {
		self.suggestion_word = None;
		self.last_correction = None;
		if self.word.is_empty() && self.context_suffix.is_none() {
			return self.commit("'", false, false);
		}
		self.word.push('\'');
		self.last_char = Some('\'');
		self.reverted = false;
		self.suggestion_cache = None;
		vec![Action::Commit("'".into())]
	}

	fn enter(&mut self) -> Vec<Action> {
		if self.no_enter_action || self.ime_action == ImeActionKind::None {
			return self.finish_word("\n", false);
		}

		let mut out = Vec::new();
		let correction = if self.autocorrect_enabled {
			self.try_correction()
		} else {
			None
		};
		if let Some((corrected, _)) = correction {
			let original = std::mem::take(&mut self.word);
			out.push(Action::DeleteChars(original.chars().count() as u32));
			out.push(Action::Commit(corrected.clone()));
			self.note_text(&corrected);
			self.remember_correction(&corrected, &original);
		}
		self.word.clear();
		self.jamo.clear();
		self.clear_correction_state();
		self.context_suffix = None;
		self.suggestion_cache = None;
		out.push(Action::Ime(self.ime_action));
		out
	}

	/// Correct the current word (when possible) and commit a word boundary.
	fn finish_word(&mut self, boundary: &str, space_before: bool) -> Vec<Action> {
		debug_assert!(!boundary.is_empty());
		let mut out = Vec::new();
		// French typography never puts a space before a comma or a period.
		// Remove one that is already there (including the space a suggestion
		// added automatically) before committing the punctuation.
		if matches!(boundary, "," | ".") && self.last_char == Some(' ') {
			out.push(Action::Delete { word: false });
		}
		// After an undo the user's boundary is still in the editor, just after
		// the caret. Reuse a kept space, or step over kept punctuation.
		let pending = self.pending_boundary.take();
		let keep_space = pending.as_deref() == Some(" ");
		let reuse_space = boundary == " " && keep_space;
		let step_over = boundary == " " && pending.is_some() && !keep_space;

		let leading_space = space_before && !matches!(self.last_char, Some(' ') | None);
		let boundary_record = if leading_space {
			format!(" {boundary}")
		} else {
			boundary.to_string()
		};

		// Correct first, while the caret still sits right after the typed word.
		let correction = if self.autocorrect_enabled {
			self.try_correction()
		} else {
			None
		};
		if let Some((corrected, alternatives)) = correction {
			let original = std::mem::take(&mut self.word);
			out.push(Action::DeleteChars(original.chars().count() as u32));
			out.push(Action::Commit(corrected.clone()));
			self.note_text(&corrected);
			self.remember_correction(&corrected, &original);
			self.last_correction = Some(CorrectionRecord {
				original,
				corrected,
				boundary: boundary_record,
				alternatives,
			});
		} else {
			self.last_correction = None;
		}

		if reuse_space {
			out.push(Action::MoveCursor(1));
			self.note(' ');
		} else if let Some(pending) = pending {
			let length = pending.chars().count() as i32;
			if length > 0 {
				out.push(Action::MoveCursor(length));
				if !step_over {
					out.push(Action::DeleteChars(length as u32));
				}
			}
			if step_over {
				self.commit_boundary(&mut out, " ", false);
			} else {
				self.commit_boundary(&mut out, boundary, leading_space);
			}
		} else {
			self.commit_boundary(&mut out, boundary, leading_space);
		}
		self.word.clear();
		self.jamo.clear();
		self.suggestion_word = None;
		self.reverted = false;
		self.context_suffix = None;
		self.suggestion_cache = None;
		out
	}

	fn commit_boundary(&mut self, out: &mut Vec<Action>, boundary: &str, leading_space: bool) {
		if leading_space {
			out.push(Action::Commit(" ".into()));
			self.note(' ');
		}
		out.push(Action::Commit(boundary.to_string()));
		for c in boundary.chars() {
			self.note(c);
		}
	}

	/// Move the caret past the boundary kept by an undo and remove it, so the
	/// next boundary or suggestion replaces it cleanly.
	fn consume_pending_boundary(&mut self, out: &mut Vec<Action>) {
		if let Some(pending) = self.pending_boundary.take() {
			let length = pending.chars().count() as i32;
			if length > 0 {
				out.push(Action::MoveCursor(length));
				out.push(Action::DeleteChars(length as u32));
			}
		}
	}

	fn try_correction(&mut self) -> Option<(String, Vec<String>)> {
		if self.reverted {
			return None;
		}
		let typed = self.word.clone();
		let length = typed.chars().count();
		if length < 2 || typed.contains('\'') {
			return None;
		}
		if case_pattern(&typed) == CasePattern::Mixed {
			return None;
		}
		let lower = typed.to_lowercase();
		let ambiguous = self.ambiguous_contraction(&lower);
		if ambiguous.is_some() && !self.word_starts_sentence {
			return None;
		}

		let suggestion = self.suggestion_for(&typed);
		let personal_exact = (
			self
			. personal
			. exact(&fold(&lower), &self.profile.languages)
			. is_some()
		);
		let forced = if personal_exact {
			None
		} else {
			ambiguous.and_then(|contraction| {
				suggestion
				. candidates
				. iter()
				. find(|candidate| candidate.text.to_lowercase() == contraction)
			})
		};
		if !suggestion.apply && forced.is_none() {
			return None;
		}
		let best = forced.or_else(|| suggestion.candidates.first())?;
		let corrected = apply_case(&typed, &best.text);
		if corrected == typed {
			return None;
		}

		let mut alternatives = Vec::new();
		for candidate in suggestion.candidates.iter().skip(1) {
			if alternatives.len() >= 2 {
				break;
			}
			let text = apply_case(&typed, &candidate.text);
			if text != corrected && text != typed {
				alternatives.push(text);
			}
		}
		Some((corrected, alternatives))
	}

	/// Contraction for a typed key that is also an ordinary word (`lets` ->
	/// `let's`). Such keys only auto-apply at the start of a sentence.
	fn ambiguous_contraction(&self, lower: &str) -> Option<&'static str> {
		self
		. profile
		. languages
		. iter()
		. find_map(|language| {
			if !language.ambiguous_contractions.contains(&lower) {
				return None;
			}
			language
			. contractions
			. iter()
			. find_map(|&(key, contraction)| {
				(key == lower).then_some(contraction)
			})
		})
	}

	fn suggestion_for(&mut self, typed: &str) -> Suggestion {
		if let Some((cached, suggestion)) = &self.suggestion_cache {
			if cached == typed {
				return suggestion.clone();
			}
		}
		let suggestion = self.profile.suggest_with_personal(typed, &self.personal);
		self.suggestion_cache = Some((typed.to_string(), suggestion.clone()));
		suggestion
	}

	fn apply_suggestion(&mut self, raw_index: &str) -> Vec<Action> {
		let index: usize = match raw_index.parse() {
			Ok(value) => value,
			Err(_) => return Vec::new(),
		};
		let options = self.current_suggestions();
		let chosen = match options.get(index) {
			Some(value) => value.clone(),
			None => return Vec::new(),
		};

		if let Some(context) = self.context.take() {
			let before = context.before.chars().count() as u32;
			let after = context.after.chars().count() as u32;
			let full = context.full();
			let typed = (
				self
				. remembered_typed(&full)
				. map(str::to_string)
				. unwrap_or(full)
			);
			self.remember_correction(&chosen, &typed);
			self.word.clear();
			self.jamo.clear();
			self.clear_correction_state();
			self.context_suffix = None;
			self.suggestion_cache = None;
			return vec![Action::ReplaceSurrounding {
				before,
				after,
				text: chosen,
			}];
		}

		if let Some(suffix) = self.context_suffix.take() {
			let before = self.word.chars().count() as u32;
			let after = suffix.chars().count() as u32;
			let original = format!("{}{}", self.word, suffix);
			self.remember_correction(&chosen, &original);
			self.word.clear();
			self.jamo.clear();
			self.clear_correction_state();
			self.suggestion_cache = None;
			self.note_text(&chosen);
			return vec![Action::ReplaceSurrounding {
				before,
				after,
				text: chosen,
			}];
		}

		if self.word.is_empty() {
			if let Some(record) = self.last_correction.take() {
				let delete =
					record.corrected.chars().count() + record.boundary.chars().count();
				let mut out = Vec::new();
				if delete > 0 {
					out.push(Action::DeleteChars(delete as u32));
				}
				let text = format!("{chosen}{}", record.boundary);
				out.push(Action::Commit(text.clone()));
				self.note_text(&text);
				self.remember_correction(&chosen, &record.original);
				self.word.clear();
				self.jamo.clear();
				self.suggestion_word = None;
				self.suggestion_cache = None;
				if chosen != record.original {
					self.last_correction = Some(CorrectionRecord {
						original: record.original,
						corrected: chosen,
						boundary: record.boundary,
						alternatives: record.alternatives,
					});
				}
				return out;
			}
		}

		let typed = std::mem::take(&mut self.word);
		self.jamo.clear();
		let delete = typed.chars().count() as u32;
		let mut out = Vec::new();
		self.consume_pending_boundary(&mut out);
		if delete > 0 {
			out.push(Action::DeleteChars(delete));
		}
		out.push(Action::CommitSuggestion(format!("{chosen} ")));
		self.remember_correction(&chosen, &typed);
		self.clear_correction_state();
		self.suggestion_word = Some(chosen);
		self.suggestion_cache = None;
		self.note(' ');
		out
	}

	fn backspace(&mut self) -> Vec<Action> {
		if let Some(record) = self.last_correction.take() {
			if self.word.is_empty() {
				let boundary_length = record.boundary.chars().count();
				let delete = record.corrected.chars().count() + boundary_length;
				let mut out = Vec::new();
				if delete > 0 {
					out.push(Action::DeleteChars(delete as u32));
				}
				out.push(Action::Commit(format!(
					"{}{}",
					record.original, record.boundary
				)));
				if boundary_length > 0 {
					out.push(Action::MoveCursor(-(boundary_length as i32)));
				}
				self.pending_boundary = if record.boundary.is_empty() {
					None
				} else {
					Some(record.boundary)
				};
				self.word = record.original;
				self.jamo = hangul::decompose_keystrokes(&self.word);
				self.last_char = self.word.chars().last();
				self.at_start = false;
				self.reverted = true;
				self.suggestion_word = None;
				self.suggestion_cache = None;
				return out;
			}
		}

		// Deleting the convenience space after a suggestion reveals the
		// accepted word. Protect it from a new autocorrection until the user
		// edits it.
		if self.word.is_empty() && self.last_char == Some(' ') {
			if let Some(accepted) = self.suggestion_word.clone() {
				self.at_start = false;
				self.last_char = accepted.chars().last();
				self.word = accepted;
				self.jamo = hangul::decompose_keystrokes(&self.word);
				self.reverted = true;
				self.suggestion_cache = None;
				return vec![Action::Delete { word: false }];
			}
		}

		if self.profile.hangul() && !self.jamo.is_empty() {
			return self.backspace_hangul();
		}

		self.suggestion_word = None;
		self.at_start = false;
		self.last_char = None;
		self.word.pop();
		self.jamo.clear();
		self.reverted = false;
		self.suggestion_cache = None;
		vec![Action::Delete { word: false }]
	}

	/// Remove the last jamo, not the whole syllable: `가` becomes `ㄱ` and
	/// `값` becomes `갑`.
	fn backspace_hangul(&mut self) -> Vec<Action> {
		let old = std::mem::take(&mut self.word);
		self.jamo.pop();
		let next = hangul::compose_jamo(&self.jamo);
		let common = common_prefix_chars(&old, &next);
		let delete = old.chars().count().saturating_sub(common);
		let mut out = Vec::new();
		if delete > 0 {
			out.push(Action::DeleteChars(delete as u32));
		}
		let suffix: String = next.chars().skip(common).collect();
		if !suffix.is_empty() {
			out.push(Action::Commit(suffix));
		}
		self.word = next;
		self.suggestion_word = None;
		self.at_start = false;
		self.last_char = self.word.chars().last();
		self.reverted = false;
		self.suggestion_cache = None;
		out
	}

	fn word_with_suffix(&self) -> String {
		match &self.context_suffix {
			Some(suffix) => format!("{}{}", self.word, suffix),
			None => self.word.clone(),
		}
	}

	fn current_suggestions(&mut self) -> Vec<String> {
		if !self.suggestions_enabled {
			return Vec::new();
		}
		if let Some(full) = self.context.as_ref().map(ContextWord::full) {
			if full.chars().count() < 2 {
				return Vec::new();
			}
			if case_pattern(&full) == CasePattern::Mixed {
				return Vec::new();
			}
			// If the word was changed, re-open the suggestions for the form
			// the user actually typed, not for the visible correction.
			let source = self.remembered_typed(&full).unwrap_or(&full).to_string();
			let suggestion = self.suggestion_for(&source);
			let limit = suggestion_limit(&source);
			let full_key = full.to_lowercase();
			let source_key = source.to_lowercase();
			let mut out = Vec::new();
			for candidate in suggestion.candidates.iter() {
				if out.len() >= limit {
					break;
				}
				let text = apply_case(&full, &candidate.text);
				let key = text.to_lowercase();
				if key == full_key || key == source_key {
					continue;
				}
				if !out.contains(&text) {
					out.push(text);
				}
			}
			return out;
		}
		if !self.word.is_empty() || self.context_suffix.is_some() {
			let typed = self.word_with_suffix();
			let length = typed.chars().count();
			if length == 0 {
				return Vec::new();
			}
			if length == 1 {
				let suggestion = self.suggestion_for(&typed);
				let limit = suggestion_limit(&typed);
				let mut out = Vec::new();
				for candidate in suggestion.candidates.iter() {
					let text = apply_case(&typed, &candidate.text);
					if text == typed || out.contains(&text) {
						continue;
					}
					out.push(text);
					if out.len() >= limit {
						break;
					}
				}
				return out;
			}
			if length < 2 {
				return Vec::new();
			}
			if case_pattern(&typed) == CasePattern::Mixed {
				return Vec::new();
			}
			let suggestion = self.suggestion_for(&typed);
			let limit = suggestion_limit(&typed);
			return (
				suggestion
				. candidates
				. iter()
				. take(limit)
				. map(|candidate| apply_case(&typed, &candidate.text))
				. collect()
			);
		}

		if let Some(record) = &self.last_correction {
			let mut out = vec![record.original.clone()];
			for alternative in &record.alternatives {
				if out.len() >= STRIP_SUGGESTIONS {
					break;
				}
				if alternative != &record.corrected {
					out.push(alternative.clone());
				}
			}
			return out;
		}
		Vec::new()
	}

	fn drop_word(&mut self) {
		self.word.clear();
		self.jamo.clear();
		self.word_starts_sentence = false;
		self.last_char = None;
		self.clear_correction_state();
		self.context_suffix = None;
		self.suggestion_cache = None;
	}

	fn note_text(&mut self, text: &str) {
		if let Some(c) = text.chars().last() {
			self.note(c);
		}
	}

	fn note(&mut self, c: char) {
		self.last_char = Some(c);
		match c {
			'.' | '!' | '?' | '\n' => self.at_start = true,
			' ' => {
				if self.caps == CapsMode::Words {
					self.at_start = true;
				}
			}
			c if c.is_alphanumeric() && self.caps != CapsMode::Characters => {
				self.at_start = false;
			}
			_ => {}
		}
	}

	fn auto_caps(&self) -> bool {
		match self.caps {
			CapsMode::Characters => true,
			CapsMode::Words | CapsMode::Sentences => self.at_start,
			CapsMode::None => false,
		}
	}

	fn cycle_page(&mut self) {
		self.page = self.page.next();
		self.shift = ShiftState::Off;
	}

	/// Spacebar label: the active language codes, e.g. `FR` or `FR-EN`.
	fn language_label(&self) -> String {
		let label = (
			self
			. profile
			. languages
			. iter()
			. map(|language| language.short)
			. collect::<Vec<_>>()
			. join("-")
		);
		if label.is_empty() {
			"espace".to_string()
		} else {
			label
		}
	}

	fn enter_label(&self) -> &'static str {
		if self.no_enter_action || self.ime_action == ImeActionKind::None {
			"⏎"
		} else {
			self.ime_action.key_label()
		}
	}

	fn enter_icon(&self) -> &'static str {
		if self.no_enter_action || self.ime_action == ImeActionKind::None {
			return "enter";
		}
		match self.ime_action {
			ImeActionKind::Go | ImeActionKind::Next => "arrow-right",
			ImeActionKind::Search => "search",
			ImeActionKind::Send => "send",
			ImeActionKind::Done => "check",
			ImeActionKind::Previous => "arrow-left",
			ImeActionKind::None => "enter",
		}
	}

	/// Full keyboard model for the UI, as JSON.
	pub fn render_json(&mut self) -> String {
		self.profile.start_loading();
		let loading = self.profile.is_loading();
		let suggestions = self.current_suggestions();
		let language_label = self.language_label();

		let mut out = String::with_capacity(1800);
		out.push_str("{\"page\":");
		json::string_into(&mut out, self.page.id());
		out.push_str(",\"shift\":");
		out.push_str(&self.shift.code().to_string());
		out.push_str(",\"caps\":");
		json::bool_into(&mut out, self.shift != ShiftState::Off || self.auto_caps());
		out.push_str(",\"mergeLetterAlts\":");
		json::bool_into(&mut out, self.profile.layout.merge_letter_alts);
		out.push_str(",\"suggestions\":[");
		for (i, suggestion) in suggestions.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			json::string_into(&mut out, suggestion);
		}
		out.push_str("],\"loading\":");
		json::bool_into(&mut out, loading);
		out.push_str(",\"letters\":[");
		for (i, letter) in self.page_letters().iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			letter_json(&mut out, letter, self.shift != ShiftState::Off);
		}
		out.push_str("],\"strip\":[");
		for (i, key) in self.profile.layout.strip.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			let active = key.id == "shift" && self.shift != ShiftState::Off;
			out.push_str("{\"id\":");
			json::string_into(&mut out, key.id);
			out.push_str(",\"label\":");
			json::string_into(&mut out, key.label);
			out.push_str(",\"kind\":");
			json::string_into(&mut out, kind_id(key.kind));
			out.push_str(",\"icon\":");
			json::string_into(&mut out, key.icon);
			out.push_str(",\"on\":");
			json::bool_into(&mut out, active);
			out.push('}');
		}
		out.push_str("],\"keys\":[");
		for (i, key) in self.profile.layout.utilities.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			let label = match key.id {
				"page" => self.page.switch_label(),
				"space" => language_label.as_str(),
				"enter" => self.enter_label(),
				_ => key.label,
			};
			let icon = if key.id == "enter" {
				self.enter_icon()
			} else {
				key.icon
			};
			out.push_str("{\"id\":");
			json::string_into(&mut out, key.id);
			out.push_str(",\"label\":");
			json::string_into(&mut out, label);
			out.push_str(",\"kind\":");
			json::string_into(&mut out, kind_id(key.kind));
			out.push_str(",\"icon\":");
			json::string_into(&mut out, icon);
			out.push_str(",\"alts\":[");
			for (i, alt) in key.alts.iter().enumerate() {
				if i > 0 {
					out.push(',');
				}
				json::string_into(&mut out, alt);
			}
			out.push_str("],\"w\":");
			json::float_into(&mut out, key.weight);
			out.push_str(",\"on\":");
			json::bool_into(&mut out, false);
			out.push('}');
		}
		out.push_str("]}");
		out
	}
}

fn is_word_char(c: char) -> bool {
	c.is_alphabetic() || c == '\'' || c == '’'
}

/// Word-ending characters before the caret, used for context corrections.
fn trailing_word(text: &str) -> String {
	let mut chars: Vec<char> = (
		text
		. chars()
		. rev()
		. take_while(|c| is_word_char(*c))
		. collect()
	);
	chars.reverse();
	chars.into_iter().collect()
}

/// Word-starting characters after the caret.
fn leading_word(text: &str) -> String {
	text.chars().take_while(|c| is_word_char(*c)).collect()
}

fn is_closer(c: char) -> bool {
	matches!(c, '"' | '\'' | ')' | ']' | '}' | '»' | '”' | '’')
}

/// True when the caret sits at the start of a sentence. Trailing spaces are
/// ignored, and a newline or sentence-ending punctuation starts one even
/// behind a closing quote or bracket.
fn starts_sentence(text: &str) -> bool {
	let chars: Vec<char> = text.chars().collect();
	let mut i = chars.len();
	let mut newline = false;
	while i > 0 {
		let c = chars[i - 1];
		if c == '\n' || c == '\r' {
			newline = true;
			i -= 1;
		} else if c == ' ' || c == '\t' || c == '\u{00A0}' || c == '\u{202F}' {
			i -= 1;
		} else {
			break;
		}
	}
	if i == 0 || newline {
		return true;
	}
	if matches!(chars[i - 1], '.' | '!' | '?' | '…') {
		return true;
	}
	if is_closer(chars[i - 1]) {
		while i > 0 && (chars[i - 1].is_whitespace() || is_closer(chars[i - 1])) {
			i -= 1;
		}
		return i == 0 || matches!(chars[i - 1], '.' | '!' | '?' | '…');
	}
	false
}

fn case_pattern(word: &str) -> CasePattern {
	let mut first = true;
	let mut first_upper = false;
	let mut rest_upper = false;
	let mut has_lower = false;
	for c in word.chars().filter(|c| c.is_alphabetic()) {
		if first {
			first_upper = c.is_uppercase();
			first = false;
		} else if c.is_uppercase() {
			rest_upper = true;
		}
		has_lower |= c.is_lowercase();
	}
	let has_upper = first_upper || rest_upper;
	if has_upper && !has_lower {
		CasePattern::AllCaps
	} else if first_upper && !rest_upper {
		CasePattern::Capitalized
	} else if has_upper {
		CasePattern::Mixed
	} else {
		CasePattern::Lower
	}
}

fn apply_case(typed: &str, candidate: &str) -> String {
	match case_pattern(typed) {
		CasePattern::AllCaps => candidate.to_uppercase(),
		CasePattern::Capitalized => {
			let mut chars = candidate.chars();
			match chars.next() {
				Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
				None => String::new(),
			}
		}
		_ => candidate.to_string(),
	}
}

fn letter_json(out: &mut String, letter: &crate::layout::Letter, shifted: bool) {
	let label = if shifted && !letter.shift_label.is_empty() {
		letter.shift_label
	} else {
		letter.label
	};
	out.push_str("{\"id\":");
	json::string_into(out, letter.id);
	out.push_str(",\"label\":");
	json::string_into(out, label);
	out.push_str(",\"commit\":");
	json::string_into(out, letter_text(letter, shifted));
	out.push_str(",\"letter\":");
	json::bool_into(out, letter.letter);
	out.push_str(",\"row\":");
	out.push_str(&letter.row.to_string());
	out.push_str(",\"alts\":[");
	for (i, alt) in letter.alts.iter().enumerate() {
		if i > 0 {
			out.push(',');
		}
		json::string_into(out, alt);
	}
	out.push_str("]}");
}

/// Text committed by a letter, honouring the layout's shifted jamo.
fn letter_text(letter: &Letter, shifted: bool) -> &'static str {
	if shifted && !letter.shift_commit.is_empty() {
		letter.shift_commit
	} else {
		letter.commit
	}
}

/// Number of leading characters two strings share.
fn common_prefix_chars(a: &str, b: &str) -> usize {
	a.chars()
	. zip(b.chars())
	. take_while(|(left, right)| left == right)
	. count()
}

fn kind_id(kind: crate::layout::KeyKind) -> &'static str {
	use crate::layout::KeyKind::*;
	match kind {
		Char => "char",
		Shift => "shift",
		Backspace => "backspace",
		Enter => "enter",
		Space => "space",
		Page => "page",
	}
}

#[cfg(test)]
mod memory_tests {
	use super::*;

	#[test]
	fn correction_memory_keeps_one_typed_form_per_word() {
		let mut memory = CorrectionMemory::default();
		memory.remember("bonjour", "bouiojn");
		memory.remember("Bonjour", "bknjpur");
		assert_eq!(memory.entries.len(), 1);
		assert_eq!(memory.lookup("bonjour"), Some("bknjpur"));
	}

	#[test]
	fn correction_memory_is_bounded() {
		let mut memory = CorrectionMemory::default();
		for index in 0..(MAX_MEMORY_ENTRIES + 10) {
			memory.remember(
				&format!("visible{index}"),
				&format!("typed{index}"),
			);
		}
		assert_eq!(memory.entries.len(), MAX_MEMORY_ENTRIES);
		assert!(memory.lookup("visible0").is_none());
		assert!(
			memory
			. lookup(&format!("visible{}", MAX_MEMORY_ENTRIES + 9))
			. is_some()
		);
	}
}
