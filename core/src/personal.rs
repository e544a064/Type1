//! Personal words supplied by the host, usually Android's user dictionary.
//!
//! The embedded language dictionaries are immutable. Personal words are a
//! small runtime overlay: exact matches are protected as known words and close
//! matches are offered as suggestions without being auto-applied.

use std::collections::HashMap;

use crate::language::Language;
use crate::lexicon::fold;

/// Base score for a personal word. It is above the minimum apply threshold but
/// below the most common language words, so personal entries help ranking
/// without taking over ordinary typing.
const PERSONAL_BASE_SCORE: f32 = 8.0;
const PERSONAL_FREQUENCY_SCORE: f32 = 0.01;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalWord {
	text: String,
	locale: Option<String>,
	frequency: u8,
	folded: String,
}

impl PersonalWord {
	pub fn new(text: impl Into<String>, locale: Option<String>, frequency: u8) -> Self {
		let text = text.into();
		let locale = locale.and_then(|value| {
			let value = value.trim();
			if value.is_empty() {
				None
			} else {
				Some(value.to_string())
			}
		});
		let folded = fold(&text);
		PersonalWord {
			text,
			locale,
			frequency,
			folded,
		}
	}

	pub fn text(&self) -> &str {
		&self.text
	}

	pub fn folded(&self) -> &str {
		&self.folded
	}

	pub fn frequency(&self) -> u8 {
		self.frequency
	}

	pub fn locale(&self) -> Option<&str> {
		self.locale.as_deref()
	}

	pub fn score(&self) -> f32 {
		PERSONAL_BASE_SCORE + self.frequency as f32 * PERSONAL_FREQUENCY_SCORE
	}

	/// Whether this entry applies to at least one active language. Android
	/// locales look like `en_US` or `fr-FR`; a missing locale applies to all.
	pub fn matches(&self, languages: &[&Language]) -> bool {
		let Some(locale) = &self.locale else {
			return true;
		};
		let language = locale
		. split(['_', '-'])
		. next()
		. unwrap_or(locale);
		languages
		. iter()
		. any(|active| active.id.eq_ignore_ascii_case(language))
	}
}

#[derive(Clone, Debug, Default)]
pub struct PersonalLexicon {
	words: Vec<PersonalWord>,
	by_fold: HashMap<String, Vec<usize>>,
	by_len: Vec<Vec<usize>>,
}

impl PersonalLexicon {
	pub fn new(words: Vec<PersonalWord>) -> Self {
		let mut by_fold: HashMap<String, Vec<usize>> = HashMap::new();
		let mut by_len: Vec<Vec<usize>> = Vec::new();
		for (index, word) in words.iter().enumerate() {
			by_fold
			. entry(word.folded.clone())
			. or_default()
			. push(index);
			let length = word.folded.len();
			if by_len.len() <= length {
				by_len.resize_with(length + 1, Vec::new);
			}
			by_len[length].push(index);
		}
		PersonalLexicon {
			words,
			by_fold,
			by_len,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.words.is_empty()
	}

	pub fn len(&self) -> usize {
		self.words.len()
	}

	pub fn words(&self) -> &[PersonalWord] {
		&self.words
	}

	/// Best exact match by folded form, limited to the active languages.
	pub fn exact<'a>(&'a self, folded: &str, languages: &[&Language]) -> Option<&'a PersonalWord> {
		self
		. by_fold
		. get(folded)?
		. iter()
		. map(|&index| &self.words[index])
		.filter(|word| word.matches(languages))
		.max_by(|a, b| {
			a.frequency
			.cmp(&b.frequency)
			.then_with(|| b.text.cmp(&a.text))
		})
	}

	/// Same folded length, used for typo candidates.
	pub fn words_of_len(&self, length: usize) -> impl Iterator<Item = &PersonalWord> + '_ {
		self
		. by_len
		. get(length)
		. into_iter()
		.flatten()
		.map(|&index| &self.words[index])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::language;

	#[test]
	fn exact_matches_fold_and_language() {
		let lexicon = PersonalLexicon::new(vec![
			PersonalWord::new("Zoé", Some("fr_FR".into()), 250),
			PersonalWord::new("zoe", Some("en_US".into()), 10),
			PersonalWord::new("Solo", None, 10),
		]);
		assert_eq!(
			lexicon
			. exact("zoe", &[&language::FRENCH])
			.map(PersonalWord::text),
			Some("Zoé")
		);
		assert_eq!(
			lexicon
			. exact("zoe", &[&language::ENGLISH])
			.map(PersonalWord::text),
			Some("zoe")
		);
		assert_eq!(
			lexicon
			. exact("solo", &[&language::FRENCH])
			.map(PersonalWord::text),
			Some("Solo")
		);
	}

	#[test]
	fn entries_without_a_locale_match_every_profile() {
		let lexicon = PersonalLexicon::new(vec![PersonalWord::new("Zorblat", None, 0)]);
		assert!(lexicon.exact("zorblat", &[&language::FRENCH]).is_some());
		assert!(lexicon.exact("zorblat", &[&language::ENGLISH]).is_some());
	}
}
