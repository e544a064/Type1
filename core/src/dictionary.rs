//! Frequency-ranked French dictionary shipped with the core.
//!
//! The data is derived from Lexique 3.83 (https://www.lexique.org), distributed
//! under CC BY-SA 4.0. See `data/NOTICE.md` for attribution.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::lexicon::fold;

/// One orthographic form with its log frequency.
pub struct Word {
	pub text: &'static str,
	pub folded: String,
	/// `ln(frequency per million + 1)`, computed offline.
	pub log_freq: f32,
}

struct Index {
	words: Vec<Word>,
	by_text: HashMap<&'static str, usize>,
	by_fold: HashMap<String, usize>,
}

static INDEX: OnceLock<Index> = OnceLock::new();

fn index() -> &'static Index {
	INDEX.get_or_init(|| {
		let source: &'static str = include_str!("../data/fr.tsv");
		let mut words = Vec::with_capacity(20_000);
		for line in source.lines() {
			let Some((text, score)) = line.split_once('\t') else {
				continue;
			};
			if text.is_empty() {
				continue;
			}
			let log_freq = score.parse::<i32>().unwrap_or(0) as f32 / 100.0;
			words.push(Word {
				text,
				folded: fold(text),
				log_freq,
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
	})
}

pub fn words() -> &'static [Word] {
	&index().words
}

/// Exact lookup. `word` must already be lowercase.
pub fn exact(word: &str) -> Option<&'static Word> {
	index().by_text.get(word).map(|&i| &index().words[i])
}

/// Lookup by accent-free, lowercase form.
pub fn folded(form: &str) -> Option<&'static Word> {
	index().by_fold.get(form).map(|&i| &index().words[i])
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
	}
}
