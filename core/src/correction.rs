//! Layout-aware spelling correction.
//!
//! Candidates are generated from the shipped dictionary and scored with a
//! noisy-channel model: `P(typed | intended)` comes from the distance between
//! the physical keys, `P(intended)` from the word frequency.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::dictionary;
use crate::layout;
use crate::lexicon::fold;

/// Minimum score a candidate needs before it may replace the typed word.
pub const MIN_SCORE: f32 = -0.5;
/// Weight of the word-frequency prior relative to the touch costs.
const FREQ_WEIGHT: f32 = 0.1;
const MAX_COST: f32 = 1.8;
const MAX_RESULTS: usize = 6;
/// Longest token the weighted distance will consider.
const MAX_CHARS: usize = 31;
/// `a`–`z` use 0–25, everything else (apostrophes, stray letters) shares 26.
const OTHER: usize = 26;

#[derive(Clone, Debug)]
pub struct Candidate {
	pub text: String,
	pub cost: f32,
	pub score: f32,
}

fn positions() -> &'static HashMap<char, (f32, f32)> {
	static POSITIONS: OnceLock<HashMap<char, (f32, f32)>> = OnceLock::new();
	POSITIONS.get_or_init(|| {
		let mut map = HashMap::new();
		for (index, letter) in layout::LETTERS.iter().enumerate() {
			if let Some(c) = letter.id.chars().next() {
				map.insert(c, (index as f32, letter.row as f32 * 0.55));
			}
		}
		map
	})
}

fn char_index(byte: u8) -> usize {
	match byte {
		b'a'..=b'z' => (byte - b'a') as usize,
		_ => OTHER,
	}
}

/// Pairwise substitution costs, precomputed once from the key positions.
fn costs() -> &'static [[f32; OTHER + 1]; OTHER + 1] {
	static COSTS: OnceLock<[[f32; OTHER + 1]; OTHER + 1]> = OnceLock::new();
	COSTS.get_or_init(|| {
		let mut table = [[1.5f32; OTHER + 1]; OTHER + 1];
		for (a, row) in table.iter_mut().enumerate() {
			row[a] = 0.0;
		}
		for (a, row) in table.iter_mut().enumerate().take(26) {
			for (b, slot) in row.iter_mut().enumerate().take(26) {
				let ca = (b'a' + a as u8) as char;
				let cb = (b'a' + b as u8) as char;
				if let (Some(&(x1, y1)), Some(&(x2, y2))) =
					(positions().get(&ca), positions().get(&cb))
				{
					let dx = x1 - x2;
					let dy = y1 - y2;
					*slot = (dx * dx + dy * dy).sqrt().min(2.6);
				}
			}
		}
		table
	})
}

/// Weighted Damerau-Levenshtein distance, abandoned as soon as every path is
/// more expensive than `max`. Operates on folded (ASCII) bytes.
fn distance(a: &[u8], b: &[u8], max: f32) -> Option<f32> {
	let n = a.len();
	let m = b.len();
	if n.abs_diff(m) > 2 || n > MAX_CHARS || m > MAX_CHARS {
		return None;
	}

	let costs = costs();
	let mut prev2 = [f32::INFINITY; MAX_CHARS + 1];
	let mut prev = [0.0f32; MAX_CHARS + 1];
	let mut cur = [0.0f32; MAX_CHARS + 1];
	for (j, slot) in prev.iter_mut().enumerate().take(m + 1) {
		*slot = j as f32;
	}

	for i in 1..=n {
		cur[0] = i as f32;
		let mut row_min = cur[0];
		let ai = char_index(a[i - 1]);
		for j in 1..=m {
			let mut best = (prev[j - 1] + costs[ai][char_index(b[j - 1])])
			. min(prev[j] + 1.0)
			. min(cur[j - 1] + 1.0);
			if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
				let transposed = prev2[j - 2] + 0.6;
				if transposed < best {
					best = transposed;
				}
			}
			cur[j] = best;
			if best < row_min {
				row_min = best;
			}
		}
		if row_min > max {
			return None;
		}
		prev2 = prev;
		prev = cur;
	}
	if prev[m] <= max {
		Some(prev[m])
	} else {
		None
	}
}

const CLITICS: &[(&str, f32)] = &[
	("jusqu", 0.4),
	("lorsqu", 0.4),
	("puisqu", 0.4),
	("presqu", 0.4),
	("qu", 0.7),
	("j", 0.6),
	("t", 0.6),
	("l", 0.6),
	("d", 0.6),
	("n", 0.6),
	("s", 0.6),
	("m", 0.6),
	("c", 0.6),
];

fn starts_with_vowel(s: &str) -> bool {
	matches!(
		s.chars().next(),
		Some(
			'a' | 'e'
				| 'i' | 'o' | 'u' | 'y'
				| 'à' | 'â' | 'ä' | 'é' | 'è' | 'ê' | 'ë' | 'î' | 'ï' | 'ô' | 'ö' | 'ù'
				| 'û' | 'ü' | 'œ' | 'æ'
		)
	)
}

