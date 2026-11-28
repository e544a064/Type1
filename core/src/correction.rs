//! Layout-aware spelling correction.
//!
//! Candidates are generated from the shipped dictionary and scored with a
//! noisy-channel model: `P(typed | intended)` comes from the distance between
//! the physical keys, `P(intended)` from the word frequency.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::dictionary::{self, Word};
use crate::hangul;
use crate::language::Language;
use crate::layout;
use crate::lexicon::fold;
use crate::personal::PersonalLexicon;
use crate::profile::Profile;
use crate::script::Script;

/// Minimum score a candidate needs before it may replace the typed word.
const MIN_SCORE: f32 = 0.5;
const MAX_COST: f32 = 1.55;
/// Frequency dominates the ranking: among words whose letters are all one key
/// away, the common word is the likely one.
const AGGRESSIVE_WEIGHT: f32 = 1.5;
/// Changing the word length is less likely than pressing a neighbouring key.
const LENGTH_PENALTY: f32 = 2.0;
/// A known word is only overridden when a near-key candidate is this much more
/// likely (`pa` -> `la`, but `la` stays `la`). With the expanded dictionary a
/// rare but valid form must not be replaced by a merely common neighbour.
const KNOWN_WORD_MARGIN: f32 = 5.5;
/// The top candidate must lead the runner-up by this much.
const AGGRESSIVE_MARGIN: f32 = 0.25;
/// A substituted letter must be this close to count as a neighbouring key.
const NEIGHBOUR_COST: f32 = 1.5;
/// A second ring around the intended key, used when nothing closer exists.
const TWO_KEY_COST: f32 = 2.3;
/// A same-length one-letter spelling variant, not a keyboard slip.
const SPELLING_COST: f32 = 4.0;
const TRANSPOSE_COST: f32 = 0.6;
/// Cost of inserting the elision apostrophe.
const APOSTROPHE_COST: f32 = 0.15;
/// Two-key candidates are shown but need more confidence before auto-apply.
const TWO_KEY_MIN_SCORE: f32 = 2.0;
/// Hangul confidence bands use the total explanation cost across all jamo
/// changes, not just the worst single change. A word whose whole path is cheap
/// ranks above a merely frequent word with many changes.
const HANGUL_TIER1_COST: f32 = 1.75;
const HANGUL_TIER2_COST: f32 = 4.0;
/// A low-confidence tier-1 candidate is discarded when the best tier-2
/// candidate is this much more likely. Otherwise a rare word that happens to be
/// one key away can hide a much more plausible spelling variant.
const TIER1_FALLBACK_MARGIN: f32 = 5.5;
/// Very last resort: closest same-length word by weighted edit distance.
const LAST_RESORT_MIN_LEN: usize = 4;
const LAST_RESORT_TIE: f32 = 0.5;
const MAX_RESULTS: usize = 6;
/// Confidence for a curated contraction (`dont` -> `don't`). It must beat the
/// apostrophe-less form that web-derived dictionaries sometimes list as a word.
const CONTRACTION_LOG_FREQ: f32 = 8.0;
/// A contraction is much more likely than the bare key (`dont`), which is
/// normally a misspelling or a rare word. Applied to any candidate that
/// matches a contraction key before it is shown to the user.
const CONTRACTION_ALIAS_BONUS: f32 = 4.0;
/// Longest unit sequence the weighted distance will consider. Latin tokens are
/// at most 31 folded bytes; Korean words are at most 31 syllables, which can
/// decompose to roughly 150 jamo.
const MAX_CHARS: usize = 160;
/// `a`–`z` use 0–25, everything else (apostrophes, stray letters) shares 26.
const OTHER: usize = 26;
/// Compatibility jamo use U+3131–U+318E, i.e. 94 possible unit codes.
const UNIT_COUNT: usize = 94;

#[derive(Clone, Debug)]
pub struct Candidate {
	pub text: String,
	pub score: f32,
	/// 1 for a one-key slip, 2 for a farther slip or spelling variant,
	/// 3 for the closest word by edit distance when nothing else matches.
	pub tier: u8,
	/// True for a host personal-dictionary candidate. Such candidates are
	/// suggestions only and never trigger automatic replacement.
	pub personal: bool,
}

impl Candidate {
	fn normal(text: impl Into<String>, score: f32, tier: u8) -> Self {
		Candidate {
			text: text.into(),
			score,
			tier,
			personal: false,
		}
	}

