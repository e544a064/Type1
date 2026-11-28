//! Hangul jamo composition and decomposition.
//!
//! The keyboard commits compatibility jamo (`ㄱ`, `ㅏ`, ...). Composition
//! follows the Unicode Hangul syllable algorithm: a lead consonant and a vowel
//! make one precomposed syllable, and a trailing consonant or a second vowel
//! can extend or split it. Decomposition reverses the precomposed syllable so
//! suggestions can work on the original keystrokes.

/// First precomposed Hangul syllable (가).
const SYLLABLE_BASE: u32 = 0xAC00;
/// Number of lead consonants (choseong).
const LEAD_COUNT: usize = 19;
/// Number of vowels (jungseong).
const VOWEL_COUNT: usize = 21;
/// Number of trailing consonants (jongseong), including none.
const TAIL_COUNT: usize = 28;

const LEADS: [char; LEAD_COUNT] = [
	'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ',
	'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

const VOWELS: [char; VOWEL_COUNT] = [
	'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ',
	'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// Index 0 means "no trailing consonant".
const TAILS: [char; TAIL_COUNT] = [
	'\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ',
	'ㅀ', 'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/// Compound vowels, written as first + second = compound.
const COMPOUND_VOWELS: &[(char, char, char)] = &[
	('ㅏ', 'ㅣ', 'ㅐ'),
	('ㅑ', 'ㅣ', 'ㅒ'),
	('ㅓ', 'ㅣ', 'ㅔ'),
	('ㅕ', 'ㅣ', 'ㅖ'),
	('ㅗ', 'ㅏ', 'ㅘ'),
	('ㅗ', 'ㅐ', 'ㅙ'),
	('ㅗ', 'ㅣ', 'ㅚ'),
	('ㅜ', 'ㅓ', 'ㅝ'),
	('ㅜ', 'ㅔ', 'ㅞ'),
	('ㅜ', 'ㅣ', 'ㅟ'),
	('ㅡ', 'ㅣ', 'ㅢ'),
];

/// Compound trailing consonants, written as first + second = compound.
const COMPOUND_TAILS: &[(char, char, char)] = &[
	('ㄱ', 'ㅅ', 'ㄳ'),
	('ㄴ', 'ㅈ', 'ㄵ'),
	('ㄴ', 'ㅎ', 'ㄶ'),
	('ㄹ', 'ㄱ', 'ㄺ'),
	('ㄹ', 'ㅁ', 'ㄻ'),
	('ㄹ', 'ㅂ', 'ㄼ'),
	('ㄹ', 'ㅅ', 'ㄽ'),
	('ㄹ', 'ㅌ', 'ㄾ'),
	('ㄹ', 'ㅍ', 'ㄿ'),
	('ㄹ', 'ㅎ', 'ㅀ'),
	('ㅂ', 'ㅅ', 'ㅄ'),
];

/// Double lead consonants, written as the same consonant twice.
const DOUBLE_LEADS: &[(char, char)] = &[
	('ㄱ', 'ㄲ'),
	('ㄷ', 'ㄸ'),
	('ㅂ', 'ㅃ'),
	('ㅅ', 'ㅆ'),
	('ㅈ', 'ㅉ'),
];

pub fn is_syllable(c: char) -> bool {
	(SYLLABLE_BASE..=0xD7A3).contains(&(c as u32))
}

/// True for compatibility jamo, the forms committed by the keyboard.
pub fn is_jamo(c: char) -> bool {
	matches!(c as u32, 0x3131..=0x318E)
}

fn is_vowel(c: char) -> bool {
	vowel_index(c).is_some()
}

fn is_consonant(c: char) -> bool {
	lead_index(c).is_some() || tail_index(c).is_some()
}

fn lead_index(c: char) -> Option<usize> {
	LEADS.iter().position(|&lead| lead == c)
}

fn vowel_index(c: char) -> Option<usize> {
	VOWELS.iter().position(|&vowel| vowel == c)
}

fn tail_index(c: char) -> Option<usize> {
	TAILS.iter().position(|&tail| tail == c && tail != '\0')
}

fn compound_vowel(first: char, second: char) -> Option<char> {
	COMPOUND_VOWELS
	. iter()
	. find(|&&(a, b, _)| a == first && b == second)
	. map(|&(_, _, compound)| compound)
}

fn compound_tail(first: char, second: char) -> Option<char> {
	COMPOUND_TAILS
	. iter()
	. find(|&&(a, b, _)| a == first && b == second)
	. map(|&(_, _, compound)| compound)
}

fn compound_tail_parts(tail: char) -> (Option<char>, char) {
	COMPOUND_TAILS
	. iter()
	. find(|&&(_, _, compound)| compound == tail)
	. map(|&(first, second, _)| (Some(first), second))
	. unwrap_or((None, tail))
}

fn double_lead(first: char, second: char) -> Option<char> {
	DOUBLE_LEADS
	. iter()
	. find(|&&(base, _)| base == first && base == second)
	. map(|&(_, double)| double)
}

fn split_vowel(vowel: char, out: &mut Vec<char>) {
	// These four vowels have their own Sebeolsik key (or shift key), so they
	// behave like a single keystroke. The other compound vowels are built from
	// two vowel keys.
	if matches!(vowel, 'ㅐ' | 'ㅔ' | 'ㅒ' | 'ㅖ') {
		out.push(vowel);
		return;
	}
	let parts = COMPOUND_VOWELS
	. iter()
	. find(|&&(_, _, compound)| compound == vowel);
	match parts {
		Some(&(first, second, _)) => {
			out.push(first);
			split_vowel(second, out);
		}
		None => out.push(vowel),
	}
}

fn split_tail(tail: char, out: &mut Vec<char>) {
	match COMPOUND_TAILS
	. iter()
	. find(|&&(_, _, compound)| compound == tail)
	{
		Some(&(first, second, _)) => {
			out.push(first);
			out.push(second);
		}
		None => out.push(tail),
	}
}

/// Compose a lead, vowel and optional tail into a precomposed syllable.
pub fn compose_syllable(lead: char, vowel: char, tail: Option<char>) -> Option<char> {
	let lead = lead_index(lead)?;
	let vowel = vowel_index(vowel)?;
	let tail = match tail {
		Some(tail) => tail_index(tail)?,
		None => 0,
	};
	let index = (lead * VOWEL_COUNT + vowel) * TAIL_COUNT + tail;
	char::from_u32(SYLLABLE_BASE + index as u32)
}

/// Split a precomposed syllable into its lead, vowel and optional tail.
pub fn decompose_syllable(c: char) -> Option<(char, char, Option<char>)> {
	if !is_syllable(c) {
		return None;
	}
	let index = (c as u32 - SYLLABLE_BASE) as usize;
	let lead = index / (VOWEL_COUNT * TAIL_COUNT);
	let vowel = (index / TAIL_COUNT) % VOWEL_COUNT;
	let tail = index % TAIL_COUNT;
	Some((
		LEADS[lead],
		VOWELS[vowel],
		(tail != 0).then_some(TAILS[tail]),
	))
}

/// Recover the jamo sequence behind visible text.
///
/// Compound vowels and compound tails are split into the keys that build them,
/// so the result can be composed again or searched for neighbouring-key
/// mistakes. A double lead consonant stays one jamo because Sebeolsik types it
/// with shift. Returns an empty vector when the text holds anything other than
/// Hangul, because correction is then not meaningful.
pub fn decompose_keystrokes(text: &str) -> Vec<char> {
	let mut out = Vec::new();
	for c in text.chars() {
		if let Some((lead, vowel, tail)) = decompose_syllable(c) {
			out.push(lead);
			split_vowel(vowel, &mut out);
			if let Some(tail) = tail {
				split_tail(tail, &mut out);
			}
		} else if is_jamo(c) {
			out.push(c);
		} else {
			return Vec::new();
		}
	}
	out
}

/// Compose a jamo sequence, editing the previous syllable when the new key
/// extends it. Incomplete syllables are emitted as the jamo that were typed.
pub fn compose_jamo(jamo: &[char]) -> String {
	let mut out = String::new();
	let mut state = State::default();
	for &c in jamo {
		state.push(c, &mut out);
	}
	state.flush(&mut out);
	out
}

#[derive(Default)]
struct State {
	lead: Option<char>,
	vowel: Option<char>,
	tail: Option<char>,
}

impl State {
	fn push(&mut self, c: char, out: &mut String) {
		if is_vowel(c) {
			self.push_vowel(c, out);
		} else if is_consonant(c) {
			self.push_consonant(c, out);
		} else {
			self.flush(out);
			out.push(c);
		}
	}

	fn push_vowel(&mut self, vowel: char, out: &mut String) {
		match (self.vowel, self.tail) {
			(None, _) => {
				if self.lead.is_some() {
					self.vowel = Some(vowel);
				} else {
					self.flush(out);
					self.vowel = Some(vowel);
				}
			}
			(Some(current), None) => {
				if let Some(compound) = compound_vowel(current, vowel) {
					self.vowel = Some(compound);
				} else {
					self.flush(out);
					self.vowel = Some(vowel);
				}
			}
			(Some(_), Some(tail)) => {
				let (previous_tail, next_lead) = compound_tail_parts(tail);
				self.tail = previous_tail;
				self.flush(out);
				self.lead = Some(next_lead);
				self.vowel = Some(vowel);
			}
		}
	}

	fn push_consonant(&mut self, consonant: char, out: &mut String) {
		if self.vowel.is_none() {
			if let Some(lead) = self.lead {
				if let Some(double) = double_lead(lead, consonant) {
					self.lead = Some(double);
				} else {
					self.flush(out);
					self.lead = Some(consonant);
				}
			} else {
				self.lead = Some(consonant);
			}
			return;
		}
		if let Some(tail) = self.tail {
			if let Some(compound) = compound_tail(tail, consonant) {
				self.tail = Some(compound);
			} else {
				self.flush(out);
				self.lead = Some(consonant);
			}
			return;
		}
		if tail_index(consonant).is_some() {
			self.tail = Some(consonant);
		} else {
			self.flush(out);
			self.lead = Some(consonant);
		}
	}

	fn flush(&mut self, out: &mut String) {
		let lead = self.lead.take();
		let vowel = self.vowel.take();
		let tail = self.tail.take();
		match (lead, vowel) {
			(Some(lead), Some(vowel)) => {
				if let Some(syllable) = compose_syllable(lead, vowel, tail) {
					out.push(syllable);
				} else {
					out.push(lead);
					out.push(vowel);
					if let Some(tail) = tail {
						out.push(tail);
					}
				}
			}
			(Some(lead), None) => out.push(lead),
			(None, Some(vowel)) => out.push(vowel),
			(None, None) => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn compose(jamo: &str) -> String {
		compose_jamo(&jamo.chars().collect::<Vec<_>>())
	}

	#[test]
	fn composes_simple_syllables() {
		assert_eq!(compose("ㄱㅏ"), "가");
		assert_eq!(compose("ㄱㅏㄴ"), "간");
		assert_eq!(compose("ㄱㅏㄴㅏ"), "가나");
		assert_eq!(compose("ㅎㅏㄴㄱㅡㄹ"), "한글");
	}

	#[test]
	fn composes_compound_vowels_and_tails() {
		assert_eq!(compose("ㅇㅗㅏ"), "와");
		assert_eq!(compose("ㄱㅏㅂㅅ"), "값");
		assert_eq!(compose("ㄱㅏㄹㄱㅏ"), "갈가");
		assert_eq!(compose("ㄱㅏㄴㅈㅏ"), "간자");
		assert_eq!(compose("ㅇㅏㄴㅈ"), "앉");
		assert_eq!(compose("ㄷㅏㄹㄱ"), "닭");
	}

	#[test]
	fn composes_double_consonants() {
		assert_eq!(compose("ㄱㄱㅏ"), "까");
		assert_eq!(compose("ㄱㄱㅏㄲ"), "깎");
		assert_eq!(compose("ㄲㅏㄲ"), "깎");
	}

	#[test]
	fn decomposition_round_trips() {
		for text in ["가", "간", "한글", "와", "값", "까", "깎", "갈가", "해", "왜"] {
			let jamo = decompose_keystrokes(text);
			assert_eq!(compose_jamo(&jamo), text, "{text}");
		}
	}

	#[test]
	fn decomposes_compound_forms_into_keys() {
		assert_eq!(decompose_keystrokes("와"), "ㅇㅗㅏ".chars().collect::<Vec<_>>());
		assert_eq!(decompose_keystrokes("값"), "ㄱㅏㅂㅅ".chars().collect::<Vec<_>>());
		assert_eq!(decompose_keystrokes("까"), "ㄲㅏ".chars().collect::<Vec<_>>());
		assert_eq!(decompose_keystrokes("해"), "ㅎㅐ".chars().collect::<Vec<_>>());
		assert_eq!(decompose_keystrokes("왜"), "ㅇㅗㅐ".chars().collect::<Vec<_>>());
	}

	#[test]
	fn every_syllable_round_trips() {
		for code in 0xAC00..=0xD7A3 {
			let syllable = char::from_u32(code).unwrap();
			let text = syllable.to_string();
			let jamo = decompose_keystrokes(&text);
			assert_eq!(compose_jamo(&jamo), text, "{syllable}");
		}
	}

	#[test]
	fn rejects_text_that_is_not_hangul() {
		assert!(decompose_keystrokes("abc").is_empty());
		assert!(decompose_keystrokes("한a").is_empty());
	}
}
