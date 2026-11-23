//! Full-interaction regressions.
//!
//! These tests drive the real engine through a whole editing session and apply
//! every emitted action to a small editor model. They cover typing, autocorrect,
//! suggestion taps, punctuation spacing, undo, backspace, page switches and
//! selection deletion, not just single key presses.

use std::sync::OnceLock;

use type1_core::{Action, Engine};

const TEXT: i32 = 0x0001;
const CAP_SENTENCES: i32 = 0x4000;
const MULTILINE: i32 = 0x0002_0001;

struct Rng(u64);

impl Rng {
	fn next(&mut self) -> u64 {
		let mut x = self.0;
		x ^= x << 13;
		x ^= x >> 7;
		x ^= x << 17;
		self.0 = x;
		x
	}

	fn below(&mut self, n: usize) -> usize {
		(self.next() % n as u64) as usize
	}
}

fn parse_cases(source: &'static str) -> Vec<(&'static str, &'static str)> {
	source
	. lines()
	. filter_map(|line| {
		let (typed, expected) = line.split_once('\t')?;
		if typed.is_empty() || expected.is_empty() {
			return None;
		}
		Some((typed, expected))
	})
	. collect()
}

static AUTO_CASES: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();

fn auto_cases() -> &'static [(&'static str, &'static str)] {
	AUTO_CASES.get_or_init(|| parse_cases(include_str!("data/autocorrect_cases.tsv")))
}

static SAFE_WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();

fn is_safe_word(word: &str) -> bool {
	type1_core::dictionary::exact(word).is_some()
	&& (
		type1_core::correction::suggest(word)
		. candidates
		. is_empty()
	)
	&& (
		type1_core::correction::suggest(&capitalize(word))
		. candidates
		. is_empty()
	)
}

/// Common words the corrector leaves alone, including their capitalised form.
fn safe_words() -> &'static [&'static str] {
	SAFE_WORDS.get_or_init(|| {
		const POOL: &[&str] = &[
			"bonjour", "toi", "moi", "oui", "non", "merci", "comment", "maison",
			"test", "la", "le", "les", "un", "une", "des", "du", "de", "et", "je",
			"tu", "il", "elle", "nous", "vous", "ils", "elles", "est", "sont",
			"avec", "pour", "dans", "sur", "pas", "plus", "tout", "bien",
			"toujours", "jamais", "beaucoup", "petit", "grand", "homme", "femme",
			"jour", "soir", "monde", "temps", "vie", "amour", "travail", "encore",
			"aussi", "quand", "alors", "ici", "peut", "faire", "voir", "savoir",
		];
		let words: Vec<&'static str> = (
			POOL
			. iter()
			. copied()
			. filter(|word| is_safe_word(word))
			. collect()
		);
		assert!(words.len() >= 30, "only {} safe words", words.len());
		words
	})
}

fn capitalize(word: &str) -> String {
	let mut chars = word.chars();
	match chars.next() {
		Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
		None => String::new(),
	}
}

#[derive(Clone, Copy, Debug)]
enum Step<'a> {
	/// Press `typed`; the engine should settle on `expected` at the next boundary.
	Word { typed: &'a str, expected: &'a str },
	Space,
	Punct(char),
	/// Keys from the numbers page, inserted directly.
	Digits(&'a str),
}

fn punct_key(c: char) -> &'static str {
	match c {
		',' => "comma",
		'.' => "dot",
		'!' => "sym:!",
		'?' => "sym:?",
		':' => "sym::",
		';' => "sym:;",
		'\'' => "apos",
		_ => "sym:.",
	}
}

fn boundary_text(c: char) -> &'static str {
	match c {
		' ' => " ",
		',' => ",",
		'.' => ".",
		'!' => " !",
		'?' => " ?",
		':' => " :",
		';' => " ;",
		_ => "",
	}
}

fn boundary_step(c: char) -> Step<'static> {
	if c == ' ' { Step::Space } else { Step::Punct(c) }
}

