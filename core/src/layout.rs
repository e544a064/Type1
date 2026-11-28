//! Declarative layout for the one-line keyboard.
//!
//! The middle band holds every letter on a single horizontal line. Letters are
//! grouped by physical columns (AZERTY `A Q W`, `Z S X`, ...; QWERTY `Q A Z`,
//! `W S X`, ...), so the first letter of each group is the top-row letter and
//! the line reads `AZERTYUIOP` / `QWERTYUIOP` from left to right.
//!
//! The bottom band holds the utility keys (page, shift, space, comma, delete,
//! enter). The suggestion strip is rendered by the UI from the engine payload.

use crate::json;
use crate::script::Script;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
	Char,
	Shift,
	Backspace,
	Enter,
	Space,
	Page,
}

/// One character in the middle band.
#[derive(Debug, Clone, Copy)]
pub struct Letter {
	pub id: &'static str,
	pub label: &'static str,
	pub commit: &'static str,
	/// Follows shift / auto-capitalisation.
	pub letter: bool,
	/// Physical keyboard row: 0 top, 1 home, 2 bottom. Used for the vertical
	/// offset of the glyph inside the single line.
	pub row: u8,
	pub alts: &'static [&'static str],
	/// Text committed when shift is active. Empty keeps [`Self::commit`].
	pub shift_commit: &'static str,
	/// Label shown when shift is active. Empty keeps [`Self::label`].
	pub shift_label: &'static str,
}

const fn l(
	id: &'static str,
	label: &'static str,
	row: u8,
	alts: &'static [&'static str],
) -> Letter {
	Letter {
		id,
		label,
		commit: id,
		letter: true,
		row,
		alts,
		shift_commit: "",
		shift_label: "",
	}
}

const fn c(id: &'static str, label: &'static str, row: u8) -> Letter {
	Letter {
		id,
		label,
		commit: label,
		letter: false,
		row,
		alts: &[],
		shift_commit: "",
		shift_label: "",
	}
}

/// A Hangul key. `commit` and `shift_commit` are compatibility jamo.
const fn j(
	id: &'static str,
	label: &'static str,
	commit: &'static str,
	row: u8,
	shift_label: &'static str,
	shift_commit: &'static str,
) -> Letter {
	ja(id, label, commit, row, shift_label, shift_commit, &[])
}

/// A Hangul key with extra long-press jamo, such as the tense consonants.
const fn ja(
	id: &'static str,
	label: &'static str,
	commit: &'static str,
	row: u8,
	shift_label: &'static str,
	shift_commit: &'static str,
	alts: &'static [&'static str],
) -> Letter {
	Letter {
		id,
		label,
		commit,
		letter: true,
		row,
		alts,
		shift_commit,
		shift_label,
	}
}

/// The 26 letters in AZERTY column order: top, home, bottom, left to right.
/// Reading only the top-row letters gives `AZERTYUIOP`.
pub static LETTERS: &[Letter] = &[
	l("a", "A", 0, &["à", "â", "ä", "á"]),
	l("q", "Q", 1, &[]),
	l("w", "W", 2, &[]),
	l("z", "Z", 0, &[]),
	l("s", "S", 1, &["ß"]),
	l("x", "X", 2, &[]),
	l("e", "E", 0, &["é", "è", "ê", "ë"]),
	l("d", "D", 1, &[]),
	l("c", "C", 2, &["ç"]),
	l("r", "R", 0, &[]),
	l("f", "F", 1, &[]),
	l("v", "V", 2, &[]),
	l("t", "T", 0, &[]),
	l("g", "G", 1, &[]),
	l("b", "B", 2, &[]),
	l("y", "Y", 0, &["ÿ"]),
	l("h", "H", 1, &[]),
	l("n", "N", 2, &["ñ"]),
	l("u", "U", 0, &["ù", "û", "ü", "ú"]),
	l("j", "J", 1, &[]),
	l("i", "I", 0, &["î", "ï", "í"]),
	l("k", "K", 1, &[]),
	l("o", "O", 0, &["ô", "ö", "œ", "ó"]),
	l("l", "L", 1, &[]),
	l("p", "P", 0, &[]),
	l("m", "M", 1, &[]),
];

