//! Generates the bulk fixtures in `core/tests/data/`:
//!
//! - `autocorrect_cases.tsv` (mixed typo classes)
//! - `two_key_cases.tsv` (typos with two-key substitutions)
//! - `spelling_cases.tsv` (one-letter spelling variants)
//!
//! The cases are deterministic variants of dictionary words that the current
//! corrector resolves to the intended word. They form a bulk regression suite:
//! rerun this example after tuning the corrector and commit the new fixtures.
//!
//!     cargo run --release --example make_autocorrect_cases

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;

use type1_core::correction::suggest;
use type1_core::dictionary;
use type1_core::layout::LETTERS;

const MIXED_TARGET: usize = 2_500;
const TWO_KEY_TARGET: usize = 2_000;
const SPELLING_TARGET: usize = 2_000;

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

fn letters() -> Vec<char> {
	LETTERS
	.iter()
	.filter_map(|letter| letter.id.chars().next())
	.collect()
}

fn neighbour(c: char, rng: &mut Rng, order: &[char]) -> char {
	let Some(index) = order.iter().position(|&x| x == c) else {
		return c;
	};
	let next = if rng.below(2) == 0 {
		(index + 1).min(order.len() - 1)
	} else {
		index.saturating_sub(1)
	};
	order[next]
}

fn substitute(word: &str, count: usize, rng: &mut Rng, order: &[char]) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	for _ in 0..count {
		let i = rng.below(chars.len());
		chars[i] = neighbour(chars[i], rng, order);
	}
	chars.into_iter().collect()
}

fn far_neighbour(c: char, rng: &mut Rng, order: &[char]) -> char {
	let Some(index) = order.iter().position(|&x| x == c) else {
		return c;
	};
	let next = if rng.below(2) == 0 {
		(index + 2).min(order.len() - 1)
	} else {
		index.saturating_sub(2)
	};
	order[next]
}

fn substitute_far(word: &str, count: usize, rng: &mut Rng, order: &[char]) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	for _ in 0..count {
		let i = rng.below(chars.len());
		chars[i] = far_neighbour(chars[i], rng, order);
	}
	chars.into_iter().collect()
}

fn spelling(word: &str, rng: &mut Rng, order: &[char]) -> String {
	let chars: Vec<char> = word.chars().collect();
	if chars.len() < 4 {
		return word.to_string();
	}
	let positions: Vec<usize> = (0..chars.len())
	.filter(|&index| order.contains(&chars[index]))
	.collect();
	if positions.is_empty() {
		return word.to_string();
	}
	let i = positions[rng.below(positions.len())];
	let Some(original) = order.iter().position(|&c| c == chars[i]) else {
		return word.to_string();
	};
	let far: Vec<char> = order
	.iter()
	.enumerate()
	.filter(|(index, _)| index.abs_diff(original) >= 3)
	.map(|(_, &c)| c)
	.collect();
	if far.is_empty() {
		return word.to_string();
	}
	let mut chars = chars;
	chars[i] = far[rng.below(far.len())];
	chars.into_iter().collect()
}

fn transpose(word: &str, rng: &mut Rng) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	if chars.len() < 2 {
		return word.to_string();
	}
	let i = rng.below(chars.len() - 1);
	chars.swap(i, i + 1);
	chars.into_iter().collect()
}

fn remove(word: &str, rng: &mut Rng) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	if chars.len() < 4 {
		return word.to_string();
	}
	let i = rng.below(chars.len());
	chars.remove(i);
	chars.into_iter().collect()
}

fn insert(word: &str, rng: &mut Rng, order: &[char]) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	let i = rng.below(chars.len());
	let c = neighbour(chars[i], rng, order);
	chars.insert(i, c);
	chars.into_iter().collect()
}

fn accentless(word: &str) -> String {
	type1_core::lexicon::fold(word)
}

fn visible_limit(typed: &str) -> usize {
	match typed.chars().count() {
		0..=3 => 6,
		4 => 5,
		5 => 4,
		_ => 3,
	}
}

