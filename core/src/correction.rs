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
pub const MIN_SCORE: f32 = 0.5;
const MAX_COST: f32 = 1.55;
/// Frequency dominates the ranking: among words whose letters are all one key
/// away, the common word is the likely one.
const AGGRESSIVE_WEIGHT: f32 = 1.5;
/// Changing the word length is less likely than pressing a neighbouring key.
const LENGTH_PENALTY: f32 = 2.0;
/// A known word is only overridden when a near-key candidate is this much more
/// likely (`pa` -> `la`, but `la` stays `la`).
const KNOWN_WORD_MARGIN: f32 = 2.0;
/// The top candidate must lead the runner-up by this much.
pub const AGGRESSIVE_MARGIN: f32 = 0.25;
/// A substituted letter must be this close to count as a neighbouring key.
const NEIGHBOUR_COST: f32 = 1.5;
const TRANSPOSE_COST: f32 = 0.6;
/// Cost of inserting the elision apostrophe.
const APOSTROPHE_COST: f32 = 0.15;
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

/// The candidates to show for a typed token, and whether the best one is
/// trustworthy enough to replace the token automatically.
#[derive(Clone, Debug)]
pub struct Suggestion {
	pub candidates: Vec<Candidate>,
	pub apply: bool,
}

impl Suggestion {
	pub fn empty() -> Self {
		Suggestion {
			candidates: Vec::new(),
			apply: false,
		}
	}
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

fn is_infinitive(text: &str) -> bool {
	text.ends_with("er") || text.ends_with("ir") || text.ends_with("re") || text.ends_with("oir")
}

/// Whether `clitic` can elide before `word`. Pronouns and `ce` need a verb;
/// `de` takes nouns, adverbs or infinitives; `l'` is broad.
fn elision_allowed(clitic: &str, word: &dictionary::Word) -> bool {
	let text = word.text;
	let vowel = if clitic == "qu" {
		starts_with_vowel(text)
	} else {
		starts_with_vowel_or_h(text)
	};
	if !vowel {
		return false;
	}
	match clitic {
		"j" | "t" | "m" | "n" => {
			(word.flags & dictionary::VERB != 0 || matches!(text, "en" | "y"))
				&& !(clitic == "t" && text == "est")
		}
		"s" => {
			word.flags & dictionary::VERB != 0
				|| matches!(text, "il" | "ils" | "on" | "y" | "en")
		}
		"c" => {
			matches!(text, "est" | "était" | "étaient" | "eût" | "en")
				|| text.starts_with("est-")
		}
		"d" => {
			word.flags & (dictionary::NOUN | dictionary::ADV) != 0
				|| (word.flags & dictionary::VERB != 0 && is_infinitive(text))
				|| matches!(
					text,
					"où" | "ici" | "ailleurs" | "abord" | "après" | "avant" | "ores"
				)
		}
		"l" => {
			word.flags & (dictionary::NOUN | dictionary::VERB | dictionary::ADJ) != 0
				|| text == "on"
		}
		_ => true,
	}
}

fn insert(found: &mut HashMap<String, Candidate>, candidate: Candidate) {
	match found.get(&candidate.text) {
		Some(existing) if existing.score >= candidate.score => {}
		_ => {
			found.insert(candidate.text.clone(), candidate);
		}
	}
}

/// `jusqua` -> `jusqu'à`: the remainder folds to the verb `a`, not `à`.
fn elision_remainder(clitic: &str, word: &dictionary::Word) -> (&'static str, f32) {
	if clitic == "jusqu" && word.text == "a" {
		if let Some(grave) = dictionary::exact("à") {
			return (grave.text, grave.log_freq);
		}
	}
	(word.text, word.log_freq)
}

fn insert_elision(
	found: &mut HashMap<String, Candidate>,
	clitic: &str,
	remainder: &str,
	log_freq: f32,
	penalty: f32,
	cost: f32,
) {
	let text = format!("{clitic}'{remainder}");
	insert(
		found,
		Candidate {
			text,
			cost,
			score: log_freq * AGGRESSIVE_WEIGHT - penalty - cost,
		},
	);
}

/// Candidate corrections for a typed token, best first. Empty when the token is
/// already a known word, too short, or contains an apostrophe.
pub fn candidates(typed: &str) -> Vec<Candidate> {
	let lower = typed.to_lowercase();
	if lower.chars().count() < 2 || lower.contains('\'') || dictionary::exact(&lower).is_some() {
		return Vec::new();
	}
	candidates_with(&lower, true)
}

fn candidates_with(lower: &str, elisions: bool) -> Vec<Candidate> {
	if lower.chars().count() < 2 || lower.contains('\'') {
		return Vec::new();
	}

	let folded_typed = fold(lower);
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
				score: word.log_freq * AGGRESSIVE_WEIGHT - cost,
			},
		);
	}

	if elisions {
		// Elision: `jai` -> `j'ai`, `taime` -> `t'aime`. The remainder may itself
		// contain a typo, so `taimx` and `taipe` can still reach `t'aime`.
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

			let folded_rest = fold(rest);
			if let Some(word) = dictionary::folded(&folded_rest) {
				if elision_allowed(clitic, word) {
					let (remainder, log_freq) = elision_remainder(clitic, word);
					insert_elision(
						&mut found,
						clitic,
						remainder,
						log_freq,
						penalty,
						APOSTROPHE_COST,
					);
				}
			}

			// Short remainders are covered by the exact lookup above; scanning
			// them for typos only invents forms like `j'air`.
			if folded_rest.len() < 3 {
				continue;
			}
			for word in dictionary::words() {
				if word.folded.len() < 2 || word.folded.len() != folded_rest.len() {
					continue;
				}
				if !elision_allowed(clitic, word) {
					continue;
				}
				let Some(distance) =
					distance(folded_rest.as_bytes(), word.folded.as_bytes(), MAX_COST)
				else {
					continue;
				};
				if distance == 0.0 {
					continue;
				}
				let (remainder, log_freq) = elision_remainder(clitic, word);
				insert_elision(
					&mut found,
					clitic,
					remainder,
					log_freq,
					penalty,
					APOSTROPHE_COST + distance,
				);
			}
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
			let length_diff = word_bytes.len().abs_diff(typed_bytes.len()) as f32;
			insert(
				&mut found,
				Candidate {
					text: word.text.to_string(),
					cost,
					score: word.log_freq * AGGRESSIVE_WEIGHT
						- cost
						- LENGTH_PENALTY * length_diff,
				},
			);
		}
	}

	let mut list: Vec<Candidate> = found.into_values().collect();
	sort_candidates(&mut list);
	list.truncate(MAX_RESULTS);
	list
}

