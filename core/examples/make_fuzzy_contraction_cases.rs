//! Generates the fuzzy-contraction bulk fixture in `core/tests/data/`.
//!
//! Each case is a key-slip variant of a contraction key that the English
//! corrector auto-corrects back to the contraction.
//!
//!     cargo run --release --example make_fuzzy_contraction_cases

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;

use type1_core::language::{ENGLISH as ENGLISH_LANGUAGE, ENGLISH_CONTRACTIONS};
use type1_core::layout::{Layout, AZERTY, QWERTY};
use type1_core::Profile;

fn neighbours(letter: char, layout: &Layout) -> Vec<char> {
	let Some(index) = layout
	. letters
	. iter()
	. position(|entry| entry.id.starts_with(letter))
	else {
		return Vec::new();
	};
	let mut out = Vec::new();
	for offset in [2usize, 1] {
		if let Some(entry) = index.checked_sub(offset).and_then(|i| layout.letters.get(i)) {
			if let Some(c) = entry.id.chars().next() {
				out.push(c);
			}
		}
	}
	for offset in [1usize, 2] {
		if let Some(entry) = layout.letters.get(index + offset) {
			if let Some(c) = entry.id.chars().next() {
				out.push(c);
			}
		}
	}
	out
}

fn variants(key: &str, layout: &Layout) -> Vec<String> {
	let chars: Vec<char> = key.chars().collect();
	let mut out = Vec::new();
	for position in 0..chars.len() {
		for neighbour in neighbours(chars[position], layout) {
			if neighbour == chars[position] {
				continue;
			}
			let mut variant = chars.clone();
			variant[position] = neighbour;
			out.push(variant.into_iter().collect());
		}
	}
	for position in 0..chars.len().saturating_sub(1) {
		let mut variant = chars.clone();
		variant.swap(position, position + 1);
		out.push(variant.into_iter().collect());
	}
	let first_neighbour: Vec<Option<char>> = chars
	. iter()
	. map(|&c| neighbours(c, layout).first().copied())
	. collect();
	for a in 0..chars.len() {
		for b in (a + 1)..chars.len() {
			if let (Some(left), Some(right)) = (first_neighbour[a], first_neighbour[b]) {
				let mut variant = chars.clone();
				variant[a] = left;
				variant[b] = right;
				out.push(variant.into_iter().collect());
			}
		}
	}
	if chars.len() > 4 {
		for position in 0..chars.len() {
			let mut variant = chars.clone();
			variant.remove(position);
			out.push(variant.into_iter().collect());
		}
	}
	for position in 0..chars.len() {
		for neighbour in neighbours(chars[position], layout) {
			let mut variant = chars.clone();
			variant.insert(position, neighbour);
			out.push(variant.into_iter().collect());
		}
	}
	out
}

fn main() {
	let mut rows: Vec<(String, String, String)> = Vec::new();
	let mut seen: HashSet<(String, String)> = HashSet::new();
	for layout in [&AZERTY, &QWERTY] {
		let profile = Profile::new(layout, vec![&ENGLISH_LANGUAGE]);
		for &(key, contraction) in ENGLISH_CONTRACTIONS {
			if key.chars().count() < 4 {
				continue;
			}
			for variant in variants(key, layout) {
				if variant == key || !seen.insert((layout.id.to_string(), variant.clone())) {
					continue;
				}
				let suggestion = profile.suggest(&variant);
				let best = suggestion
				. candidates
				. first()
				.map(|candidate| candidate.text.as_str());
				if suggestion.apply && best == Some(contraction) {
					rows.push((layout.id.to_string(), variant, contraction.to_string()));
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
	fs::write("tests/data/fuzzy_contraction_cases.tsv", out).unwrap();
	println!("wrote {} fuzzy contraction cases", rows.len());
}
