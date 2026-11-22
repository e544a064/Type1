//! Dev-only evaluation harness for the corrector.
//!
//! Generates synthetic typos from the shipped dictionary and reports top-1
//! accuracy and how often random strings would be "corrected".
//!
//!     cargo run --release --example evaluate

use type1_core::correction::{self, MIN_SCORE};
use type1_core::dictionary;
use type1_core::layout::LETTERS;

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

fn neighbour(c: char, rng: &mut Rng) -> char {
	let Some(index) = LETTERS.iter().position(|letter| letter.id.starts_with(c)) else {
		return c;
	};
	let offset = if rng.below(2) == 0 { 1 } else { usize::MAX };
	let next = (index.wrapping_add(offset)).min(LETTERS.len() - 1);
	LETTERS[next].id.chars().next().unwrap_or(c)
}

fn typo(word: &str, rng: &mut Rng) -> String {
	let mut chars: Vec<char> = word.chars().collect();
	match rng.below(5) {
		0 => {
			let i = rng.below(chars.len());
			chars[i] = neighbour(chars[i], rng);
		}
		1 => {
			let i = rng.below(chars.len());
			chars.remove(i);
		}
		2 => {
			let i = rng.below(chars.len());
			chars.insert(i, neighbour(chars[i], rng));
		}
		3 => {
			let i = rng.below(chars.len() - 1);
			chars.swap(i, i + 1);
		}
		_ => return type1_core::lexicon::fold(word),
	}
	chars.into_iter().collect()
}

fn main() {
	let words: Vec<&'static str> = dictionary::words()
	.iter()
	.filter(|word| {
		let len = word.text.chars().count();
		word.log_freq > 3.0 && (4..=9).contains(&len) && !word.text.contains('\'')
	})
	.map(|word| word.text)
	.collect();

	let debug = std::env::args().any(|arg| arg == "--debug");
	let mut rng = Rng(0x1234_5678_9abc_def0);
	let mut tested = 0usize;
	let mut top1 = 0usize;
	let mut clitic_tested = 0usize;
	let mut clitic_top1 = 0usize;
	let mut failures = Vec::new();
	for word in words.iter().step_by(7).take(400) {
		let generated = typo(word, &mut rng);
		if generated == *word {
			continue;
		}
		tested += 1;
		let first = correction::candidates(&generated)
		.first()
		.map(|candidate| candidate.text.clone());
		let clitic = matches!(
			word.chars().next(),
			Some('c' | 'd' | 'j' | 'l' | 'm' | 'n' | 'q' | 's' | 't')
		);
		if clitic {
			clitic_tested += 1;
		}
		if first.as_deref() == Some(word) {
			top1 += 1;
			if clitic {
				clitic_top1 += 1;
			}
		} else if debug {
			failures.push(format!(
				"{generated:>10} -> {:<10} (want {word})",
				first.unwrap_or_default()
			));
		}
	}

	let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
	let mut random_hits = 0usize;
	let random_samples = 1000usize;
	for _ in 0..random_samples {
		let len = 5 + rng.below(4);
		let text: String = (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect();
		if correction::candidates(&text)
		.first()
		.map(|candidate| candidate.score >= MIN_SCORE)
		.unwrap_or(false)
		{
			random_hits += 1;
		}
	}

	println!("dictionary words: {}", dictionary::words().len());
	println!("synthetic typos: {}", tested);
	println!(
		"top-1 accuracy: {} ({:.1}%)",
		top1,
		top1 as f64 * 100.0 / tested.max(1) as f64
	);
	println!(
		"clitic-initial top-1: {clitic_top1} / {clitic_tested}"
	);
	println!(
		"random strings auto-corrected: {} / {} ({:.1}%)",
		random_hits,
		random_samples,
		random_hits as f64 * 100.0 / random_samples as f64
	);
	if debug {
		for failure in failures.iter().take(40) {
			println!("  {failure}");
		}
	}
}
