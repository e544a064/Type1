//! Generates a wider fuzzy-contraction fixture: every single-letter
//! substitution at every position for every contraction key.
//!
//!     cargo run --release --example make_fuzzy_contraction_extra_cases

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;

use type1_core::language::{ENGLISH as ENGLISH_LANGUAGE, ENGLISH_CONTRACTIONS};
use type1_core::layout::{AZERTY, QWERTY};
use type1_core::Profile;

const PER_LAYOUT: usize = 1_500;

fn main() {
	let mut rows: Vec<(String, String, String)> = Vec::new();
	let mut seen: HashSet<(String, String)> = HashSet::new();
	for layout in [&AZERTY, &QWERTY] {
		let profile = Profile::new(layout, vec![&ENGLISH_LANGUAGE]);
		let mut count = 0usize;
		'layout: for &(key, contraction) in ENGLISH_CONTRACTIONS {
			let chars: Vec<char> = key.chars().collect();
			if chars.len() < 4 {
				continue;
			}
			for position in 0..chars.len() {
				for byte in b'a'..=b'z' {
					let letter = byte as char;
					if letter == chars[position] {
						continue;
					}
					let mut variant = chars.clone();
					variant[position] = letter;
					let variant: String = variant.into_iter().collect();
					if !seen.insert((layout.id.to_string(), variant.clone())) {
						continue;
					}
					let suggestion = profile.suggest(&variant);
					let best = (
						suggestion
						. candidates
						. first()
						. map(|candidate| candidate.text.as_str())
					);
					if suggestion.apply && best == Some(contraction) {
						rows.push((
							layout.id.to_string(),
							variant,
							contraction.to_string(),
						));
						count += 1;
						if count >= PER_LAYOUT {
							break 'layout;
						}
					}
				}
			}
		}
	}

	rows.sort();
	let mut out = String::new();
	for (layout, typed, expected) in &rows {
		writeln!(out, "{layout}\t{typed}\t{expected}").unwrap();
	}
	fs::create_dir_all("tests/data").unwrap();
	fs::write("tests/data/fuzzy_contraction_extra_cases.tsv", out).unwrap();
	println!("wrote {} extra fuzzy contraction cases", rows.len());
}