/// The 26 letters in QWERTY column order: top, home, bottom, left to right.
/// Reading only the top-row letters gives `QWERTYUIOP`.
pub static QWERTY_LETTERS: &[Letter] = &[
	l("q", "Q", 0, &[]),
	l("a", "A", 1, &["à", "â", "ä"]),
	l("z", "Z", 2, &[]),
	l("w", "W", 0, &[]),
	l("s", "S", 1, &["ß"]),
	l("x", "X", 2, &[]),
	l("e", "E", 0, &["é", "è", "ê", "ë"]),
	l("d", "D", 1, &[]),
	l("c", "C", 2, &["ç"]),
	l("r", "R", 0, &[]),
	l("f", "F", 1, &[]),
	l("v", "V", 2, &[]),
	l("t", "T", 0, &[]),
	l("g", "G", 1, &[]),
	l("b", "B", 2, &[]),
	l("y", "Y", 0, &["ÿ"]),
	l("h", "H", 1, &[]),
	l("n", "N", 2, &["ñ"]),
	l("u", "U", 0, &["ù", "û", "ü", "ú"]),
	l("j", "J", 1, &[]),
	l("m", "M", 2, &[]),
	l("i", "I", 0, &["î", "ï", "í"]),
	l("k", "K", 1, &[]),
	l("o", "O", 0, &["ô", "ö", "œ", "ó"]),
	l("l", "L", 1, &[]),
	l("p", "P", 0, &[]),
];

/// Sebeolsik Final (세벌식 최종) in the same physical column order.
///
/// Reading the top-row keys gives `ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ`, the home-row keys give
/// `ㅁㄴㅇㄹㅎㅗㅓㅏㅣ`, and the bottom-row keys give `ㅋㅌㅊㅍㅠㅜㅡ`.
/// Shift adds the tense consonants and `ㅒ`/`ㅖ`.
pub static SEBEOLSIK_LETTERS: &[Letter] = &[
	ja("ko_q", "ㅂ", "ㅂ", 0, "ㅃ", "ㅃ", &["ㅃ"]),
	j("ko_a", "ㅁ", "ㅁ", 1, "", ""),
	j("ko_z", "ㅋ", "ㅋ", 2, "", ""),
	ja("ko_w", "ㅈ", "ㅈ", 0, "ㅉ", "ㅉ", &["ㅉ"]),
	j("ko_s", "ㄴ", "ㄴ", 1, "", ""),
	j("ko_x", "ㅌ", "ㅌ", 2, "", ""),
	ja("ko_e", "ㄷ", "ㄷ", 0, "ㄸ", "ㄸ", &["ㄸ"]),
	j("ko_d", "ㅇ", "ㅇ", 1, "", ""),
	j("ko_c", "ㅊ", "ㅊ", 2, "", ""),
	ja("ko_r", "ㄱ", "ㄱ", 0, "ㄲ", "ㄲ", &["ㄲ"]),
	j("ko_f", "ㄹ", "ㄹ", 1, "", ""),
	j("ko_v", "ㅍ", "ㅍ", 2, "", ""),
	ja("ko_t", "ㅅ", "ㅅ", 0, "ㅆ", "ㅆ", &["ㅆ"]),
	j("ko_g", "ㅎ", "ㅎ", 1, "", ""),
	j("ko_b", "ㅠ", "ㅠ", 2, "", ""),
	j("ko_y", "ㅛ", "ㅛ", 0, "", ""),
	j("ko_h", "ㅗ", "ㅗ", 1, "", ""),
	j("ko_n", "ㅜ", "ㅜ", 2, "", ""),
	j("ko_u", "ㅕ", "ㅕ", 0, "", ""),
	j("ko_j", "ㅓ", "ㅓ", 1, "", ""),
	j("ko_m", "ㅡ", "ㅡ", 2, "", ""),
	j("ko_i", "ㅑ", "ㅑ", 0, "", ""),
	j("ko_k", "ㅏ", "ㅏ", 1, "", ""),
	ja("ko_o", "ㅐ", "ㅐ", 0, "ㅒ", "ㅒ", &["ㅒ"]),
	j("ko_l", "ㅣ", "ㅣ", 1, "", ""),
	ja("ko_p", "ㅔ", "ㅔ", 0, "ㅖ", "ㅖ", &["ㅖ"]),
];