const BOUNDARIES: &[char] = &[' ', ',', '.', '!', '?', ':', ';'];

fn render_steps(
	mut out: String,
	mut last: Option<char>,
	mut at_start: bool,
	caps: bool,
	steps: &[Step<'_>],
) -> String {
	for step in steps {
		match *step {
			Step::Word { expected, .. } => {
				// A letter after any punctuation gets the automatic French space.
				if matches!(
					last,
					Some('.' | ',' | ';' | ':' | '!' | '?')
				) {
					out.push(' ');
				}
				let rendered = if caps && at_start {
					capitalize(expected)
				} else {
					expected.to_string()
				};
				out.push_str(&rendered);
				last = rendered.chars().last();
				if caps {
					at_start = false;
				}
			}
			Step::Space => {
				out.push(' ');
				last = Some(' ');
			}
			Step::Punct(c) => match c {
				',' | '.' => {
					if last == Some(' ') {
						out.pop();
					}
					out.push(c);
					last = Some(c);
					if c == '.' {
						at_start = true;
					}
				}
				'!' | '?' | ':' | ';' => {
					if !matches!(last, None | Some(' ')) {
						out.push(' ');
					}
					out.push(c);
					last = Some(c);
					if matches!(c, '!' | '?') {
						at_start = true;
					}
				}
				_ => {
					out.push(c);
					last = Some(c);
				}
			},
			Step::Digits(text) => {
				out.push_str(text);
				last = text.chars().last();
				at_start = false;
			}
		}
	}
	out
}

#[derive(Default)]
struct Editor {
	chars: Vec<char>,
	caret: usize,
	selection_anchor: Option<usize>,
	selection_width: usize,
	needs_sync: bool,
}

impl Editor {
	fn text(&self) -> String {
		self.chars.iter().collect()
	}

	fn caret(&self) -> usize {
		self.caret
	}

	fn set_text(&mut self, text: &str, caret: usize) {
		self.chars = text.chars().collect();
		self.caret = caret.min(self.chars.len());
		self.selection_anchor = None;
		self.selection_width = 0;
		self.needs_sync = false;
	}

	/// Mirror the platform's context re-read after a caret move or a delete.
	fn sync(&mut self, engine: &mut Engine) {
		let before: String = (
			self.chars[..self.caret]
			. iter()
			. rev()
			. take(64)
			. collect::<Vec<_>>()
			. into_iter()
			. rev()
			. collect()
		);
		let after: String = (
			self.chars[self.caret..]
			. iter()
			. take(64)
			. collect()
		);
		engine.set_context_word(&before, &after);
		self.needs_sync = false;
	}

	fn flush(&mut self, engine: &mut Engine) {
		if self.needs_sync {
			self.sync(engine);
		}
	}

	fn has_selection(&self) -> bool {
		self.selection_width > 0
	}

	fn collapse_selection(&mut self) {
		if let Some(anchor) = self.selection_anchor.take() {
			self.selection_width = 0;
			self.caret = anchor;
		}
	}

	fn commit(&mut self, text: &str) {
		if self.has_selection() {
			self.delete_selection();
		}
		let count = text.chars().count();
		for (offset, ch) in text.chars().enumerate() {
			self.chars.insert(self.caret + offset, ch);
		}
		self.caret += count;
		self.selection_anchor = None;
		self.selection_width = 0;
	}

	fn delete_before(&mut self, count: usize) {
		let start = self.caret.saturating_sub(count);
		self.chars.drain(start..self.caret);
		self.caret = start;
	}

	fn delete_after(&mut self, count: usize) {
		let end = (self.caret + count).min(self.chars.len());
		self.chars.drain(self.caret..end);
	}

	fn delete_selection(&mut self) {
		if !self.has_selection() {
			return;
		}
		let anchor = self.selection_anchor.unwrap_or(self.caret);
		let start = anchor.saturating_sub(self.selection_width);
		self.chars.drain(start..anchor);
		self.caret = start;
		self.selection_anchor = None;
		self.selection_width = 0;
	}

	fn delete_word(&mut self) {
		if self.has_selection() {
			self.delete_selection();
			return;
		}
		let mut index = self.caret;
		while index > 0 && self.chars[index - 1].is_whitespace() {
			index -= 1;
		}
		while index > 0 && !self.chars[index - 1].is_whitespace() {
			index -= 1;
		}
		let count = self.caret - index;
		if count > 0 {
			self.delete_before(count);
		} else {
			self.delete_before(1);
		}
	}

	fn trim_spaces_after_caret(&mut self) {
		for _ in 0..4 {
			match self.chars.get(self.caret) {
				Some(' ') | Some('\u{00A0}') => {
					self.chars.remove(self.caret);
				}
				_ => break,
			}
		}
	}

	fn apply(&mut self, actions: &[Action]) {
		for action in actions {
			match action {
				Action::Commit(text) => self.commit(text),
				Action::CommitSuggestion(text) => {
					self.commit(text);
					self.trim_spaces_after_caret();
				}
				Action::Delete { word } => {
					if *word {
						self.delete_word();
					} else if self.has_selection() {
						self.delete_selection();
					} else {
						self.delete_before(1);
					}
					self.needs_sync = true;
				}
				Action::DeleteChars(count) => {
					if self.has_selection() {
						self.delete_selection();
					} else {
						self.delete_before(*count as usize);
					}
				}
				Action::ReplaceSurrounding {
					before,
					after,
					text,
				} => {
					self.delete_before(*before as usize);
					self.delete_after(*after as usize);
					self.commit(text);
				}
				Action::MoveCursor(delta) => {
					self.collapse_selection();
					let caret = self.caret as i32 + delta;
					self.caret = caret.clamp(0, self.chars.len() as i32) as usize;
					self.needs_sync = true;
				}
				Action::StartSelection => {
					self.collapse_selection();
					self.selection_anchor = Some(self.caret);
					self.selection_width = 0;
				}
				Action::ExtendSelection(delta) => {
					if let Some(anchor) = self.selection_anchor {
						let target = (
							(self.selection_width as i32 - delta)
							. clamp(0, anchor as i32) as usize
						);
						if target != self.selection_width {
							self.selection_width = target;
							self.caret = anchor - target;
						}
					}
				}
				Action::DeleteSelection => {
					self.delete_selection();
					self.needs_sync = true;
				}
				Action::Ime(_) => {}
			}
		}
	}
}

fn press(engine: &mut Engine, editor: &mut Editor, id: &str) {
	let actions = engine.on_key(id);
	editor.apply(&actions);
	editor.flush(engine);
}

fn type_text(engine: &mut Engine, editor: &mut Editor, text: &str) {
	for ch in text.chars() {
		if ch.is_ascii_uppercase() {
			press(engine, editor, "shift");
			let lower = ch.to_ascii_lowercase();
			press(engine, editor, &lower.to_string());
		} else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
			press(engine, editor, &ch.to_string());
		} else if ch == ' ' {
			press(engine, editor, "space");
		} else if ch == '\'' || ch == '’' {
			press(engine, editor, "apos");
		} else {
			// Accented letters and any non-key character stay part of the word.
			press(engine, editor, &format!("alt:{ch}"));
		}
	}
}

