//! Keyboard profiles: a layout plus one or more languages.
//!
//! The setup screen creates profiles from the separately available layouts and
//! languages, rather than choosing from a precomputed list of combinations.

use std::sync::OnceLock;

use crate::correction::{Corrector, Suggestion};
use crate::language::{self, Language};
use crate::layout::{self, Layout};
use crate::personal::PersonalLexicon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingRules {
	French,
	/// Latin languages without French spaces before `! ? : ;`.
	Latin,
}

impl SpacingRules {
	pub fn space_before(self, symbol: &str) -> bool {
		matches!(self, Self::French) && matches!(symbol, "!" | "?" | ":" | ";")
	}
}

#[derive(Debug)]
pub struct Profile {
	pub layout: &'static Layout,
	pub languages: Vec<&'static Language>,
	pub spacing: SpacingRules,
	corrector: OnceLock<Corrector>,
}

impl Profile {
	pub fn new(layout: &'static Layout, languages: Vec<&'static Language>) -> Self {
		let spacing = if languages.iter().any(|language| language.id == "fr") {
			SpacingRules::French
		} else {
			SpacingRules::Latin
		};
		Profile {
			layout,
			languages,
			spacing,
			corrector: OnceLock::new(),
		}
	}

	pub fn from_ids(layout_id: &str, language_ids: &[&str]) -> Option<Self> {
		let layout = layout::find(layout_id)?;
		let mut languages = Vec::with_capacity(language_ids.len());
		for id in language_ids {
			let language = language::find(id)?;
			if !languages
			. iter()
			. any(|existing: &&Language| std::ptr::eq(*existing, language))
			{
				languages.push(language);
			}
		}
		if languages.is_empty() {
			return None;
		}
		Some(Self::new(layout, languages))
	}

	pub fn default_profile() -> Self {
		Self::new(&layout::AZERTY, vec![&language::FRENCH])
	}

	pub fn same_as(&self, other: &Profile) -> bool {
		std::ptr::eq(self.layout, other.layout)
			&& self.languages.len() == other.languages.len()
			&& self
			. languages
			. iter()
			. zip(&other.languages)
			. all(|(a, b)| std::ptr::eq(*a, *b))
	}

	pub(crate) fn corrector(&self) -> &Corrector {
		self.corrector.get_or_init(|| Corrector::new(self))
	}

	pub fn suggest(&self, typed: &str) -> Suggestion {
		self.corrector().suggest(typed)
	}

	pub fn suggest_with_personal(
		&self,
		typed: &str,
		personal: &PersonalLexicon,
	) -> Suggestion {
		self.corrector().suggest_with_personal(typed, personal)
	}
}

impl Default for Profile {
	fn default() -> Self {
		Self::default_profile()
	}
}

pub fn default_profile() -> &'static Profile {
	static DEFAULT: OnceLock<Profile> = OnceLock::new();
	DEFAULT.get_or_init(Profile::default_profile)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_profile_uses_azerty_and_french() {
		let profile = Profile::default_profile();
		assert_eq!(profile.layout.id, "azerty");
		assert_eq!(profile.languages.len(), 1);
		assert_eq!(profile.languages[0].id, "fr");
		assert_eq!(profile.spacing, SpacingRules::French);
	}

	#[test]
	fn profiles_are_built_from_ids() {
		let profile = Profile::from_ids("azerty", &["fr"]).unwrap();
		assert_eq!(profile.layout.id, "azerty");
		assert_eq!(profile.languages[0].id, "fr");
		assert!(Profile::from_ids("missing", &["fr"]).is_none());
		assert!(Profile::from_ids("azerty", &["missing"]).is_none());
		assert!(Profile::from_ids("azerty", &[]).is_none());
	}

	#[test]
	fn french_profile_uses_french_spacing() {
		let profile = Profile::new(&layout::AZERTY, vec![&language::FRENCH]);
		assert_eq!(profile.spacing, SpacingRules::French);
	}

	#[test]
	fn english_profile_uses_latin_spacing() {
		let profile = Profile::new(&layout::AZERTY, vec![&language::ENGLISH]);
		assert_eq!(profile.spacing, SpacingRules::Latin);
	}
}