pub static NUMBERS: &[Letter] = &[
	c("1", "1", 1),
	c("2", "2", 1),
	c("3", "3", 1),
	c("4", "4", 1),
	c("5", "5", 1),
	c("6", "6", 1),
	c("7", "7", 1),
	c("8", "8", 1),
	c("9", "9", 1),
	c("0", "0", 1),
	c("dot", ".", 1),
];

pub static SYMBOLS: &[Letter] = &[
	c("dot", ".", 1),
	c("comma", ",", 1),
	c("question", "?", 1),
	c("excl", "!", 1),
	c("apos", "'", 1),
	c("dquote", "\"", 1),
	c("dash", "-", 1),
	c("colon", ":", 1),
	c("semicolon", ";", 1),
	c("at", "@", 1),
	c("slash", "/", 1),
];

/// Fixed key of the bottom band.
#[derive(Debug, Clone, Copy)]
pub struct UtilityKey {
	pub id: &'static str,
	pub label: &'static str,
	pub kind: KeyKind,
	pub weight: f32,
	/// Text inserted by `KeyKind::Char` utilities.
	pub commit: &'static str,
	/// Vector icon drawn instead of the label ("" means text).
	pub icon: &'static str,
	/// Extra symbols offered by a long press.
	pub alts: &'static [&'static str],
}

/// Keys pinned to the ends of the suggestion strip.
pub static STRIP: &[UtilityKey] = &[
	UtilityKey {
		id: "shift",
		label: "⇧",
		kind: KeyKind::Shift,
		weight: 1.0,
		commit: "",
		icon: "shift",
		alts: &[],
	},
	UtilityKey {
		id: "backspace",
		label: "⌫",
		kind: KeyKind::Backspace,
		weight: 1.0,
		commit: "",
		icon: "backspace",
		alts: &[],
	},
];

/// Fixed keys of the bottom band.
pub static UTILITIES: &[UtilityKey] = &[
	UtilityKey {
		id: "page",
		label: "123",
		kind: KeyKind::Page,
		weight: 1.00,
		commit: "",
		icon: "",
		// Long-press shortcut: one digit without leaving the current page.
		alts: &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
	},
	UtilityKey {
		id: "comma",
		label: ",",
		kind: KeyKind::Char,
		weight: 0.90,
		commit: ",",
		icon: "",
		alts: &[],
	},
	UtilityKey {
		id: "space",
		label: "espace",
		kind: KeyKind::Space,
		weight: 5.20,
		commit: " ",
		icon: "",
		alts: &[],
	},
	UtilityKey {
		id: "dot",
		label: ".",
		kind: KeyKind::Char,
		weight: 0.90,
		commit: ".",
		icon: "",
		alts: &[
			"&", "%", "+", "\"", "-", ":", "'", "@", ";", "/", "(", ")", "#", "!", "?",
		],
	},
	UtilityKey {
		id: "enter",
		label: "⏎",
		kind: KeyKind::Enter,
		weight: 1.25,
		commit: "\n",
		icon: "",
		alts: &[],
	},
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
	Letters,
	Numbers,
	Symbols,
}

impl Page {
	pub fn next(self) -> Self {
		match self {
			Page::Letters => Page::Numbers,
			Page::Numbers => Page::Symbols,
			Page::Symbols => Page::Letters,
		}
	}