fn simulate_steps(engine: &mut Engine, editor: &mut Editor, steps: &[Step<'_>]) {
	for step in steps {
		match *step {
			Step::Word { typed, .. } => type_text(engine, editor, typed),
			Step::Space => press(engine, editor, "space"),
			Step::Punct(c) => press(engine, editor, punct_key(c)),
			Step::Digits(text) => {
				for ch in text.chars() {
					press(engine, editor, &ch.to_string());
				}
			}
		}
	}
}

fn typed_steps(rng: &mut Rng) -> Vec<Step<'static>> {
	let words = safe_words();
	let mut steps = Vec::new();
	if rng.below(12) == 0 {
		steps.push(boundary_step(BOUNDARIES[rng.below(6) + 1]));
		if rng.below(2) == 0 {
			steps.push(Step::Space);
		}
	}
	let count = 1 + rng.below(3);
	for word_index in 0..count {
		let word = words[rng.below(words.len())];
		steps.push(Step::Word {
			typed: word,
			expected: word,
		});
		if word_index + 1 < count {
			match rng.below(10) {
				0 => steps.push(Step::Space),
				1 => steps.push(Step::Punct('.')),
				2 => steps.push(Step::Punct(',')),
				3 => steps.push(Step::Punct('!')),
				4 => steps.push(Step::Punct('?')),
				5 => steps.push(Step::Punct(':')),
				6 => steps.push(Step::Punct(';')),
				7 => {
					steps.push(Step::Space);
					steps.push(Step::Punct(','));
				}
				8 => {
					steps.push(Step::Punct(','));
					steps.push(Step::Space);
				}
				_ => {
					steps.push(Step::Space);
					steps.push(Step::Punct('.'));
				}
			}
		}
	}
	match rng.below(8) {
		0 => steps.push(Step::Space),
		1 => steps.push(Step::Punct('.')),
		2 => steps.push(Step::Punct(',')),
		3 => steps.push(Step::Punct('!')),
		4 => steps.push(Step::Punct('?')),
		5 => steps.push(Step::Punct(':')),
		6 => steps.push(Step::Punct(';')),
		_ => {}
	}
	steps
}