/// Same-length distance where every substituted letter has to be a
/// neighbouring key (or an adjacent pair swapped). Returns `None` when the
/// input cannot be explained that way.
fn near_distance(a: &[u8], b: &[u8]) -> Option<f32> {
	let n = a.len();
	if n != b.len() || n > MAX_CHARS {
		return None;
	}
	let costs = costs();
	let mut dp = [f32::INFINITY; MAX_CHARS + 1];
	dp[0] = 0.0;
	for i in 1..=n {
		let step = costs[char_index(a[i - 1])][char_index(b[i - 1])];
		if step <= NEIGHBOUR_COST {
			dp[i] = dp[i - 1] + step;
		}
		if i >= 2 && a[i - 1] == b[i - 2] && a[i - 2] == b[i - 1] {
			let swapped = dp[i - 2] + TRANSPOSE_COST;
			if swapped < dp[i] {
				dp[i] = swapped;
			}
		}
	}
	if dp[n].is_finite() {
		Some(dp[n])
	} else {
		None
	}
}

fn sort_candidates(list: &mut [Candidate]) {
	list.sort_by(|a, b| {
		b.score
		. partial_cmp(&a.score)
		.unwrap_or(std::cmp::Ordering::Equal)
		. then_with(|| a.text.cmp(&b.text))
	});
}

/// Words where every typed letter is one key away from the intended one,
/// ranked by frequency. This is the narrow-keyboard rescue: an input can be
/// complete nonsense and still point at a common word.
fn add_neighbour_candidates(
	typed: &str,
	weight: f32,
	found: &mut HashMap<String, Candidate>,
) {
	let lower = typed.to_lowercase();
	let length = lower.chars().count();
	if length < 2 || lower.contains('\'') {
		return;
	}

	let folded_typed = fold(&lower);
	let typed_bytes = folded_typed.as_bytes();
	for word in dictionary::words() {
		if word.folded.len() != typed_bytes.len() {
			continue;
		}
		let Some(distance) = near_distance(typed_bytes, word.folded.as_bytes()) else {
			continue;
		};
		insert(
			found,
			Candidate {
				text: word.text.to_string(),
				cost: distance,
				score: word.log_freq * weight - distance,
			},
		);
	}
}

/// Elisions whose clitic and remainder are both one key away from the typed
/// token, with the apostrophe missing: `vqjmd` -> `t'aime`.
fn add_aggressive_elisions(typed: &str, found: &mut HashMap<String, Candidate>) {
	let lower = typed.to_lowercase();
	if lower.chars().count() < 3 || lower.contains('\'') {
		return;
	}
	let folded = fold(&lower);
	let bytes = folded.as_bytes();

	for &(clitic, penalty) in CLITICS {
		let clitic_len = clitic.len();
		if bytes.len() <= clitic_len {
			continue;
		}
		let Some(prefix_cost) = near_distance(&bytes[..clitic_len], clitic.as_bytes()) else {
			continue;
		};
		let rest = &bytes[clitic_len..];
		let shortest = if clitic_len >= 3 { 1 } else { 2 };
		if rest.len() < shortest {
			continue;
		}
		for word in dictionary::words() {
			if word.folded.len() != rest.len() {
				continue;
			}
			if !elision_allowed(clitic, word) {
				continue;
			}
			let Some(rest_cost) = near_distance(rest, word.folded.as_bytes()) else {
				continue;
			};
			let cost = prefix_cost + rest_cost + APOSTROPHE_COST;
			insert(
				found,
				Candidate {
					text: format!("{clitic}'{}", word.text),
					cost,
					score: word.log_freq * AGGRESSIVE_WEIGHT - cost - penalty,
				},
			);
		}
	}
}

