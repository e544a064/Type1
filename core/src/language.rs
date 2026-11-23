//! Languages exposed by keyboard profiles.
//!
//! A profile can combine several languages. Each language owns its dictionary
//! and the correction rules that are specific to it.

use crate::dictionary::{Dictionary, FRENCH as FRENCH_DICTIONARY};
use crate::json;

#[derive(Debug)]
pub struct Language {
	pub id: &'static str,
	pub label: &'static str,
	pub dictionary: &'static Dictionary,
	/// French elision/clitic handling (`jai` -> `j'ai`).
	pub elisions: bool,
}

pub static FRENCH: Language = Language {
	id: "fr",
	label: "Français",
	dictionary: &FRENCH_DICTIONARY,
	elisions: true,
};

/// Languages with a correction model today. English and Korean can be added
/// here once their dictionaries and rules exist.
pub static LANGUAGES: &[&Language] = &[&FRENCH];

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