fn typed_spacing_shard(seed: u64, cases: usize) {
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let caps = rng.below(2) == 0;
		let steps = typed_steps(&mut rng);
		let expected = render_steps(String::new(), None, caps, caps, &steps);
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		simulate_steps(&mut engine, &mut editor, &steps);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"typed case {index}: steps {steps:?}\n  expected {expected:?}\n  actual   {actual:?}"
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_typing_spacing_0() {
	typed_spacing_shard(0x1000_0001, 2_500);
}

#[test]
fn full_typing_spacing_1() {
	typed_spacing_shard(0x1000_0002, 2_500);
}

#[test]
fn full_typing_spacing_2() {
	typed_spacing_shard(0x1000_0003, 2_500);
}

#[test]
fn full_typing_spacing_3() {
	typed_spacing_shard(0x1000_0004, 2_500);
}

fn autocorrect_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, expected_word) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let boundary = BOUNDARIES[rng.below(BOUNDARIES.len())];
		let steps = vec![
			Step::Word {
				typed,
				expected: expected_word,
			},
			boundary_step(boundary),
		];
		let expected = render_steps(String::new(), None, caps, caps, &steps);
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		simulate_steps(&mut engine, &mut editor, &steps);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"autocorrect case {}: typed {typed:?}, boundary {boundary:?}\n  expected {expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_autocorrect_boundaries_0() {
	autocorrect_shard(0x2000_0001, 0, 1_200);
}

#[test]
fn full_autocorrect_boundaries_1() {
	autocorrect_shard(0x2000_0002, 1_200, 1_200);
}

#[test]
fn full_autocorrect_boundaries_2() {
	autocorrect_shard(0x2000_0003, 2_400, 1_200);
}

fn suffix_patterns() -> &'static [Vec<Step<'static>>] {
	static PATTERNS: OnceLock<Vec<Vec<Step<'static>>>> = OnceLock::new();
	PATTERNS.get_or_init(|| {
		vec![
			vec![
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "toj",
					expected: "toi",
				},
				Step::Punct('!'),
			],
			vec![
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
				Step::Punct('?'),
			],
			vec![
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
				Step::Punct('.'),
			],
			vec![
				Step::Punct('.'),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
				Step::Punct('!'),
			],
			vec![
				Step::Punct('!'),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
			],
			vec![
				Step::Punct('?'),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
			],
			vec![Step::Word {
				typed: "toi",
				expected: "toi",
			}],
			vec![
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "bknjpur",
					expected: "bonjour",
				},
				Step::Punct('!'),
			],
			vec![
				Step::Punct(';'),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
			],
			vec![
				Step::Punct(':'),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
			],
			vec![
				Step::Punct('!'),
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
			],
			vec![
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "toi",
					expected: "toi",
				},
				Step::Punct(','),
				Step::Space,
				Step::Word {
					typed: "oui",
					expected: "oui",
				},
				Step::Punct('!'),
			],
		]
	})
}

