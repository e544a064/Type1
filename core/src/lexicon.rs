//! Accent folding shared by the dictionary and the corrector.

/// Accent-insensitive, lowercase fold: `École` and `ecole` both become `ecole`.
pub fn fold(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	for c in input.chars() {
		for lower in c.to_lowercase() {
			match lower {
				'œ' => out.push_str("oe"),
				'æ' => out.push_str("ae"),
				'à' | 'â' | 'ä' | 'á' => out.push('a'),
				'é' | 'è' | 'ê' | 'ë' => out.push('e'),
				'î' | 'ï' | 'í' => out.push('i'),
				'ô' | 'ö' | 'ó' => out.push('o'),
				'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
				'ç' => out.push('c'),
				'ÿ' => out.push('y'),
				'ñ' => out.push('n'),
				other => out.push(other),
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn folds_accents_and_case() {
		assert_eq!(fold("École"), "ecole");
		assert_eq!(fold("où"), "ou");
		assert_eq!(fold("œuf"), "oeuf");
	}
}