	fn personal(text: impl Into<String>, score: f32, tier: u8) -> Self {
		Candidate {
			text: text.into(),
			score,
			tier,
			personal: true,
		}
	}
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

fn char_index(byte: u8) -> usize {
	match byte {
		b'a'..=b'z' => (byte - b'a') as usize,
		_ => OTHER,
	}
}

/// Pack a compatibility jamo into the unit byte used by the distance code.
fn jamo_unit(c: char) -> u8 {
	debug_assert!(hangul::is_jamo(c), "{c} is not a jamo");
	(c as u32 - 0x3131) as u8
}

type CostTable = [[f32; UNIT_COUNT]; UNIT_COUNT];

/// A jamo key on the layout, used to build the substitution cost table.
#[derive(Clone, Debug)]
struct JamoKey {
	code: usize,
	x: f32,
	y: f32,
}

/// Correction model for one keyboard profile: its key geometry and all of its
/// language dictionaries.
#[derive(Debug)]
pub struct Corrector {
	costs: CostTable,
	hangul: bool,
	languages: Vec<&'static Language>,
	elisions: bool,
	/// Korean words grouped by decomposed jamo length, built on first use.
	hangul_words: OnceLock<Vec<Vec<&'static Word>>>,
}

impl Corrector {
	pub fn new(profile: &Profile) -> Self {
		let hangul = profile.layout.script == Script::Hangul;
		let costs = if hangul {
			let mut keys = Vec::new();
			for letter in profile.layout.letters.iter() {
				let x = key_column(letter.id);
				let y = letter.row as f32 * 0.55;
				if let Some(jamo) = letter.commit.chars().next() {
					let code = jamo_unit(jamo) as usize;
					keys.push(JamoKey { code, x, y });
				}
				if let Some(jamo) = letter.shift_commit.chars().next() {
					let code = jamo_unit(jamo) as usize;
					keys.push(JamoKey { code, x, y });
				}
			}
			jamo_cost_table(&keys)
		} else {
			cost_table(profile.layout.letters)
		};
		Corrector {
			costs,
			hangul,
			languages: profile.languages.clone(),
			elisions: profile
			. languages
			. iter()
			. any(|language| language.elisions),
			hangul_words: OnceLock::new(),
		}
	}

	/// Typed units: folded ASCII bytes for Latin, packed jamo codes for Hangul.
	fn units(&self, text: &str) -> Vec<u8> {
		if self.hangul {
			hangul::decompose_keystrokes(text)
			. into_iter()
			. map(jamo_unit)
			. collect()
		} else {
			fold(text).into_bytes()
		}
	}

	fn word_units(&self, word: &'static Word) -> Cow<'static, [u8]> {
		if self.hangul {
			Cow::Owned(self.units(word.text))
		} else {
			Cow::Borrowed(word.folded.as_bytes())
		}
	}

	fn words_of_units_len(
		&self,
		length: usize,
	) -> Box<dyn Iterator<Item = &'static Word> + '_> {
		if self.hangul {
			let index = self.hangul_words.get_or_init(|| {
				let mut by_len: Vec<Vec<&'static Word>> = Vec::new();
				for language in &self.languages {
					for word in language.dictionary.words() {
						let len = hangul::decompose_keystrokes(word.text).len();
						if by_len.len() <= len {
							by_len.resize_with(len + 1, Vec::new);
						}
						by_len[len].push(word);
					}
				}
				by_len
			});
			Box::new(index.get(length).into_iter().flatten().copied())
		} else {
			Box::new(
				self.languages
				. iter()
				. flat_map(move |language| language.dictionary.words_of_len(length)),
			)
		}
	}

	fn exact(&self, word: &str) -> Option<&'static Word> {
		self.languages
		. iter()
		. filter_map(|language| language.dictionary.exact(word))
		. max_by(|a, b| a.log_freq.total_cmp(&b.log_freq))
	}

	fn folded(&self, form: &str) -> Option<&'static Word> {
		self.languages
		. iter()
		. filter_map(|language| language.dictionary.folded(form))
		. max_by(|a, b| a.log_freq.total_cmp(&b.log_freq))
	}

	#[cfg(test)]
	fn unit_index(&self, unit: u8) -> usize {
		if self.hangul {
			unit as usize
		} else {
			char_index(unit)
		}
	}

	#[cfg(test)]
	fn substitution_cost(&self, a: u8, b: u8) -> f32 {
		self.costs[self.unit_index(a)][self.unit_index(b)]
	}

}

/// Physical QWERTY column of a layout letter. Korean ids are `ko_q`, so the
/// last character still identifies the original key.
fn key_column(id: &str) -> f32 {
	match id.chars().last() {
		Some('q' | 'a' | 'z') => 0.0,
		Some('w' | 's' | 'x') => 1.0,
		Some('c'..='e') => 2.0,
		Some('r' | 'f' | 'v') => 3.0,
		Some('t' | 'g' | 'b') => 4.0,
		Some('y' | 'h' | 'n') => 5.0,
		Some('u' | 'j' | 'm') => 6.0,
		Some('i' | 'k') => 7.0,
		Some('o' | 'l') => 8.0,
		Some('p') => 9.0,
		_ => 0.0,
	}
}

/// Pairwise substitution costs for one layout.
fn cost_table(letters: &[layout::Letter]) -> CostTable {
	let mut positions: [Option<(f32, f32)>; 26] = [None; 26];
	for (index, letter) in letters.iter().enumerate() {
		if let Some(c) = letter.id.chars().next() {
			positions[(c as u8 - b'a') as usize] =
				Some((index as f32, letter.row as f32 * 0.55));
		}
	}

	let mut table = [[1.5f32; UNIT_COUNT]; UNIT_COUNT];
	table[OTHER][OTHER] = 0.0;
	for (a, row) in table.iter_mut().enumerate().take(26) {
		for (b, slot) in row.iter_mut().enumerate().take(26) {
			if let (Some((x1, y1)), Some((x2, y2))) = (positions[a], positions[b]) {
				let dx = x1 - x2;
				let dy = y1 - y2;
				*slot = (dx * dx + dy * dy).sqrt().min(2.6);
			}
		}
	}
	table
}