fn suggestion_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let patterns = suffix_patterns();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, expected_word) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let pattern = &patterns[rng.below(patterns.len())];
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();

		type_text(&mut engine, &mut editor, typed);
		press(&mut engine, &mut editor, "sug:0");
		simulate_steps(&mut engine, &mut editor, pattern);

		let base_word = if caps {
			capitalize(expected_word)
		} else {
			expected_word.to_string()
		};
		let expected = render_steps(
			format!("{base_word} "),
			Some(' '),
			false,
			caps,
			pattern,
		);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"suggestion case {}: typed {typed:?}, pattern {pattern:?}\n  expected {expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_suggestion_suffix_0() {
	suggestion_shard(0x3000_0001, 0, 1_200);
}

#[test]
fn full_suggestion_suffix_1() {
	suggestion_shard(0x3000_0002, 1_200, 1_200);
}

#[test]
fn full_suggestion_suffix_2() {
	suggestion_shard(0x3000_0003, 2_400, 1_200);
}

fn undo_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, expected_word) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let boundary = BOUNDARIES[rng.below(BOUNDARIES.len())];
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();

		let steps = vec![
			Step::Word {
				typed,
				expected: expected_word,
			},
			boundary_step(boundary),
		];
		simulate_steps(&mut engine, &mut editor, &steps);
		let corrected = render_steps(String::new(), None, caps, caps, &steps);
		if editor.text() != corrected {
			failures.push(format!(
				"undo case {} before undo: typed {typed:?}, boundary {boundary:?}\n  expected {corrected:?}\n  actual   {:?}",
				offset + index,
				editor.text()
			));
			if failures.len() >= 8 {
				break;
			}
			continue;
		}

		press(&mut engine, &mut editor, "backspace");
		let original = if caps {
			capitalize(typed)
		} else {
			typed.to_string()
		};
		let boundary = boundary_text(boundary);
		let expected = format!("{original}{boundary}");
		let actual = editor.text();
		let caret = editor.caret();
		if actual != expected || caret != original.chars().count() {
			failures.push(format!(
				"undo case {} after undo: typed {typed:?}, boundary {boundary:?}\n  expected {expected:?} caret {}\n  actual   {actual:?} caret {caret}",
				offset + index,
				original.chars().count()
			));
			if failures.len() >= 8 {
				break;
			}
			continue;
		}

		let follow = rng.below(3);
		let (follow_key, follow_expected) = match follow {
			0 => (
				"comma",
				format!("{original},"),
			),
			1 => (
				"space",
				if boundary == " " {
					format!("{original} ")
				} else {
					format!("{original}{boundary} ")
				},
			),
			_ => ("sym:!", format!("{original} !")),
		};
		press(&mut engine, &mut editor, follow_key);
		let actual = editor.text();
		if actual != follow_expected {
			failures.push(format!(
				"undo case {} follow-up: typed {typed:?}, boundary {boundary:?}, follow {follow_key:?}\n  expected {follow_expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_backspace_undo_0() {
	undo_shard(0x4000_0001, 0, 1_200);
}

#[test]
fn full_backspace_undo_1() {
	undo_shard(0x4000_0002, 1_200, 1_200);
}

#[test]
fn full_backspace_undo_2() {
	undo_shard(0x4000_0003, 2_400, 1_200);
}

fn recorrection_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, expected_word) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let text = if caps {
			capitalize(typed)
		} else {
			typed.to_string()
		};
		let caret = rng.below(text.chars().count() + 1);
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		editor.set_text(&text, caret);
		editor.sync(&mut engine);
		press(&mut engine, &mut editor, "sug:0");

		let expected = if caps {
			capitalize(expected_word)
		} else {
			expected_word.to_string()
		};
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"recorrection case {}: typed {typed:?}, caret {caret}\n  expected {expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_recorrection_0() {
	recorrection_shard(0x5000_0001, 0, 1_200);
}

