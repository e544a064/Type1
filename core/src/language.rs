//! Languages exposed by keyboard profiles.
//!
//! A profile can combine several languages. Each language owns its dictionary
//! and the correction rules that are specific to it.

use crate::dictionary::{
	Dictionary, ENGLISH as ENGLISH_DICTIONARY, FRENCH as FRENCH_DICTIONARY,
};
use crate::json;

#[derive(Debug)]
pub struct Language {
	pub id: &'static str,
	pub label: &'static str,
	/// Short code shown on the spacebar (`FR`, `EN`, ...).
	pub short: &'static str,
	pub dictionary: &'static Dictionary,
	/// French elision/clitic handling (`jai` -> `j'ai`).
	pub elisions: bool,
}

pub static FRENCH: Language = Language {
	id: "fr",
	label: "Français",
	short: "FR",
	dictionary: &FRENCH_DICTIONARY,
	elisions: true,
};

pub static ENGLISH: Language = Language {
	id: "en",
	label: "English (UK)",
	short: "EN",
	dictionary: &ENGLISH_DICTIONARY,
	elisions: false,
};

/// Languages with a correction model today. Korean can be added here once its
/// dictionary and Hangul composition exist.
pub static LANGUAGES: &[&Language] = &[&FRENCH, &ENGLISH];

pub fn find(id: &str) -> Option<&'static Language> {
	LANGUAGES.iter().copied().find(|language| language.id == id)
}

pub fn catalog_json() -> String {
	let mut out = String::from("[");
	for (index, language) in LANGUAGES.iter().enumerate() {
		if index > 0 {
			out.push(',');
		}
		out.push_str("{\"id\":");
		json::string_into(&mut out, language.id);
		out.push_str(",\"label\":");
		json::string_into(&mut out, language.label);
		out.push('}');
	}
	out.push(']');
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn english_is_available() {
		let english = find("en").unwrap();
		assert_eq!(english.label, "English (UK)");
		assert_eq!(english.short, "EN");
		assert!(!english.elisions);
		assert!(english.dictionary.words().len() >= 15_000);
	}

	#[test]
	fn catalog_lists_both_languages() {
		let json: serde_json::Value = serde_json::from_str(&catalog_json()).unwrap();
		assert_eq!(json[0]["id"], "fr");
		assert_eq!(json[0]["label"], "Français");
		assert_eq!(json[1]["id"], "en");
		assert_eq!(json[1]["label"], "English (UK)");
	}
}