fn starts_with_vowel_or_h(s: &str) -> bool {
	starts_with_vowel(s) || s.starts_with('h')
}

fn insert(found: &mut HashMap<String, Candidate>, candidate: Candidate) {
	match found.get(&candidate.text) {
		Some(existing) if existing.score >= candidate.score => {}
		_ => {
			found.insert(candidate.text.clone(), candidate);
		}
	}
}

/// Candidate corrections for a typed token, best first. Empty when the token is
/// already a known word, too short, or contains an apostrophe.
pub fn candidates(typed: &str) -> Vec<Candidate> {
	let lower = typed.to_lowercase();
	if lower.chars().count() < 2 || lower.contains('\'') {
		return Vec::new();
	}
	if dictionary::exact(&lower).is_some() {
		return Vec::new();
	}

	let folded_typed = fold(&lower);
	let typed_bytes = folded_typed.as_bytes();
	let mut found: HashMap<String, Candidate> = HashMap::new();

	// Missing accents only (`ecole` -> `école`).
	if let Some(word) = dictionary::folded(&folded_typed) {
		let cost = 0.25;
		insert(
			&mut found,
			Candidate {
				text: word.text.to_string(),
				cost,
				score: word.log_freq * FREQ_WEIGHT - cost,
			},
		);
	}

	// Elision: `jai` -> `j'ai`, `taime` -> `t'aime`.
	for &(clitic, penalty) in CLITICS {
		if !lower.starts_with(clitic) {
			continue;
		}
		let rest = &lower[clitic.len()..];
		// Long clitics (`jusqu`, `lorsqu`) can precede a single-letter word.
		let shortest = if clitic.len() >= 3 { 1 } else { 2 };
		if rest.chars().count() < shortest {
			continue;
		}
		let allowed = if clitic == "qu" {
			starts_with_vowel(rest)
		} else {
			starts_with_vowel_or_h(rest)
		};
		if !allowed {
			continue;
		}
		if let Some(word) = dictionary::folded(&fold(rest)) {
			let mut remainder = word.text;
			let mut log_freq = word.log_freq;
			// `jusqua` -> `jusqu'à`: the remainder folds to the verb `a`.
			if clitic == "jusqu" && remainder == "a" {
				if let Some(grave) = dictionary::exact("à") {
					remainder = grave.text;
					log_freq = grave.log_freq;
				}
			}
			insert(
				&mut found,
				Candidate {
					text: format!("{clitic}'{remainder}"),
					cost: 0.15,
					score: log_freq * FREQ_WEIGHT - penalty,
				},
			);
		}
	}

	// Typographical and neighbouring-key errors.
	for word in dictionary::words() {
		let word_bytes = word.folded.as_bytes();
		if word_bytes.len() < 2 {
			continue;
		}
		if let Some(cost) = distance(typed_bytes, word_bytes, MAX_COST) {
			if cost == 0.0 {
				continue;
			}
			insert(
				&mut found,
				Candidate {
					text: word.text.to_string(),
					cost,
					score: word.log_freq * FREQ_WEIGHT - cost,
				},
			);
		}
	}

	let mut list: Vec<Candidate> = found.into_values().collect();
	list.sort_by(|a, b| {
		b.score
		. partial_cmp(&a.score)
		. unwrap_or(std::cmp::Ordering::Equal)
		. then_with(|| a.text.cmp(&b.text))
	});
	list.truncate(MAX_RESULTS);
	list
}

#[cfg(test)]
mod tests {
	use super::*;

	fn best(typed: &str) -> Option<String> {
		candidates(typed).into_iter().next().map(|c| c.text)
	}

	#[test]
	fn neighbour_key_typo() {
		assert_eq!(best("dkre").as_deref(), Some("dire"));
	}

	#[test]
	fn transposition() {
		assert_eq!(best("tset").as_deref(), Some("test"));
	}

	#[test]
	fn missing_accent() {
		assert_eq!(best("ecole").as_deref(), Some("école"));
	}

	#[test]
	fn elision() {
		assert_eq!(best("jai").as_deref(), Some("j'ai"));
		assert_eq!(best("taime").as_deref(), Some("t'aime"));
		assert_eq!(best("lhomme").as_deref(), Some("l'homme"));
		assert_eq!(best("daccord").as_deref(), Some("d'accord"));
		assert_eq!(best("quil").as_deref(), Some("qu'il"));
		assert_eq!(best("jusqua").as_deref(), Some("jusqu'à"));
	}

	#[test]
	fn known_words_have_no_candidates() {
		assert!(candidates("test").is_empty());
		assert!(candidates("dire").is_empty());
	}

	#[test]
	fn low_confidence_candidates_stay_below_the_margin() {
		let list = candidates("mmm");
		let best = list.first().expect("a weak candidate");
		assert!(best.score < MIN_SCORE, "score {}", best.score);
	}

	#[test]
	fn garbage_has_no_candidates() {
		assert!(candidates("xqzv").is_empty());
	}
}