#[test]
fn full_recorrection_1() {
	recorrection_shard(0x5000_0002, 1_200, 1_200);
}

fn selection_shard(seed: u64, cases: usize) {
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let caps = rng.below(2) == 0;
		let steps = typed_steps(&mut rng);
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		simulate_steps(&mut engine, &mut editor, &steps);
		let before = editor.text();
		if before.chars().count() < 2 {
			continue;
		}
		let count = 1 + rng.below(before.chars().count().min(5));
		press(&mut engine, &mut editor, "start_selection");
		for _ in 0..count {
			press(&mut engine, &mut editor, "select_left");
		}
		press(&mut engine, &mut editor, "delete_selection");
		let expected: String = (
			before
			. chars()
			. take(before.chars().count() - count)
			. collect()
		);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"selection case {index}: before {before:?}, delete {count}\n  expected {expected:?}\n  actual   {actual:?}"
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_selection_delete_0() {
	selection_shard(0x6000_0001, 1_000);
}

#[test]
fn full_selection_delete_1() {
	selection_shard(0x6000_0002, 1_000);
}

#[derive(Clone, Copy, Debug)]
enum PageStep<'a> {
	Text(&'a str),
	Space,
	Comma,
	Dot,
	Page,
	Digits(&'a str),
	Symbol(char),
	Sym(char),
}

const DIGIT_RUNS: &[&str] = &["1", "2", "3", "7", "0", "12", "42", "007", "2026", "5"];
const PAGE_SYMBOLS: &[char] = &['!', '?', ':', ';', ',', '.'];

fn symbol_key(c: char) -> &'static str {
	match c {
		'!' => "excl",
		'?' => "question",
		':' => "colon",
		';' => "semicolon",
		',' => "comma",
		'.' => "dot",
		_ => unreachable!("unsupported page symbol {c}"),
	}
}

fn simulate_page_steps(engine: &mut Engine, editor: &mut Editor, steps: &[PageStep<'_>]) {
	for step in steps {
		match *step {
			PageStep::Text(text) => type_text(engine, editor, text),
			PageStep::Space => press(engine, editor, "space"),
			PageStep::Comma => press(engine, editor, "comma"),
			PageStep::Dot => press(engine, editor, "dot"),
			PageStep::Page => press(engine, editor, "page"),
			PageStep::Digits(text) => {
				for ch in text.chars() {
					press(engine, editor, &ch.to_string());
				}
			}
			PageStep::Symbol(c) => press(engine, editor, symbol_key(c)),
			PageStep::Sym(c) => press(engine, editor, punct_key(c)),
		}
	}
}

fn page_case(rng: &mut Rng) -> (Vec<PageStep<'static>>, Vec<Step<'static>>) {
	let words = safe_words();
	let mut sim = Vec::new();
	let mut exp = Vec::new();
	let first = words[rng.below(words.len())];
	sim.push(PageStep::Text(first));
	exp.push(Step::Word {
		typed: first,
		expected: first,
	});
	match rng.below(3) {
		0 => {
			sim.push(PageStep::Space);
			exp.push(Step::Space);
		}
		1 => {
			sim.push(PageStep::Comma);
			exp.push(Step::Punct(','));
		}
		_ => {}
	}

	let digits = DIGIT_RUNS[rng.below(DIGIT_RUNS.len())];
	let symbol = PAGE_SYMBOLS[rng.below(PAGE_SYMBOLS.len())];
	let symbol2 = PAGE_SYMBOLS[rng.below(PAGE_SYMBOLS.len())];
	match rng.below(5) {
		// Letters -> numbers -> symbols -> letters, one symbol.
		0 => {
			sim.push(PageStep::Page);
			sim.push(PageStep::Digits(digits));
			exp.push(Step::Digits(digits));
			sim.push(PageStep::Page);
			sim.push(PageStep::Symbol(symbol));
			exp.push(Step::Punct(symbol));
			sim.push(PageStep::Page);
		}
		// Letters -> numbers -> symbols -> letters, digits only.
		1 => {
			sim.push(PageStep::Page);
			sim.push(PageStep::Digits(digits));
			exp.push(Step::Digits(digits));
			sim.push(PageStep::Page);
			sim.push(PageStep::Page);
		}
		// Letters -> numbers -> symbols -> letters, one symbol, no digits.
		2 => {
			sim.push(PageStep::Page);
			sim.push(PageStep::Page);
			sim.push(PageStep::Symbol(symbol));
			exp.push(Step::Punct(symbol));
			sim.push(PageStep::Page);
		}
		// Long-press shortcut: a symbol without changing page.
		3 => {
			sim.push(PageStep::Sym(symbol));
			exp.push(Step::Punct(symbol));
		}
		// Two symbols after digits.
		_ => {
			sim.push(PageStep::Page);
			sim.push(PageStep::Digits(digits));
			exp.push(Step::Digits(digits));
			sim.push(PageStep::Page);
			sim.push(PageStep::Symbol(symbol));
			exp.push(Step::Punct(symbol));
			sim.push(PageStep::Symbol(symbol2));
			exp.push(Step::Punct(symbol2));
			sim.push(PageStep::Page);
		}
	}

	match rng.below(4) {
		0 => {
			sim.push(PageStep::Space);
			exp.push(Step::Space);
		}
		1 => {
			sim.push(PageStep::Comma);
			exp.push(Step::Punct(','));
		}
		2 => {
			sim.push(PageStep::Dot);
			exp.push(Step::Punct('.'));
		}
		_ => {}
	}
	let last = words[rng.below(words.len())];
	sim.push(PageStep::Text(last));
	exp.push(Step::Word {
		typed: last,
		expected: last,
	});
	(sim, exp)
}

fn page_shard(seed: u64, cases: usize) {
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let caps = rng.below(2) == 0;
		let (sim, exp) = page_case(&mut rng);
		let expected = render_steps(String::new(), None, caps, caps, &exp);
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		simulate_page_steps(&mut engine, &mut editor, &sim);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"page case {index}: steps {sim:?}\n  expected {expected:?}\n  actual   {actual:?}"
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_pages_and_digits_0() {
	page_shard(0x7000_0001, 600);
}

#[test]
fn full_pages_and_digits_1() {
	page_shard(0x7000_0002, 600);
}

fn suggestion_backspace_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, expected_word) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();

		type_text(&mut engine, &mut editor, typed);
		press(&mut engine, &mut editor, "sug:0");
		let word = if caps {
			capitalize(expected_word)
		} else {
			expected_word.to_string()
		};
		let after_suggestion = format!("{word} ");
		if editor.text() != after_suggestion {
			failures.push(format!(
				"suggestion-backspace case {} after tap: typed {typed:?}\n  expected {after_suggestion:?}\n  actual   {:?}",
				offset + index,
				editor.text()
			));
			if failures.len() >= 8 {
				break;
			}
			continue;
		}

		press(&mut engine, &mut editor, "backspace");
		let after_delete = word.clone();
		if editor.text() != after_delete {
			failures.push(format!(
				"suggestion-backspace case {} after delete: typed {typed:?}\n  expected {after_delete:?}\n  actual   {:?}",
				offset + index,
				editor.text()
			));
			if failures.len() >= 8 {
				break;
			}
			continue;
		}

		let (follow_key, expected) = match rng.below(3) {
			0 => ("comma", format!("{word},")),
			1 => ("space", format!("{word} ")),
			_ => ("sym:!", format!("{word} !")),
		};
		press(&mut engine, &mut editor, follow_key);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"suggestion-backspace case {} after {follow_key}: typed {typed:?}\n  expected {expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_suggestion_backspace_0() {
	suggestion_backspace_shard(0x8000_0001, 0, 600);
}

#[test]
fn full_suggestion_backspace_1() {
	suggestion_backspace_shard(0x8000_0002, 600, 600);
}

fn suggestion_choice_shard(seed: u64, offset: usize, cases: usize) {
	let corpus = auto_cases();
	let patterns = suffix_patterns();
	let mut rng = Rng(seed);
	let mut failures = Vec::new();
	for index in 0..cases {
		let (typed, _) = corpus[(offset + index) % corpus.len()];
		let caps = rng.below(2) == 0;
		let mut engine = Engine::new();
		engine.set_input_type(if caps { TEXT | CAP_SENTENCES } else { TEXT }, 0);
		let mut editor = Editor::default();
		type_text(&mut engine, &mut editor, typed);

		let json: serde_json::Value =
			serde_json::from_str(&engine.render_json()).unwrap();
		let suggestions: Vec<String> = (
			json["suggestions"]
			. as_array()
			. unwrap()
			. iter()
			. map(|value| value.as_str().unwrap().to_string())
			. collect()
		);
		if suggestions.is_empty() {
			continue;
		}
		let choice = rng.below(suggestions.len());
		let chosen = suggestions[choice].clone();
		press(&mut engine, &mut editor, &format!("sug:{choice}"));
		let pattern = &patterns[rng.below(patterns.len())];
		simulate_steps(&mut engine, &mut editor, pattern);
		let expected = render_steps(
			format!("{chosen} "),
			Some(' '),
			false,
			caps,
			pattern,
		);
		let actual = editor.text();
		if actual != expected {
			failures.push(format!(
				"suggestion-choice case {}: typed {typed:?}, choice {choice} ({chosen:?}), pattern {pattern:?}\n  expected {expected:?}\n  actual   {actual:?}",
				offset + index
			));
			if failures.len() >= 8 {
				break;
			}
		}
	}
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn full_suggestion_choice_0() {
	suggestion_choice_shard(0x9000_0001, 0, 600);
}

#[test]
fn full_suggestion_choice_1() {
	suggestion_choice_shard(0x9000_0002, 600, 600);
}

#[test]
fn full_example_typo_comma_word_bang() {
	let steps = vec![
		Step::Word {
			typed: "bknjpur",
			expected: "bonjour",
		},
		Step::Punct(','),
		Step::Space,
		Step::Word {
			typed: "toj",
			expected: "toi",
		},
		Step::Punct('!'),
	];
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	let mut editor = Editor::default();
	simulate_steps(&mut engine, &mut editor, &steps);
	assert_eq!(editor.text(), "Bonjour, toi !");
}

#[test]
fn full_example_suggestion_then_comma_word_bang() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	let mut editor = Editor::default();
	type_text(&mut engine, &mut editor, "bknjpur");
	press(&mut engine, &mut editor, "sug:0");
	let suffix = vec![
		Step::Punct(','),
		Step::Space,
		Step::Word {
			typed: "toj",
			expected: "toi",
		},
		Step::Punct('!'),
	];
	simulate_steps(&mut engine, &mut editor, &suffix);
	assert_eq!(editor.text(), "Bonjour, toi !");
}

#[test]
fn full_newline_capitalisation() {
	let corpus = auto_cases();
	for (index, (typed, expected_word)) in corpus.iter().take(200).enumerate() {
		let mut engine = Engine::new();
		engine.set_input_type(TEXT | CAP_SENTENCES | MULTILINE, 0);
		let mut editor = Editor::default();
		type_text(&mut engine, &mut editor, typed);
		press(&mut engine, &mut editor, "enter");
		type_text(&mut engine, &mut editor, "toi");
		press(&mut engine, &mut editor, "sym:!");
		let expected = format!("{}\nToi !", capitalize(expected_word));
		let actual = editor.text();
		assert_eq!(actual, expected, "newline case {index}: typed {typed:?}");
	}
}