	pub fn id(self) -> &'static str {
		match self {
			Page::Letters => "letters",
			Page::Numbers => "numbers",
			Page::Symbols => "symbols",
		}
	}

	/// Label of the page key: what a tap will show next.
	pub fn switch_label(self) -> &'static str {
		match self {
			Page::Letters => "123",
			Page::Numbers => "#+=",
			Page::Symbols => "ABC",
		}
	}
}

#[derive(Debug)]
pub struct Layout {
	pub id: &'static str,
	pub label: &'static str,
	pub script: Script,
	/// Whether a long-press popup also offers the neighbouring keys' alts.
	pub merge_letter_alts: bool,
	pub letters: &'static [Letter],
	pub numbers: &'static [Letter],
	pub symbols: &'static [Letter],
	pub strip: &'static [UtilityKey],
	pub utilities: &'static [UtilityKey],
}

impl Layout {
	pub fn chars(&self, page: Page) -> &'static [Letter] {
		match page {
			Page::Letters => self.letters,
			Page::Numbers => self.numbers,
			Page::Symbols => self.symbols,
		}
	}
}

/// The default layout.
pub static AZERTY: Layout = Layout {
	id: "azerty",
	label: "AZERTY",
	script: Script::Latin,
	merge_letter_alts: true,
	letters: LETTERS,
	numbers: NUMBERS,
	symbols: SYMBOLS,
	strip: STRIP,
	utilities: UTILITIES,
};

/// QWERTY shares the number, symbol, strip and utility data with AZERTY.
pub static QWERTY: Layout = Layout {
	id: "qwerty",
	label: "QWERTY",
	script: Script::Latin,
	merge_letter_alts: true,
	letters: QWERTY_LETTERS,
	numbers: NUMBERS,
	symbols: SYMBOLS,
	strip: STRIP,
	utilities: UTILITIES,
};

/// Korean Sebeolsik Final: jamo keys, Hangul composition.
pub static SEBEOLSIK: Layout = Layout {
	id: "sebeolsik",
	label: "Sebeolsik Final",
	script: Script::Hangul,
	merge_letter_alts: false,
	letters: SEBEOLSIK_LETTERS,
	numbers: NUMBERS,
	symbols: SYMBOLS,
	strip: STRIP,
	utilities: UTILITIES,
};

/// Layouts offered when creating a keyboard profile.
pub static LAYOUTS: &[&Layout] = &[&AZERTY, &QWERTY, &SEBEOLSIK];

pub fn find(id: &str) -> Option<&'static Layout> {
	LAYOUTS.iter().copied().find(|layout| layout.id == id)
}

