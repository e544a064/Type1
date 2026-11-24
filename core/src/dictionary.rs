//! Frequency-ranked dictionary shipped with the core.
//!
//! Each language owns its own [`Dictionary`]. The French data is derived from
//! Lexique 3.83 (https://www.lexique.org) and the English data from wordfreq
//! (https://github.com/rspeer/wordfreq), both distributed under CC BY-SA 4.0.
//! See `data/NOTICE.md` for attribution.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::lexicon::fold;

/// One orthographic form with its log frequency.
#[derive(Debug)]
pub struct Word {
	pub text: &'static str,
	pub folded: String,
	/// `ln(frequency per million + 1)`, computed offline.
	pub log_freq: f32,
	/// Part-of-speech flags from the dictionary builder.
	pub flags: u8,
}

pub const VERB: u8 = 1;
pub const NOUN: u8 = 2;
pub const ADJ: u8 = 4;
pub const ADV: u8 = 8;

#[derive(Debug)]
struct Index {
	words: Vec<Word>,
	by_text: HashMap<&'static str, usize>,
	by_fold: HashMap<String, usize>,
}

impl Index {
	fn load(source: &'static str) -> Self {
		let mut words = Vec::with_capacity(20_000);
		for line in source.lines() {
			let mut parts = line.split('\t');
			let Some(text) = parts.next() else {
				continue;
			};
			if text.is_empty() {
				continue;
			}
			let log_freq = parts
			. next()
			. and_then(|score| score.parse::<i32>().ok())
			. unwrap_or(0) as f32
				/ 100.0;
			let flags = parts
			. next()
			. and_then(|flags| flags.parse::<u8>().ok())
			. unwrap_or(0);
			words.push(Word {
				text,
				folded: fold(text),
				log_freq,
				flags,
			});
		}

		let mut by_text = HashMap::with_capacity(words.len());
		let mut by_fold: HashMap<String, usize> = HashMap::with_capacity(words.len());
		for (i, word) in words.iter().enumerate() {
			by_text.insert(word.text, i);
			match by_fold.get(&word.folded) {
				Some(&existing) if words[existing].log_freq >= word.log_freq => {}
				_ => {
					by_fold.insert(word.folded.clone(), i);
				}
			}
		}

		Index {
			words,
			by_text,
			by_fold,
		}
	}
}

/// A lazily loaded word list for one language.
#[derive(Debug)]
pub struct Dictionary {
	source: &'static str,
	index: OnceLock<Index>,
}

impl Dictionary {
	pub const fn new(source: &'static str) -> Self {
		Dictionary {
			source,
			index: OnceLock::new(),
		}
	}

	fn index(&self) -> &Index {
		self.index.get_or_init(|| Index::load(self.source))
	}

	pub fn words(&self) -> &[Word] {
		&self.index().words
	}

	/// Exact lookup. `word` must already be lowercase.
	pub fn exact(&self, word: &str) -> Option<&Word> {
		self.index().by_text.get(word).map(|&i| &self.index().words[i])
	}

	/// Lookup by accent-free, lowercase form.
	pub fn folded(&self, form: &str) -> Option<&Word> {
		self.index().by_fold.get(form).map(|&i| &self.index().words[i])
	}
}

/// French dictionary (Lexique 3.83).
pub static FRENCH: Dictionary = Dictionary::new(include_str!("../data/fr.tsv"));

/// British English dictionary (wordfreq).
pub static ENGLISH: Dictionary = Dictionary::new(include_str!("../data/en.tsv"));

/// Compatibility wrappers for the default French dictionary.
pub fn words() -> &'static [Word] {
	FRENCH.words()
}

pub fn exact(word: &str) -> Option<&'static Word> {
	FRENCH.exact(word)
}

pub fn folded(form: &str) -> Option<&'static Word> {
	FRENCH.folded(form)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn loads_the_shipped_dictionary() {
		assert!(words().len() >= 10_000);
		assert!(exact("test").is_some());
		assert!(exact("dire").is_some());
		assert!(exact("ecole").is_none());
		assert_eq!(folded("ecole").map(|w| w.text), Some("école"));
		assert_ne!(exact("aime").unwrap().flags & VERB, 0);
		assert_ne!(exact("action").unwrap().flags & NOUN, 0);
	}

	#[test]
	fn loads_the_english_dictionary() {
		let english = &ENGLISH;
		assert!(english.words().len() >= 15_000);
		assert!(english.exact("keyboard").is_some());
		assert!(english.exact("colour").is_some());
		assert!(english.exact("color").is_none());
		assert!(english.exact("centre").is_some());
		assert!(english.exact("center").is_none());
		assert!(english.exact("language").is_some());
		assert_eq!(english.folded("i").map(|word| word.text), Some("I"));
	}
}