/// Jamo substitution costs from the physical Sebeolsik key positions. Jamo on
/// the same key (base and shift variant) cost nothing, so `ㅅ`/`ㅆ` and the
/// other pairs compete purely on dictionary frequency.
fn jamo_cost_table(keys: &[JamoKey]) -> CostTable {
	let mut table = [[1.5f32; UNIT_COUNT]; UNIT_COUNT];
	for (code, row) in table.iter_mut().enumerate() {
		row[code] = 0.0;
	}
	for a in keys {
		for b in keys {
			let dx = a.x - b.x;
			let dy = a.y - b.y;
			table[a.code][b.code] = (dx * dx + dy * dy).sqrt().min(2.6);
		}
	}
	table
}

impl Corrector {
	/// Weighted Damerau-Levenshtein distance, abandoned as soon as every path is
	/// more expensive than `max`. Operates on folded ASCII bytes or packed jamo
	/// units, depending on the profile script.
	fn distance(&self, a: &[u8], b: &[u8], max: f32) -> Option<f32> {
		let n = a.len();
		let m = b.len();
		if n.abs_diff(m) > 2 || n > MAX_CHARS || m > MAX_CHARS {
			return None;
		}

		let mut prev2 = [f32::INFINITY; MAX_CHARS + 1];
		let mut prev = [0.0f32; MAX_CHARS + 1];
		let mut cur = [0.0f32; MAX_CHARS + 1];
		let hangul = self.hangul;
		let costs = &self.costs;
		for (j, slot) in prev.iter_mut().enumerate().take(m + 1) {
			*slot = j as f32;
		}

		for i in 1..=n {
			cur[0] = i as f32;
			let mut row_min = cur[0];
			for j in 1..=m {
				let step = if hangul {
					costs[a[i - 1] as usize][b[j - 1] as usize]
				} else {
					costs[char_index(a[i - 1])][char_index(b[j - 1])]
				};
				let mut best = (
					(prev[j - 1] + step)
					. min(prev[j] + 1.0)
					. min(cur[j - 1] + 1.0)
				);
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
			| 'a' | 'à' | 'â' | 'ä'
			| 'e' | 'é' | 'è' | 'ê' | 'ë'
			| 'i' | 'î' | 'ï'
			| 'o' | 'ô' | 'ö'
			| 'u' | 'ù' | 'û' | 'ü'
			| 'y'
			| 'œ' | 'æ'
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
		Some(existing) if existing.tier < candidate.tier || (
			existing.tier == candidate.tier
			&& existing.score >= candidate.score
		) => {}
		_ => {
			found.insert(candidate.text.clone(), candidate);
		}
	}
}

/// Generated conjugations are valid words but less trustworthy than attested
/// dictionary entries, so they sort after attested candidates and never
/// auto-apply.
fn word_tier(word: &Word, tier: u8) -> u8 {
	if word.flags & dictionary::GENERATED != 0 {
		4
	} else {
		tier
	}
}

/// `jusqua` -> `jusqu'à`: the remainder folds to the verb `a`, not `à`.
fn elision_remainder(
	clitic: &str,
	word: &dictionary::Word,
	dictionary: &dictionary::Dictionary,
) -> (&'static str, f32) {
	if clitic == "jusqu" && word.text == "a" {
		if let Some(grave) = dictionary.exact("à") {
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
	tier: u8,
) {
	let text = format!("{clitic}'{remainder}");
	insert(
		found,
		Candidate::normal(
			text,
			log_freq * AGGRESSIVE_WEIGHT - penalty - cost,
			tier,
		),
	);
}

/// Candidate corrections for a typed token, best first. Empty when the token is
/// already a known word, too short, or contains an apostrophe.
pub fn candidates(typed: &str) -> Vec<Candidate> {
	crate::profile::default_profile()
	. corrector()
	. candidates(typed)
}

impl Corrector {
	pub(crate) fn candidates(&self, typed: &str) -> Vec<Candidate> {
		let lower = typed.to_lowercase();
		if lower.chars().count() < 2 || lower.contains('\'') || self.exact(&lower).is_some() {
			return Vec::new();
		}
		self.candidates_inner(&lower)
	}

	fn add_elisions(
		&self,
		lower: &str,
		found: &mut HashMap<String, Candidate>,
		include_remainder_typos: bool,
	) {
		if !self.elisions {
			return;
		}
		for language in self
		. languages
		. iter()
		. copied()
		. filter(|language| language.elisions)
		{
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
				if let Some(word) = language.dictionary.folded(&folded_rest) {
					if elision_allowed(clitic, word) {
						let (remainder, log_freq) =
							elision_remainder(clitic, word, language.dictionary);
						insert_elision(
							found,
							clitic,
							remainder,
							log_freq,
							penalty,
							APOSTROPHE_COST,
							1,
						);
					}
				}

				// Short remainders are covered by the exact lookup above; scanning
				// them for typos only invents forms like `j'air`.
				if !include_remainder_typos || folded_rest.len() < 3 {
					continue;
				}
				for word in language.dictionary.words_of_len(folded_rest.len()) {
					if !elision_allowed(clitic, word) {
						continue;
					}
					let Some(distance) = self.distance(
						folded_rest.as_bytes(),
						word.folded.as_bytes(),
						MAX_COST,
					) else {
						continue;
					};
					if distance == 0.0 {
						continue;
					}
					let (remainder, log_freq) =
						elision_remainder(clitic, word, language.dictionary);
					insert_elision(
						found,
						clitic,
						remainder,
						log_freq,
						penalty,
						APOSTROPHE_COST + distance,
						1,
					);
				}
			}
		}
	}

	fn candidates_inner(&self, lower: &str) -> Vec<Candidate> {
		let typed_units = self.units(lower);
		debug_assert!(!typed_units.is_empty() && !lower.contains('\''));
		let mut found: HashMap<String, Candidate> = HashMap::new();

		if !self.hangul {
			// Missing accents only (`ecole` -> `école`).
			let folded_typed = fold(lower);
			if let Some(word) = self.folded(&folded_typed) {
				let cost = 0.25;
				insert(
					&mut found,
					Candidate::normal(
						word.text,
						word.log_freq * AGGRESSIVE_WEIGHT - cost,
						word_tier(word, 1),
					),
				);
			}

			// Elision: `jai` -> `j'ai`, `taime` -> `t'aime`. The remainder may
			// itself contain a typo, so `taimx` and `taipe` can still reach
			// `t'aime`.
			self.add_elisions(lower, &mut found, true);
		}

		// Typographical and neighbouring-key errors.
		let typed_len = typed_units.len();
		for length in typed_len.saturating_sub(1)..=typed_len.saturating_add(1) {
			for word in self.words_of_units_len(length) {
				let word_units = self.word_units(word);
				if word_units.len() < 2 {
					continue;
				}
				if let Some(cost) = self.distance(&typed_units, &word_units, MAX_COST) {
					if cost == 0.0 {
						continue;
					}
					let length_diff = word_units.len().abs_diff(typed_units.len()) as f32;
					insert(
						&mut found,
						Candidate::normal(
							word.text,
							(
								word.log_freq * AGGRESSIVE_WEIGHT
								- cost
								- LENGTH_PENALTY * length_diff
							),
							word_tier(word, 1),
						),
					);
				}
			}
		}

		let mut list: Vec<Candidate> = found.into_values().collect();
		sort_candidates(&mut list);
		list.truncate(MAX_RESULTS);
		list
	}
}

impl Corrector {
	/// Same-length distance where every substituted letter has to be within
	/// `limit` of the intended key (or an adjacent pair swapped). Returns `None`
	/// when the input cannot be explained that way.
	fn near_distance(&self, a: &[u8], b: &[u8], limit: f32) -> Option<f32> {
		let n = a.len();
		if n != b.len() || n > MAX_CHARS {
			return None;
		}
		let mut dp = [f32::INFINITY; MAX_CHARS + 1];
		dp[0] = 0.0;
		let hangul = self.hangul;
		let costs = &self.costs;
		for i in 1..=n {
			let step = if hangul {
				costs[a[i - 1] as usize][b[i - 1] as usize]
			} else {
				costs[char_index(a[i - 1])][char_index(b[i - 1])]
			};
			if step <= limit {
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

	/// One arbitrary letter changed, with the rest identical. Covers spelling
	/// variants that are nowhere near on the keyboard (`commant` -> `comment`).
	fn spelling_distance(a: &[u8], b: &[u8]) -> Option<f32> {
		if a.len() != b.len() || a.len() < 4 {
			return None;
		}
		let mut differences = 0;
		for (&x, &y) in a.iter().zip(b) {
			if x != y {
				differences += 1;
				if differences > 1 {
					return None;
				}
			}
		}
		if differences == 1 {
			Some(SPELLING_COST)
		} else {
			None
		}
	}

	/// Key-slip distance only: one-key ring, then the two-key ring.
	fn keyboard_distance(&self, a: &[u8], b: &[u8]) -> Option<(f32, u8)> {
		if let Some(distance) = self.near_distance(a, b, NEIGHBOUR_COST) {
			return Some((distance, 1));
		}
		self.near_distance(a, b, TWO_KEY_COST)
		. map(|distance| (distance, 2))
	}

	/// Closest explanation, with a tier: 1 for one-key slips, 2 for the second
	/// key ring or a one-letter spelling variant.
	fn candidate_distance(&self, a: &[u8], b: &[u8]) -> Option<(f32, u8)> {
		let result = self
		. keyboard_distance(a, b)
		. or_else(|| Self::spelling_distance(a, b).map(|distance| (distance, 2)));
		if !self.hangul {
			return result;
		}
		result.map(|(distance, _)| {
			let tier = if distance <= HANGUL_TIER1_COST {
				1
			} else if distance <= HANGUL_TIER2_COST {
				2
			} else {
				3
			};
			(distance, tier)
		})
	}
}

impl Corrector {
	/// Very last resort: the same-length dictionary word with the smallest
	/// weighted edit distance. Only used when no tier-1/2 candidate exists.
	fn last_resort_candidates(&self, lower: &str) -> Vec<Candidate> {
		let typed_units = self.units(lower);
		let length = typed_units.len();
		if length < LAST_RESORT_MIN_LEN {
			return Vec::new();
		}
		let max = length as f32 * 2.6;

		let mut best = f32::INFINITY;
		let mut matches: Vec<(&dictionary::Word, f32)> = Vec::new();
		for word in self.words_of_units_len(length) {
			let word_units = self.word_units(word);
			let Some(distance) = self.distance(&typed_units, &word_units, max) else {
				continue;
			};
			if distance < best {
				best = distance;
				matches.clear();
			}
			if distance <= best + LAST_RESORT_TIE {
				matches.push((word, distance));
			}
		}
		matches.retain(|(_, distance)| *distance <= best + LAST_RESORT_TIE);
		matches.sort_by(|a, b| {
			a.1
			. partial_cmp(&b.1)
			. unwrap_or(std::cmp::Ordering::Equal)
			. then_with(|| {
				b.0.log_freq
				. partial_cmp(&a.0.log_freq)
				. unwrap_or(std::cmp::Ordering::Equal)
			})
			. then_with(|| a.0.text.cmp(b.0.text))
		});

		matches
		. into_iter()
		. take(MAX_RESULTS)
		. map(|(word, distance)| {
			Candidate::normal(
				word.text,
				-distance + word.log_freq * 0.001,
				word_tier(word, 3),
			)
		})
		. collect()
	}
}

fn sort_candidates(list: &mut [Candidate]) {
	list.sort_by(|a, b| {
		a.tier
		. cmp(&b.tier)
		. then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
		. then_with(|| a.text.cmp(&b.text))
	});
}

impl Corrector {
	/// A contraction key that is also an ordinary word (`lets`). Its ordinary
	/// candidate is kept alongside the contraction.
	fn is_ambiguous_contraction(&self, lower: &str) -> bool {
		self
		. languages
		. iter()
		. any(|language| {
			let language = *language;
			language.ambiguous_contractions.contains(&lower)
			&& !self.languages.iter().any(|other| {
				!std::ptr::eq(*other, language)
				&& other.dictionary.exact(lower).is_some()
			})
		})
	}

	/// Restore the apostrophe in curated contractions (`dont` -> `don't`).
	///
	/// Any normal candidate that equals a contraction key is renamed to the
	/// contraction. This means `eont` or `douv`, whose best ordinary candidate
	/// is `dont`, also get `don't` at a higher probability.
	fn add_contractions(&self, lower: &str, found: &mut HashMap<String, Candidate>) {
		for language in &self.languages {
			let language = *language;
			// `dont` is a French word too: in a mixed profile leave the
			// other language's valid word alone.
			let conflicted = self.languages.iter().any(|other| {
				!std::ptr::eq(*other, language)
				&& other.dictionary.exact(lower).is_some()
			});
			if conflicted {
				continue;
			}
			for &(key, contraction) in language.contractions {
				let ambiguous = language.ambiguous_contractions.contains(&key);
				if lower == key {
					insert(
						found,
						Candidate::normal(
							contraction.to_string(),
							CONTRACTION_LOG_FREQ * AGGRESSIVE_WEIGHT,
							1,
						),
					);
				}
				// Keep an ambiguous ordinary word next to its contraction
				// (`lets` and `let's`); otherwise replace the ordinary form.
				let existing = if ambiguous {
					found.get(key).cloned()
				} else {
					found.remove(key)
				};
				if let Some(candidate) = existing {
					insert(
						found,
						Candidate::normal(
							contraction.to_string(),
							candidate.score + CONTRACTION_ALIAS_BONUS,
							candidate.tier,
						),
					);
				}
			}
		}
	}

	/// Close personal-dictionary entries. They are suggestions only; the
	/// candidate flag prevents automatic replacement.
	fn add_personal_candidates(
		&self,
		lower: &str,
		personal: &PersonalLexicon,
		found: &mut HashMap<String, Candidate>,
	) {
		if personal.is_empty() {
			return;
		}
		let typed_units = self.units(lower);
		for word in personal.words() {
			if !word.matches(&self.languages) {
				continue;
			}
			let word_units = self.units(word.text());
			if word_units == typed_units {
				continue;
			}
			let Some((distance, tier)) = self.candidate_distance(
				&typed_units,
				&word_units,
			) else {
				continue;
			};
			insert(
				found,
				Candidate::personal(
					word.text(),
					word.score() - distance,
					tier,
				),
			);
		}
	}

	/// Words where every typed unit is one or two keys away from the intended
	/// one, ranked by frequency. This is the narrow-keyboard rescue: an input can
	/// be complete nonsense and still point at a common word.
	fn add_neighbour_candidates(&self, lower: &str, found: &mut HashMap<String, Candidate>) {
		let typed_units = self.units(lower);
		for word in self.words_of_units_len(typed_units.len()) {
			let word_units = self.word_units(word);
			let Some((distance, tier)) =
				self.candidate_distance(&typed_units, &word_units)
			else {
				continue;
			};
			insert(
				found,
				Candidate::normal(
					word.text,
					word.log_freq * AGGRESSIVE_WEIGHT - distance,
					word_tier(word, tier),
				),
			);
		}
	}

	/// Elisions whose clitic and remainder are both key slips, with the apostrophe
	/// missing: `vqjmd` -> `t'aime`.
	fn add_aggressive_elisions(&self, lower: &str, found: &mut HashMap<String, Candidate>) {
		if !self.elisions {
			return;
		}
		let folded = fold(lower);
		let bytes = folded.as_bytes();

		for language in (
			self
			. languages
			. iter()
			. copied()
			. filter(|language| language.elisions)
		) {
			for &(clitic, penalty) in CLITICS {
				let clitic_len = clitic.len();
				if bytes.len() <= clitic_len {
					continue;
				}
				let Some((prefix_cost, prefix_tier)) =
					self.keyboard_distance(&bytes[..clitic_len], clitic.as_bytes())
				else {
					continue;
				};
				let rest = &bytes[clitic_len..];
				let shortest = if clitic_len >= 3 { 1 } else { 2 };
				if rest.len() < shortest {
					continue;
				}
				for word in language.dictionary.words_of_len(rest.len()) {
					if !elision_allowed(clitic, word) {
						continue;
					}
					let Some((rest_cost, rest_tier)) =
						self.keyboard_distance(rest, word.folded.as_bytes())
					else {
						continue;
					};
					let cost = prefix_cost + rest_cost + APOSTROPHE_COST;
					let (remainder, log_freq) =
						elision_remainder(clitic, word, language.dictionary);
					let tier = prefix_tier.max(rest_tier);
					insert_elision(found, clitic, remainder, log_freq, penalty, cost, tier);
				}
			}
		}
	}
}

/// Candidates for the strip plus the auto-apply decision for the default
/// profile. Prefer [`Profile::suggest`](crate::profile::Profile::suggest) when a
/// specific keyboard profile is active.
pub fn suggest(typed: &str) -> Suggestion {
	crate::profile::default_profile().suggest(typed)
}

impl Corrector {
	/// Candidates for the strip plus the auto-apply decision. Known words are
	/// kept unless a near-key candidate is much more likely; unknown words always
	/// get the best near-key interpretation, even when every letter is wrong.
	pub fn suggest(&self, typed: &str) -> Suggestion {
		self.suggest_with_personal(typed, &PersonalLexicon::default())
	}

	/// Same as [`Self::suggest`], with a host-provided personal dictionary.
	/// Personal exact matches are protected known words; close personal matches
	/// are suggestion-only candidates.
	pub fn suggest_with_personal(
		&self,
		typed: &str,
		personal: &PersonalLexicon,
	) -> Suggestion {
		let lower = typed.to_lowercase();
		let length = if self.hangul {
			self.units(&lower).len()
		} else {
			lower.chars().count()
		};
		if lower.contains('\'') {
			return Suggestion::empty();
		}
		if length == 1 && !self.hangul {
			return self.single_letter_suggestions(&lower);
		}
		if length == 0 {
			return Suggestion::empty();
		}

		let dictionary_known = self.exact(&lower);
		let personal_known = personal.exact(&fold(&lower), &self.languages);
		let personal_exact = personal_known.is_some();
		let mut known_texts: Vec<&str> = Vec::new();
		let mut found: HashMap<String, Candidate> = HashMap::new();
		let mut typed_score: Option<f32> = None;

		if let Some(word) = dictionary_known {
			let score = word.log_freq * AGGRESSIVE_WEIGHT;
			insert(
				&mut found,
				Candidate::normal(word.text, score, 1),
			);
			known_texts.push(word.text);
			typed_score = Some(score);
		}
		if let Some(word) = personal_known {
			let score = word.score();
			insert(
				&mut found,
				Candidate::personal(word.text(), score, 1),
			);
			known_texts.push(word.text());
			typed_score = Some(typed_score.map_or(score, |existing| existing.max(score)));
		}
		let is_known = !known_texts.is_empty();

		if !is_known {
			for candidate in self.candidates_inner(&lower) {
				insert(&mut found, candidate);
			}
		} else if !self.hangul {
			// A known word in one language (for example English `jai`) must not
			// hide the elision that another active language can build from it.
			self.add_elisions(&lower, &mut found, false);
		}
		self.add_neighbour_candidates(&lower, &mut found);
		self.add_personal_candidates(&lower, personal, &mut found);
		if !is_known && !self.hangul && length >= 3 {
			self.add_aggressive_elisions(&lower, &mut found);
		}
		if !self.hangul {
			self.add_contractions(&lower, &mut found);
		}

		let mut list: Vec<Candidate> = found.into_values().collect();
		let best_tier2 = (
			list
			. iter()
			. filter(|candidate| candidate.tier == 2)
			. map(|candidate| candidate.score)
			. fold(f32::NEG_INFINITY, f32::max)
		);
		if best_tier2.is_finite() {
			list.retain(|candidate| {
				candidate.tier != 1
				|| (
					known_texts
					. iter()
					. any(|text| *text == candidate.text)
				)
				|| candidate.score >= best_tier2 - TIER1_FALLBACK_MARGIN
			});
		}
		sort_candidates(&mut list);
		if list.is_empty() {
			list = self.last_resort_candidates(&lower);
		}

		if list.is_empty() {
			return Suggestion::empty();
		}
		if !self.is_ambiguous_contraction(&lower) {
			for text in &known_texts {
				list.retain(|candidate| candidate.text != *text);
			}
		}
		if is_known {
			// Keep the strip useful when the typed word happens to be valid:
			// only offer alternatives with a plausible score.
			list.retain(|candidate| candidate.score >= MIN_SCORE);
		}
		list.truncate(MAX_RESULTS);

		let Some(best) = list.first() else {
			return Suggestion::empty();
		};
		let mut apply = match best.tier {
			1 => best.score >= MIN_SCORE,
			2 => best.score >= TWO_KEY_MIN_SCORE,
			_ => false,
		};
		if let Some(typed_score) = typed_score {
			apply = apply && best.score - typed_score >= KNOWN_WORD_MARGIN;
		}
		if apply {
			let runner_up = list.iter().skip(1).find(|candidate| candidate.tier == best.tier);
			if let Some(second) = runner_up {
				if best.score - second.score < AGGRESSIVE_MARGIN {
					apply = false;
				}
			}
		}
		if personal_exact || best.personal {
			apply = false;
		}
		Suggestion {
			candidates: list,
			apply,
		}
	}

	/// Candidates for a single typed letter: one-letter words first (including
	/// accented variants and real words among the neighbours), then the
	/// neighbouring keys themselves.
	fn single_letter_suggestions(&self, typed: &str) -> Suggestion {
		let Some(letter) = typed.chars().next() else {
			return Suggestion::empty();
		};
		let folded = fold(&letter.to_string());
		if folded.chars().count() != 1 {
			return Suggestion::empty();
		}
		let Some(base) = folded.chars().next() else {
			return Suggestion::empty();
		};
		if !base.is_ascii_alphabetic() {
			return Suggestion::empty();
		}

		let typed_index = char_index(base as u8);
		let mut neighbours: Vec<(char, f32)> = Vec::new();
		for (index, byte) in (b'a'..=b'z').enumerate() {
			let neighbour = byte as char;
			if neighbour == base {
				continue;
			}
			let cost = self.costs[typed_index][index];
			if cost > 0.0 && cost <= TWO_KEY_COST {
				neighbours.push((neighbour, cost));
			}
		}
		neighbours.sort_by(|a, b| {
			a.1
			. partial_cmp(&b.1)
			. unwrap_or(std::cmp::Ordering::Equal)
			. then_with(|| a.0.cmp(&b.0))
		});
		if neighbours.is_empty() {
			// Defensive: keep at least the closest other letter.
			let mut closest = None;
			for (index, byte) in (b'a'..=b'z').enumerate() {
				let neighbour = byte as char;
				if neighbour == base {
					continue;
				}
				let cost = self.costs[typed_index][index];
				if closest.is_none_or(|(_, best)| cost < best) {
					closest = Some((neighbour, cost));
				}
			}
			if let Some(entry) = closest {
				neighbours.push(entry);
			}
		}

		let mut allowed: HashSet<char> = HashSet::new();
		allowed.insert(base);
		for (neighbour, _) in &neighbours {
			allowed.insert(*neighbour);
		}

		let mut by_base: HashMap<char, HashMap<String, f32>> = HashMap::new();
		for word in self.words_of_units_len(1) {
			let Some(word_base) = word.folded.chars().next() else {
				continue;
			};
			if !allowed.contains(&word_base) {
				continue;
			}
			let entry = by_base.entry(word_base).or_default();
			if entry.get(word.text).is_none_or(|freq| *freq < word.log_freq) {
				entry.insert(word.text.to_string(), word.log_freq);
			}
		}

		let mut list: Vec<Candidate> = Vec::new();
		let mut added: HashSet<String> = HashSet::new();
		for allowed_base in &allowed {
			let Some(entries) = by_base.get(allowed_base) else {
				continue;
			};
			let base_text = allowed_base.to_string();
			for (text, log_freq) in entries {
				if text == &base_text || text.as_str() == typed {
					continue;
				}
				if !added.insert(text.clone()) {
					continue;
				}
				list.push(Candidate::normal(
					text.clone(),
					*log_freq * AGGRESSIVE_WEIGHT,
					1,
				));
			}
		}
		// The typed letter itself as a word (`a` when typing `à`).
		let typed_base_text = base.to_string();
		if typed_base_text != typed && !added.contains(&typed_base_text) {
			if let Some(log_freq) = (
				by_base
				. get(&base)
				. and_then(|entries| entries.get(&typed_base_text))
			) {
				added.insert(typed_base_text.clone());
				list.push(Candidate::normal(
					typed_base_text,
					*log_freq * AGGRESSIVE_WEIGHT,
					1,
				));
			}
		}
		list.sort_by(|a, b| {
			b.score
			. partial_cmp(&a.score)
			. unwrap_or(std::cmp::Ordering::Equal)
			. then_with(|| a.text.cmp(&b.text))
		});

		// Keep the raw neighbour letters even when a word with the same base
		// already exists (`I` and `i` are both useful for a mistyped `k`).
		for (neighbour, cost) in neighbours {
			let text = neighbour.to_string();
			if added.insert(text.clone()) {
				list.push(Candidate::normal(text, -cost, 2));
			}
		}
		list.truncate(MAX_RESULTS);
		Suggestion {
			candidates: list,
			apply: false,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::personal::PersonalWord;

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
		let texts: Vec<&str> = (
			suggestion
			. candidates
			. iter()
			. map(|candidate| candidate.text.as_str())
			. collect()
		);
		assert!(texts.contains(&"ma"), "got {texts:?}");
	}

	#[test]
	fn known_rare_words_are_replaced_by_a_likely_neighbour() {
		let suggestion = suggest("pa");
		assert!(suggestion.apply);
		assert!(
			suggestion
			. candidates
			. iter()
			. any(|candidate| candidate.text == "la"),
			"candidates: {:?}",
			suggestion
			. candidates
			. iter()
			. map(|candidate| candidate.text.as_str())
			. collect::<Vec<_>>()
		);
	}

	#[test]
	fn known_common_words_are_not_autocorrected() {
		for word in ["la", "test"] {
			let suggestion = suggest(word);
			assert!(!suggestion.apply, "{word} should not be replaced");
			assert!(
				suggestion
				. candidates
				. iter()
				. all(|candidate| candidate.text != word),
				"{word} should not suggest itself"
			);
		}
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
	fn known_words_do_not_suggest_themselves() {
		for word in ["test", "dire"] {
			assert!(
				candidates(word)
				. iter()
				. all(|candidate| candidate.text != word)
			);
		}
	}

	#[test]
	fn rare_dictionary_words_are_known() {
		for word in ["radiologie", "radiologue"] {
			let suggestion = suggest(word);
			assert!(!suggestion.apply);
			assert!(
				suggestion
				. candidates
				. iter()
				. all(|candidate| candidate.text != word)
			);
		}
		let typo = suggest("radiologir");
		assert!(
			typo
			. candidates
			. iter()
			. any(|candidate| candidate.text == "radiologie"),
			"candidates: {:?}",
			typo
			. candidates
			. iter()
			. map(|candidate| candidate.text.as_str())
			. collect::<Vec<_>>()
		);
	}

	#[test]
	fn low_confidence_candidates_stay_below_the_margin() {
		let list = candidates("mmm");
		let best = list.first().expect("a weak candidate");
		assert!(best.score < MIN_SCORE, "score {}", best.score);
	}

	#[test]
	fn personal_words_are_protected_and_suggested() {
		let profile = Profile::from_ids("qwerty", &["en"]).unwrap();
		let personal = PersonalLexicon::new(vec![PersonalWord::new("zorblat", None, 200)]);

		let exact = profile.suggest_with_personal("zorblat", &personal);
		assert!(!exact.apply);
		assert!(
			exact
			. candidates
			. iter()
			. all(|candidate| candidate.text != "zorblat")
		);

		let typo = profile.suggest_with_personal("zorbkat", &personal);
		assert!(!typo.apply);
		assert!(
			typo
			. candidates
			. iter()
			. any(|candidate| candidate.text == "zorblat"),
			"candidates: {:?}",
			typo
			. candidates
			. iter()
			. map(|candidate| candidate.text.as_str())
			. collect::<Vec<_>>()
		);
	}

	#[test]
	fn personal_words_follow_their_locale() {
		let profile = Profile::from_ids("qwerty", &["en"]).unwrap();
		let personal = PersonalLexicon::new(vec![PersonalWord::new(
			"zorblat",
			Some("fr_FR".into()),
			200,
		)]);
		let suggestion = profile.suggest_with_personal("zorbkat", &personal);
		assert!(
			suggestion
			. candidates
			. iter()
			. all(|candidate| candidate.text != "zorblat")
		);
	}

	#[test]
	fn shift_variants_have_no_extra_correction_cost() {
		let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
		let corrector = profile.corrector();
		for (base, shifted) in [
			('ㅂ', 'ㅃ'),
			('ㅈ', 'ㅉ'),
			('ㄷ', 'ㄸ'),
			('ㄱ', 'ㄲ'),
			('ㅅ', 'ㅆ'),
			('ㅐ', 'ㅒ'),
			('ㅔ', 'ㅖ'),
		] {
			let cost = |from: char, to: char| {
				corrector.substitution_cost(jamo_unit(from), jamo_unit(to))
			};
			assert_eq!(cost(base, shifted), 0.0, "{base} -> {shifted}");
			assert_eq!(cost(shifted, base), 0.0, "{shifted} -> {base}");
		}
	}

	#[test]
	fn jamo_costs_follow_key_distance() {
		let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
		let corrector = profile.corrector();
		let cost = |from: char, to: char| {
			corrector.substitution_cost(jamo_unit(from), jamo_unit(to))
		};
		assert_eq!(cost('ㄷ', 'ㄸ'), 0.0);
		assert!((cost('ㄱ', 'ㄹ') - 0.55).abs() < 0.01);
		assert!(cost('ㄱ', 'ㅐ') > TWO_KEY_COST, "{}", cost('ㄱ', 'ㅐ'));
		assert!(cost('ㄹ', 'ㅊ') <= NEIGHBOUR_COST);
	}

	#[test]
	fn garbage_has_no_candidates() {
		assert!(candidates("xqzv").is_empty());
	}
}