pub fn catalog_json() -> String {
	let mut out = String::from("[");
	for (index, layout) in LAYOUTS.iter().enumerate() {
		if index > 0 {
			out.push(',');
		}
		out.push_str("{\"id\":");
		json::string_into(&mut out, layout.id);
		out.push_str(",\"label\":");
		json::string_into(&mut out, layout.label);
		out.push('}');
	}
	out.push(']');
	out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftState {
	Off,
	Once,
	Locked,
}

impl ShiftState {
	pub fn cycle(self) -> Self {
		match self {
			Self::Off => Self::Once,
			Self::Once => Self::Locked,
			Self::Locked => Self::Off,
		}
	}

	pub fn code(self) -> u8 {
		match self {
			Self::Off => 0,
			Self::Once => 1,
			Self::Locked => 2,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_twenty_six_letters_are_on_the_line() {
		let mut seen: Vec<&str> = LETTERS.iter().map(|l| l.commit).collect();
		seen.sort_unstable();
		for ch in "abcdefghijklmnopqrstuvwxyz".chars() {
			let s = ch.to_string();
			assert!(seen.contains(&s.as_str()), "missing {ch}");
		}
		assert_eq!(LETTERS.len(), 26);
	}

	#[test]
	fn letters_are_ordered_by_azerty_rows() {
		let row = |index: u8| -> String {
			LETTERS
			. iter()
			. filter(|letter| letter.row == index)
			. map(|letter| letter.label)
			. collect()
		};
		assert_eq!(row(0), "AZERTYUIOP");
		assert_eq!(row(1), "QSDFGHJKLM");
		assert_eq!(row(2), "WXCVBN");
	}

	#[test]
	fn qwerty_letters_match_the_physical_rows() {
		let row = |index: u8| -> String {
			QWERTY_LETTERS
			. iter()
			. filter(|letter| letter.row == index)
			. map(|letter| letter.label)
			. collect()
		};
		assert_eq!(row(0), "QWERTYUIOP");
		assert_eq!(row(1), "ASDFGHJKL");
		assert_eq!(row(2), "ZXCVBNM");
		assert_eq!(QWERTY_LETTERS.len(), 26);
	}

	#[test]
	fn sebeolsik_letters_match_the_physical_rows() {
		let row = |index: u8| -> String {
			SEBEOLSIK_LETTERS
			. iter()
			. filter(|letter| letter.row == index)
			. map(|letter| letter.label)
			. collect()
		};
		assert_eq!(row(0), "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ");
		assert_eq!(row(1), "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ");
		assert_eq!(row(2), "ㅋㅌㅊㅍㅠㅜㅡ");
		assert_eq!(SEBEOLSIK_LETTERS.len(), 26);
		let double = (
			SEBEOLSIK_LETTERS
			. iter()
			. find(|letter| letter.id == "ko_r")
			. unwrap()
		);
		assert_eq!(double.commit, "ㄱ");
		assert_eq!(double.shift_commit, "ㄲ");
		assert_eq!(double.shift_label, "ㄲ");
		assert_eq!(double.alts, &["ㄲ"]);
		for (id, alt) in [
			("ko_q", "ㅃ"),
			("ko_w", "ㅉ"),
			("ko_e", "ㄸ"),
			("ko_r", "ㄲ"),
			("ko_t", "ㅆ"),
			("ko_o", "ㅒ"),
			("ko_p", "ㅖ"),
		] {
			let letter = SEBEOLSIK_LETTERS
			.iter()
			.find(|letter| letter.id == id)
			.unwrap();
			assert_eq!(letter.alts, &[alt], "{id}");
		}
		assert!(!SEBEOLSIK.merge_letter_alts);
		assert!(AZERTY.merge_letter_alts);
		assert!(QWERTY.merge_letter_alts);
	}

	#[test]
	fn layouts_can_be_looked_up() {
		let azerty = find("azerty").unwrap();
		assert_eq!(azerty.label, "AZERTY");
		assert_eq!(azerty.letters.len(), 26);
		let qwerty = find("qwerty").unwrap();
		assert_eq!(qwerty.label, "QWERTY");
		assert_eq!(qwerty.letters.len(), 26);
		let sebeolsik = find("sebeolsik").unwrap();
		assert_eq!(sebeolsik.label, "Sebeolsik Final");
		assert_eq!(sebeolsik.script, Script::Hangul);
		assert_eq!(sebeolsik.letters.len(), 26);
		assert!(find("missing").is_none());
		let mut ids: Vec<&str> = LAYOUTS.iter().map(|layout| layout.id).collect();
		ids.sort_unstable();
		ids.dedup();
		assert_eq!(ids.len(), LAYOUTS.len());
	}

	#[test]
	fn utilities_are_stable() {
		let ids: Vec<&str> = UTILITIES.iter().map(|u| u.id).collect();
		assert_eq!(ids, vec!["page", "comma", "space", "dot", "enter"]);
		let page = UTILITIES.iter().find(|u| u.id == "page").unwrap();
		assert_eq!(page.alts, &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]);
		let dot = UTILITIES.iter().find(|u| u.id == "dot").unwrap();
		assert_eq!(
			dot.alts,
			&[
				"&", "%", "+", "\"", "-", ":", "'", "@", ";", "/", "(", ")", "#", "!", "?",
			]
		);
		let strip: Vec<&str> = STRIP.iter().map(|u| u.id).collect();
		assert_eq!(strip, vec!["shift", "backspace"]);
	}
}