fn accepted(typed: &str, word: &str, tier: u8) -> bool {
	if typed == word || typed.is_empty() || dictionary::exact(typed).is_some() {
		return false;
	}
	let suggestion = suggest(typed);
	suggestion
	. candidates
	. iter()
	. take(visible_limit(typed))
	. any(|candidate| candidate.text == word && candidate.tier == tier)
}

fn write_cases(path: &str, cases: &mut [(String, &'static str)]) {
	cases.sort_by(|a, b| a.0.cmp(&b.0));
	let mut out = String::new();
	for (typed, word) in cases.iter() {
		writeln!(out, "{typed}\t{word}").unwrap();
	}
	fs::create_dir_all("tests/data").unwrap();
	fs::write(path, out).unwrap();
	println!("wrote {} cases to {path}", cases.len());
}

fn main() {
	let order = letters();
	let mut rng = Rng(0x2545_f491_4f6c_dd1d);
	let pool: Vec<&'static str> = dictionary::words()
	.iter()
	.filter(|w| {
		let len = w.text.chars().count();
		(4..=10).contains(&len) && w.log_freq > 2.5 && !w.text.contains('\'')
	})
	.map(|w| w.text)
	.collect();

	let mut mixed = Vec::new();
	let mut two_key = Vec::new();
	let mut spelling_cases = Vec::new();
	let mut mixed_seen = HashSet::new();
	let mut two_key_seen = HashSet::new();
	let mut spelling_seen = HashSet::new();
	let mut index = 0usize;

	while (mixed.len() < MIXED_TARGET
		|| two_key.len() < TWO_KEY_TARGET
		|| spelling_cases.len() < SPELLING_TARGET)
		&& index < pool.len() * 120
	{
		let word = pool[index % pool.len()];
		index += 1;

		if mixed.len() < MIXED_TARGET {
			let variants = [
				substitute(word, 1, &mut rng, &order),
				substitute_far(word, 1, &mut rng, &order),
				spelling(word, &mut rng, &order),
				substitute(word, 2, &mut rng, &order),
				substitute(word, 3, &mut rng, &order),
				transpose(word, &mut rng),
				remove(word, &mut rng),
				insert(word, &mut rng, &order),
				accentless(word),
			];
			for typed in variants {
				if typed == word || typed.is_empty() || !mixed_seen.insert(typed.clone()) {
					continue;
				}
				if dictionary::exact(&typed).is_some() {
					continue;
				}
				let suggestion = suggest(&typed);
				let best = suggestion
				.candidates
				.first()
				.map(|candidate| candidate.text.as_str());
				if suggestion.apply && best == Some(word) {
					mixed.push((typed, word));
					if mixed.len() >= MIXED_TARGET {
						break;
					}
				}
			}
		}

		if two_key.len() < TWO_KEY_TARGET {
			let variants = [
				substitute_far(word, 1, &mut rng, &order),
				substitute_far(word, 2, &mut rng, &order),
				substitute_far(word, 3, &mut rng, &order),
			];
			for typed in variants {
				if !two_key_seen.insert(typed.clone()) {
					continue;
				}
				if accepted(&typed, word, 2) {
					two_key.push((typed, word));
					if two_key.len() >= TWO_KEY_TARGET {
						break;
					}
				}
			}
		}

		if spelling_cases.len() < SPELLING_TARGET {
			let variants = [
				spelling(word, &mut rng, &order),
				spelling(word, &mut rng, &order),
				spelling(word, &mut rng, &order),
			];
			for typed in variants {
				if !spelling_seen.insert(typed.clone()) {
					continue;
				}
				if accepted(&typed, word, 2) {
					spelling_cases.push((typed, word));
					if spelling_cases.len() >= SPELLING_TARGET {
						break;
					}
				}
			}
		}
	}

	write_cases("tests/data/autocorrect_cases.tsv", &mut mixed);
	write_cases("tests/data/two_key_cases.tsv", &mut two_key);
	write_cases("tests/data/spelling_cases.tsv", &mut spelling_cases);
}
