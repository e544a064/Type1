//! Keyboard state machine: pages, shift, capitalisation, French spacing,
//! current-word tracking and suggestion selection.

use crate::action::{Action, ImeActionKind};
use crate::json;
use crate::layout::{suggestions_for, Page, ShiftState, STRIP, UTILITIES};

const SUGGESTION_LIMIT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsMode {
	None,
	Characters,
	Words,
	Sentences,
}

#[derive(Debug)]
pub struct Engine {
	page: Page,
	shift: ShiftState,
	caps: CapsMode,
	/// True when the next letter starts a sentence (or a word in Words mode).
	at_start: bool,
	last_char: Option<char>,
	/// Letters committed since the last word boundary.
	word: String,
	ime_action: ImeActionKind,
	multiline: bool,
	numeric: bool,
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
	}

	/// Configure from `EditorInfo`. Flags mirror the Android constants so the
	/// core stays platform independent.
	pub fn set_input_type(&mut self, input_type: i32, ime_options: i32) {
		let class = input_type & 0x0000_000f;
		self.numeric = matches!(class, 0x2 | 0x3 | 0x4);
		self.multiline = class == 0x1 && (input_type & 0x0002_0000) != 0;
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
			"space" => self.commit(" ", false, false),
			"comma" => self.commit(",", false, false),
			"backspace" => {
				self.at_start = false;
				self.last_char = None;
				self.word.pop();
				vec![Action::Delete { word: false }]
			}
			"cursor_left" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::MoveCursor(-1)]
			}
			"cursor_right" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::MoveCursor(1)]
			}
			"select_left" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::ExtendSelection(-1)]
			}
			"select_right" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::ExtendSelection(1)]
			}
			"start_selection" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::StartSelection]
			}
			"delete_selection" => {
				self.word.clear();
				self.last_char = None;
				vec![Action::DeleteSelection]
			}
			"enter" => {
				self.word.clear();
				if self.multiline || self.ime_action == ImeActionKind::None {
					self.at_start = true;
					vec![Action::Commit("\n".into())]
				} else {
					vec![Action::Ime(self.ime_action)]
				}
			}
			_ => match self.page.chars().iter().find(|letter| letter.id == id) {
				Some(letter) => {
					let space_before = matches!(letter.commit, "?" | "!" | ":" | ";");
					self.commit(letter.commit, letter.letter, space_before)
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
		}
		out.push(Action::Commit(rendered.clone()));

		if is_letter {
			self.word.push_str(&rendered);
			if self.shift == ShiftState::Once {
				self.shift = ShiftState::Off;
			}
		} else {
			self.word.clear();
		}
		if let Some(c) = rendered.chars().last() {
			self.note(c);
		}
		out
	}

	fn apply_suggestion(&mut self, raw_index: &str) -> Vec<Action> {
		let index: usize = match raw_index.parse() {
			Ok(value) => value,
			Err(_) => return Vec::new(),
		};
		let candidates = suggestions_for(&self.word, SUGGESTION_LIMIT);
		let chosen = match candidates.get(index) {
			Some(value) => value.clone(),
			None => return Vec::new(),
		};
		let delete = self.word.chars().count() as u32;
		let mut out = Vec::new();
		if delete > 0 {
			out.push(Action::DeleteChars(delete));
		}
		out.push(Action::Commit(format!("{chosen} ")));
		self.word.clear();
		self.note(' ');
		out
	}

	fn note(&mut self, c: char) {
		self.last_char = Some(c);
		match c {
			'.' | '!' | '?' => self.at_start = true,
			' ' => {
				if self.caps == CapsMode::Words {
					self.at_start = true;
				}
			}
			c if c.is_alphanumeric() => {
				if self.caps != CapsMode::Characters {
					self.at_start = false;
				}
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

	/// Full keyboard model for the UI, as JSON.
	pub fn render_json(&self) -> String {
		let suggestions = if self.page == Page::Letters {
			suggestions_for(&self.word, SUGGESTION_LIMIT)
		} else {
			Vec::new()
		};

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
			out.push_str("{\"id\":");
			json::string_into(&mut out, key.id);
			out.push_str(",\"label\":");
			json::string_into(&mut out, label);
			out.push_str(",\"kind\":");
			json::string_into(&mut out, kind_id(key.kind));
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
