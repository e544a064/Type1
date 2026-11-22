//! Declarative layout for the one-line keyboard.
//!
//! The middle band holds every letter on a single horizontal line. Letters are
//! grouped by AZERTY columns (`A Q W`, `Z S X`, ...), so the reading order of
//! the first letter of each group is the familiar `A Z E R T Y U I O P` row.
//!
//! The bottom band holds the utility keys (page, shift, space, comma, delete,
//! enter). The suggestion strip is rendered by the UI from the engine payload.

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
	/// Physical AZERTY row: 0 top, 1 home, 2 bottom. Used for the vertical
	/// offset of the glyph inside the single line.
	pub row: u8,
	pub alts: &'static [&'static str],
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
	}
}

/// The 26 letters in AZERTY column order: top, home, bottom, left to right.
/// Reading only the top-row letters gives `AZERTYUIOP`.
pub static LETTERS: &[Letter] = &[
	l("a", "A", 0, &["à", "â", "ä"]),
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
	l("u", "U", 0, &["ù", "û", "ü"]),
	l("j", "J", 1, &[]),
	l("i", "I", 0, &["î", "ï"]),
	l("k", "K", 1, &[]),
	l("o", "O", 0, &["ô", "ö", "œ"]),
	l("l", "L", 1, &[]),
	l("p", "P", 0, &[]),
	l("m", "M", 1, &[]),
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
	},
	UtilityKey {
		id: "backspace",
		label: "⌫",
		kind: KeyKind::Backspace,
		weight: 1.0,
		commit: "",
		icon: "backspace",
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
	},
	UtilityKey {
		id: "space",
		label: "espace",
		kind: KeyKind::Space,
		weight: 5.20,
		commit: " ",
		icon: "",
	},
	UtilityKey {
		id: "comma",
		label: ",",
		kind: KeyKind::Char,
		weight: 0.90,
		commit: ",",
		icon: "",
	},
	UtilityKey {
		id: "enter",
		label: "⏎",
		kind: KeyKind::Enter,
		weight: 1.25,
		commit: "\n",
		icon: "",
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

	pub fn chars(self) -> &'static [Letter] {
		match self {
			Page::Letters => LETTERS,
			Page::Numbers => NUMBERS,
			Page::Symbols => SYMBOLS,
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
	fn utilities_are_stable() {
		let ids: Vec<&str> = UTILITIES.iter().map(|u| u.id).collect();
		assert_eq!(ids, vec!["page", "space", "comma", "enter"]);
		let strip: Vec<&str> = STRIP.iter().map(|u| u.id).collect();
		assert_eq!(strip, vec!["shift", "backspace"]);
	}
}
