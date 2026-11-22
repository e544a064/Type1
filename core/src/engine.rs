//! Keyboard state machine: pages, shift, capitalisation, French spacing,
//! current-word tracking, correction and suggestions.

use crate::action::{Action, ImeActionKind};
use crate::correction::{self, Suggestion};
use crate::json;
use crate::layout::{Page, ShiftState, STRIP, UTILITIES};

const STRIP_SUGGESTIONS: usize = 3;

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

#[derive(Debug)]
pub struct Engine {
	page: Page,
	shift: ShiftState,
	caps: CapsMode,
	/// True when the next letter starts a sentence (or a word in Words mode).
	at_start: bool,
	last_char: Option<char>,
	/// Letters (and inner apostrophes) committed since the last boundary.
	word: String,
	ime_action: ImeActionKind,
	multiline: bool,
	numeric: bool,
	suggestions_enabled: bool,
	last_correction: Option<CorrectionRecord>,
	/// Boundary text left just after the caret when a correction is undone.
	pending_boundary: Option<String>,
	/// The current word was restored by an undo; do not correct it again until
	/// the user edits it.
	reverted: bool,
	suggestion_cache: Option<(String, Suggestion)>,
}

impl Default for Engine {
	fn default() -> Self {
		Self::new()
	}
}

impl Engine {
	pub fn new() -> Self {
		Engine {
			page: Page::Letters,
			shift: ShiftState::Off,
			caps: CapsMode::Sentences,
			at_start: true,
			last_char: None,
			word: String::new(),
			ime_action: ImeActionKind::None,
			multiline: false,
			numeric: false,
			suggestions_enabled: true,
			last_correction: None,
			pending_boundary: None,
			reverted: false,
			suggestion_cache: None,
		}
	}

	pub fn page(&self) -> Page {
		self.page
	}

	pub fn reset(&mut self) {
		self.shift = ShiftState::Off;
		self.at_start = true;
		self.last_char = None;
		self.word.clear();
		self.last_correction = None;
		self.pending_boundary = None;
		self.reverted = false;
		self.suggestion_cache = None;
	}

	/// Configure from `EditorInfo`. Flags mirror the Android constants so the
	/// core stays platform independent.
	pub fn set_input_type(&mut self, input_type: i32, ime_options: i32) {
		let class = input_type & 0x0000_000f;
		let variation = input_type & 0x0000_0ff0;
		let blocked_variation = matches!(
			variation,
			0x10 | 0x20 | 0x30 | 0x60 | 0x70 | 0x80 | 0x90 | 0xd0 | 0xe0
		);
		let no_suggestions = input_type & 0x0008_0000 != 0;

		self.numeric = matches!(class, 0x2..=0x4);
		self.multiline = class == 0x1 && (input_type & 0x0002_0000) != 0;
		self.suggestions_enabled = class == 0x1 && !blocked_variation && !no_suggestions;
		self.caps = if input_type & 0x0000_1000 != 0 {
			CapsMode::Characters
		} else if input_type & 0x0000_2000 != 0 {
			CapsMode::Words
		} else if input_type & 0x0000_4000 != 0 {
			CapsMode::Sentences
		} else {
			CapsMode::None
		};
		self.ime_action = ImeActionKind::from_ime_options(ime_options);

		self.page = if self.numeric {
			Page::Numbers
		} else {
			Page::Letters
		};
		self.reset();
	}

	/// The editor selection moved: a pending correction can no longer be undone.
	pub fn selection_changed(&mut self) {
		self.last_correction = None;
		self.pending_boundary = None;
		self.reverted = false;
	}

	/// Handle a key press. Returns the editor actions to apply, in order.
	/// The UI always re-renders afterwards.
	pub fn on_key(&mut self, id: &str) -> Vec<Action> {
		if let Some(alt) = id.strip_prefix("alt:") {
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
			_ => match self.page.chars().iter().find(|letter| letter.id == id) {
				Some(letter) if letter.id == "apos" => self.commit_apostrophe(),
				Some(letter) if !letter.letter && self.page == Page::Symbols => {
					let space_before = matches!(letter.commit, "?" | "!" | ":" | ";");
					self.finish_word(letter.commit, space_before)
				}
				Some(letter) => self.commit(letter.commit, letter.letter, false),
				None => Vec::new(),
			},
		}
	}