pub fn aggressive_candidates(typed: &str) -> Vec<Candidate> {
	let lower = typed.to_lowercase();
	if lower.chars().count() < 3 || lower.contains('\'') {
		return Vec::new();
	}
	if dictionary::exact(&lower).is_some() {
		return Vec::new();
	}
	let mut found: HashMap<String, Candidate> = HashMap::new();
	add_neighbour_candidates(&lower, AGGRESSIVE_WEIGHT, &mut found);
	add_aggressive_elisions(&lower, &mut found);
	let mut list: Vec<Candidate> = found.into_values().collect();
	sort_candidates(&mut list);
	list.truncate(MAX_RESULTS);
	list
}

/// Candidates for the strip plus the auto-apply decision. Known words are
/// kept unless a near-key candidate is much more likely; unknown words always
/// get the best near-key interpretation, even when every letter is wrong.
pub fn suggest(typed: &str) -> Suggestion {
	let lower = typed.to_lowercase();
	let length = lower.chars().count();
	if length < 2 || lower.contains('\'') {
		return Suggestion::empty();
	}

	let known = dictionary::exact(&lower);
	let mut found: HashMap<String, Candidate> = HashMap::new();
	let typed_score = known.map(|word| {
		let score = word.log_freq * AGGRESSIVE_WEIGHT;
		insert(
			&mut found,
			Candidate {
				text: word.text.to_string(),
				cost: 0.0,
				score,
			},
		);
		score
	});

	if known.is_none() {
		for candidate in candidates_with(&lower, true) {
			insert(&mut found, candidate);
		}
	}
	add_neighbour_candidates(&lower, AGGRESSIVE_WEIGHT, &mut found);
	if known.is_none() && length >= 3 {
		add_aggressive_elisions(&lower, &mut found);
	}

	let mut list: Vec<Candidate> = found.into_values().collect();
	sort_candidates(&mut list);

	let Some(best) = list.first() else {
		return Suggestion::empty();
	};
	if known.is_some_and(|word| word.text == best.text) {
		// The typed word is already the best explanation.
		return Suggestion::empty();
	}

	if let Some(word) = known {
		list.retain(|candidate| candidate.text != word.text);
	}
	list.truncate(MAX_RESULTS);

	let Some(best) = list.first() else {
		return Suggestion::empty();
	};
	let mut apply = best.score >= MIN_SCORE;
	if let Some(typed_score) = typed_score {
		apply = apply && best.score - typed_score >= KNOWN_WORD_MARGIN;
	}
	if apply {
		if let Some(second) = list.get(1) {
			if best.score - second.score < AGGRESSIVE_MARGIN {
				apply = false;
			}
		}
	}
	Suggestion {
		candidates: list,
		apply,
	}
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
	fn frequency_outranks_a_cheaper_rare_word() {
		// `tapie` is one transposition away, but `t'aime` is the likely word.
		assert_eq!(best("taipe").as_deref(), Some("t'aime"));
		assert_eq!(best("taimx").as_deref(), Some("t'aime"));
	}

	#[test]
	fn frequency_orders_short_neighbours() {
		let list = candidates("ue");
		let position = |word: &str| list.iter().position(|c| c.text == word);
		let ne = position("ne").expect("ne");
		let eu = position("eu").expect("eu");
		assert!(ne < eu, "ne at {ne}, eu at {eu}");
		assert!(ne < 3, "ne must be visible in the 3-slot strip");
	}

	#[test]
	fn aggressive_rescue_finds_a_word_when_every_key_is_off() {
		let suggestion = suggest("geqnvx");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "beauté");
	}

	#[test]
	fn aggressive_rescue_keeps_a_clear_transposition() {
		let suggestion = suggest("tset");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "test");
	}

	#[test]
	fn normal_candidates_win_over_the_aggressive_rescue() {
		// One transposition away from a rare word, but the elided common word
		// is the better explanation.
		let suggestion = suggest("taipe");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "t'aime");
	}

	#[test]
	fn aggressive_rescue_reaches_an_elision_with_a_typo_in_the_clitic() {
		let suggestion = suggest("vqjmd");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "t'aime");
	}

	#[test]
	fn short_words_are_corrected_to_the_best_neighbour() {
		let suggestion = suggest("pq");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "la");
		let texts: Vec<&str> = suggestion
		.candidates
		.iter()
		.map(|candidate| candidate.text.as_str())
		.collect();
		assert!(texts.contains(&"ma"), "got {texts:?}");
	}

	#[test]
	fn known_rare_words_are_replaced_by_a_likely_neighbour() {
		let suggestion = suggest("pa");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "la");
	}

	#[test]
	fn known_common_words_are_left_alone() {
		assert!(suggest("la").candidates.is_empty());
		assert!(suggest("test").candidates.is_empty());
	}

	#[test]
	fn a_near_key_word_beats_a_length_changing_correction() {
		let suggestion = suggest("secg");
		assert!(suggestion.apply);
		assert_eq!(suggestion.candidates[0].text, "sert");
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
