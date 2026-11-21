//! Tiny JSON writer. No dependencies: the payloads are small and fully
//! controlled by the core, so a full serializer would be overkill.

/// Append `value` as a quoted, escaped JSON string.
pub fn string_into(out: &mut String, value: &str) {
	out.push('"');
	for c in value.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if (c as u32) < 0x20 => {
				out.push_str(&format!("\\u{:04x}", c as u32));
			}
			c => out.push(c),
		}
	}
	out.push('"');
}

/// Append a float with a stable two-decimal representation.
pub fn float_into(out: &mut String, value: f32) {
	out.push_str(&format!("{value:.2}"));
}

pub fn bool_into(out: &mut String, value: bool) {
	out.push_str(if value { "true" } else { "false" });
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn escapes_quotes_and_controls() {
		let mut s = String::new();
		string_into(&mut s, "a\"b\\c\nd");
		assert_eq!(s, "\"a\\\"b\\\\c\\nd\"");
	}

	#[test]
	fn keeps_unicode() {
		let mut s = String::new();
		string_into(&mut s, "éàç");
		assert_eq!(s, "\"éàç\"");
	}
}
