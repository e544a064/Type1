//! Generates the all-caps bulk fixture in `core/tests/data/`.
//!
//! The cases are deterministic typo variants of common English words that the
//! corrector resolves to the intended word. They are uppercased so the runtime
//! test exercises caps-lock autocorrection.
//!
//!     cargo run --release --example make_all_caps_cases

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;

use type1_core::dictionary::ENGLISH as ENGLISH_DICTIONARY;
use type1_core::language::ENGLISH as ENGLISH_LANGUAGE;
use type1_core::layout::QWERTY;
use type1_core::Profile;

const TARGET: usize = 2_000;

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
	QWERTY
	. letters
	. iter()
	. filter_map(|letter| letter.id.chars().next())
	. collect()
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

fn main() {
	let order = letters();
	let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
	let profile = Profile::new(&QWERTY, vec![&ENGLISH_LANGUAGE]);
	let pool: Vec<&'static str> = ENGLISH_DICTIONARY
	. words()
	. iter()
	.filter(|word| {
		let length = word.text.chars().count();
		(4..=10).contains(&length)
			&& word.log_freq > 2.5
			&& word.text.chars().all(|c| c.is_ascii_alphabetic())
	})
	.map(|word| word.text)
	.collect();

	let mut cases: Vec<(String, String)> = Vec::new();
	let mut seen = HashSet::new();
	let mut index = 0usize;
	while cases.len() < TARGET && index < pool.len() * 120 {
		let word = pool[index % pool.len()];
		index += 1;
		let variants = [
			substitute(word, 1, &mut rng, &order),
			substitute(word, 2, &mut rng, &order),
			transpose(word, &mut rng),
			remove(word, &mut rng),
			insert(word, &mut rng, &order),
		];
		for typed in variants {
			if typed == word || typed.is_empty() || !seen.insert(typed.clone()) {
				continue;
			}
			let suggestion = profile.suggest(&typed);
			let best = suggestion
			. candidates
			. first()
			.map(|candidate| candidate.text.as_str());
			if suggestion.apply && best == Some(word) {
				cases.push((typed.to_uppercase(), word.to_uppercase()));
				if cases.len() >= TARGET {
					break;
				}
			}
		}
	}

	cases.sort_by(|a, b| a.0.cmp(&b.0));
	let mut out = String::new();
	for (typed, expected) in &cases {
		writeln!(out, "{typed}\t{expected}").unwrap();
	}
	fs::create_dir_all("tests/data").unwrap();
	fs::write("tests/data/all_caps_cases.tsv", out).unwrap();
	println!("wrote {} cases", cases.len());
}