	/// Commit a character, applying shift, auto-capitalisation and French
	/// spacing (a space is required before `? ! : ;`, and after sentence
	/// punctuation when a new word starts).
	fn commit(&mut self, text: &str, is_letter: bool, space_before: bool) -> Vec<Action> {
		let mut out = Vec::new();

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
			self.word.push_str(&rendered);
			self.reverted = false;
			if self.shift == ShiftState::Once {
				self.shift = ShiftState::Off;
			}
		} else {
			self.word.clear();
			self.last_correction = None;
		}
		self.suggestion_cache = None;
		if let Some(c) = rendered.chars().last() {
			self.note(c);
		}
		out
	}

	/// An apostrophe inside a word continues the token (`j'ai`).
	fn commit_apostrophe(&mut self) -> Vec<Action> {
		self.last_correction = None;
		if self.word.is_empty() {
			return self.commit("'", false, false);
		}
		self.word.push('\'');
		self.last_char = Some('\'');
		self.reverted = false;
		self.suggestion_cache = None;
		vec![Action::Commit("'".into())]
	}

	fn enter(&mut self) -> Vec<Action> {
		if self.multiline || self.ime_action == ImeActionKind::None {
			return self.finish_word("\n", false);
		}

		let mut out = Vec::new();
		let correction = if self.suggestions_enabled {
			self.try_correction()
		} else {
			None
		};
		if let Some((corrected, _)) = correction {
			let original = std::mem::take(&mut self.word);
			out.push(Action::DeleteChars(original.chars().count() as u32));
			out.push(Action::Commit(corrected.clone()));
			self.note_text(&corrected);
		}
		self.word.clear();
		self.last_correction = None;
		self.pending_boundary = None;
		self.reverted = false;
		self.suggestion_cache = None;
		out.push(Action::Ime(self.ime_action));
		out
	}

	/// Correct the current word (when possible) and commit a word boundary.
	fn finish_word(&mut self, boundary: &str, space_before: bool) -> Vec<Action> {
		let mut out = Vec::new();
		// After an undo the user's boundary is still in the editor, just after
		// the caret. Reuse a kept space, or step over kept punctuation.
		let pending = self.pending_boundary.take();
		let keep_space = pending.as_deref() == Some(" ");
		let reuse_space = boundary == " " && keep_space;
		let step_over = boundary == " " && pending.is_some() && !keep_space;

		let leading_space = space_before && !matches!(self.last_char, Some(' ') | None);
		let boundary_text = if leading_space {
			format!(" {boundary}")
		} else {
			boundary.to_string()
		};

		// Correct first, while the caret still sits right after the typed word.
		let correction = if self.suggestions_enabled {
			self.try_correction()
		} else {
			None
		};
		if let Some((corrected, alternatives)) = correction {
			let original = std::mem::take(&mut self.word);
			out.push(Action::DeleteChars(original.chars().count() as u32));
			out.push(Action::Commit(corrected.clone()));
			self.note_text(&corrected);
			self.last_correction = Some(CorrectionRecord {
				original,
				corrected,
				boundary: boundary_text.clone(),
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
				out.push(Action::Commit(" ".into()));
				self.note(' ');
			} else if !boundary_text.is_empty() {
				if leading_space {
					out.push(Action::Commit(" ".into()));
					self.note(' ');
				}
				out.push(Action::Commit(boundary.to_string()));
				for c in boundary.chars() {
					self.note(c);
				}
			}
		} else if !boundary_text.is_empty() {
			if leading_space {
				out.push(Action::Commit(" ".into()));
				self.note(' ');
			}
			out.push(Action::Commit(boundary.to_string()));
			for c in boundary.chars() {
				self.note(c);
			}
		}
		self.word.clear();
		self.reverted = false;
		self.suggestion_cache = None;
		out
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
		let pattern = case_pattern(&typed);
		if matches!(pattern, CasePattern::AllCaps | CasePattern::Mixed) {
			return None;
		}

		let suggestion = self.suggestion_for(&typed);
		if !suggestion.apply {
			return None;
		}
		let best = suggestion.candidates.first()?;
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

	fn suggestion_for(&mut self, typed: &str) -> Suggestion {
		if let Some((cached, suggestion)) = &self.suggestion_cache {
			if cached == typed {
				return suggestion.clone();
			}
		}
		let suggestion = correction::suggest(typed);
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
				self.word.clear();
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

		let delete = self.word.chars().count() as u32;
		let mut out = Vec::new();
		self.consume_pending_boundary(&mut out);
		if delete > 0 {
			out.push(Action::DeleteChars(delete));
		}
		out.push(Action::Commit(format!("{chosen} ")));
		self.word.clear();
		self.last_correction = None;
		self.reverted = false;
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
				self.last_char = self.word.chars().last();
				self.at_start = false;
				self.reverted = true;
				self.suggestion_cache = None;
				return out;
			}
		}

		self.at_start = false;
		self.last_char = None;
		self.word.pop();
		self.reverted = false;
		self.suggestion_cache = None;
		vec![Action::Delete { word: false }]
	}

	fn current_suggestions(&mut self) -> Vec<String> {
		if !self.suggestions_enabled {
			return Vec::new();
		}
		if !self.word.is_empty() {
			let typed = self.word.clone();
			if typed.chars().count() < 2 {
				return Vec::new();
			}
			if matches!(
				case_pattern(&typed),
				CasePattern::AllCaps | CasePattern::Mixed
			) {
				return Vec::new();
			}
			let suggestion = self.suggestion_for(&typed);
			return suggestion
			.candidates
			.iter()
			.take(STRIP_SUGGESTIONS)
			.map(|candidate| apply_case(&typed, &candidate.text))
			.collect();
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
		self.last_char = None;
		self.last_correction = None;
		self.pending_boundary = None;
		self.reverted = false;
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

	fn enter_label(&self) -> &'static str {
		if self.multiline || self.ime_action == ImeActionKind::None {
			"⏎"
		} else {
			self.ime_action.key_label()
		}
	}

	fn enter_icon(&self) -> &'static str {
		if self.multiline || self.ime_action == ImeActionKind::None {
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
		let suggestions = self.current_suggestions();

		let mut out = String::with_capacity(1800);
		out.push_str("{\"page\":");
		json::string_into(&mut out, self.page.id());
		out.push_str(",\"shift\":");
		out.push_str(&self.shift.code().to_string());
		out.push_str(",\"caps\":");
		json::bool_into(&mut out, self.shift != ShiftState::Off || self.auto_caps());
		out.push_str(",\"suggestions\":[");
		for (i, suggestion) in suggestions.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			json::string_into(&mut out, suggestion);
		}
		out.push_str("],\"letters\":[");
		for (i, letter) in self.page.chars().iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			letter_json(&mut out, letter);
		}
		out.push_str("],\"strip\":[");
		for (i, key) in STRIP.iter().enumerate() {
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
		for (i, key) in UTILITIES.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			let label = match key.id {
				"page" => self.page.switch_label(),
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
			out.push_str(",\"w\":");
			json::float_into(&mut out, key.weight);
			out.push_str(",\"on\":");
			json::bool_into(&mut out, false);
			out.push('}');
		}
		out.push_str("]}");
		out
	}
}

fn case_pattern(word: &str) -> CasePattern {
	let mut first_alphabetic = true;
	let mut first_upper = false;
	let mut rest_upper = false;
	let mut has_upper = false;
	let mut has_lower = false;
	for c in word.chars() {
		if !c.is_alphabetic() {
			continue;
		}
		if first_alphabetic {
			first_upper = c.is_uppercase();
			first_alphabetic = false;
		} else if c.is_uppercase() {
			rest_upper = true;
		}
		if c.is_uppercase() {
			has_upper = true;
		}
		if c.is_lowercase() {
			has_lower = true;
		}
	}
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
	if case_pattern(typed) == CasePattern::Capitalized {
		let mut chars = candidate.chars();
		match chars.next() {
			Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
			None => String::new(),
		}
	} else {
		candidate.to_string()
	}
}

fn letter_json(out: &mut String, letter: &crate::layout::Letter) {
	out.push_str("{\"id\":");
	json::string_into(out, letter.id);
	out.push_str(",\"label\":");
	json::string_into(out, letter.label);
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
