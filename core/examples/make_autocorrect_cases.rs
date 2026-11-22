//! Generates `core/tests/data/autocorrect_cases.tsv`.
//!
//! The cases are deterministic typo variants of dictionary words that the
//! current corrector resolves to the intended word. They form a bulk
//! regression suite: rerun this example after tuning the corrector and commit
//! the new fixture.
//!
//!     cargo run --release --example make_autocorrect_cases

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;

use type1_core::correction::suggest;
use type1_core::dictionary;
use type1_core::layout::LETTERS;

const TARGET: usize = 2_500;

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

fn spelling(word: &str, rng: &mut Rng) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	if chars.len() < 4 {
		return word.to_string();
	}
	let i = rng.below(chars.len());
	let vowels = ['a', 'e', 'i', 'o', 'u'];
	let mut replacement = vowels[rng.below(vowels.len())];
	if replacement == chars[i] {
		replacement = 'y';
	}
	chars[i] = replacement;
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

	let mut seen = HashSet::new();
	let mut cases: Vec<(String, &'static str)> = Vec::new();
	let mut index = 0usize;

	while cases.len() < TARGET {
		let word = pool[index % pool.len()];
		index += 1;
		if index > pool.len() * 40 {
			break;
		}
		let variants = [
			substitute(word, 1, &mut rng, &order),
			substitute_far(word, 1, &mut rng, &order),
			spelling(word, &mut rng),
			substitute(word, 2, &mut rng, &order),
			substitute(word, 3, &mut rng, &order),
			transpose(word, &mut rng),
			remove(word, &mut rng),
			insert(word, &mut rng, &order),
			accentless(word),
		];
		for typed in variants {
			if typed == word || typed.is_empty() {
				continue;
			}
			if !seen.insert(typed.clone()) {
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
				cases.push((typed, word));
				if cases.len() >= TARGET {
					break;
				}
			}
		}
	}

	cases.sort_by(|a, b| a.0.cmp(&b.0));
	let mut out = String::new();
	for (typed, word) in &cases {
		writeln!(out, "{typed}\t{word}").unwrap();
	}
	let path = "tests/data/autocorrect_cases.tsv";
	fs::create_dir_all("tests/data").unwrap();
	fs::write(path, out).unwrap();
	println!("wrote {} cases to {path}", cases.len());
}
