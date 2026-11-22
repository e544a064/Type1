//! Actions emitted by the engine and consumed by the platform layer.

use crate::json;

/// Editor action requested by the enter key when the field is single-line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeActionKind {
	None,
	Go,
	Search,
	Send,
	Next,
	Done,
	Previous,
}

impl ImeActionKind {
	/// Map `EditorInfo.imeOptions & IME_MASK_ACTION` to a kind.
	pub fn from_ime_options(ime_options: i32) -> Self {
		match ime_options & 0x0000_00ff {
			0x0000_0002 => Self::Go,
			0x0000_0003 => Self::Search,
			0x0000_0004 => Self::Send,
			0x0000_0005 => Self::Next,
			0x0000_0006 => Self::Done,
			0x0000_0007 => Self::Previous,
			_ => Self::None,
		}
	}

	pub fn id(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Go => "go",
			Self::Search => "search",
			Self::Send => "send",
			Self::Next => "next",
			Self::Done => "done",
			Self::Previous => "previous",
		}
	}

	/// Glyph drawn on the enter key.
	pub fn key_label(self) -> &'static str {
		match self {
			Self::None => "⏎",
			Self::Go | Self::Next => "→",
			Self::Search => "🔍",
			Self::Send => "➤",
			Self::Done => "✓",
			Self::Previous => "←",
		}
	}
}

/// Something the platform layer must do with the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
	/// Insert text at the cursor.
	Commit(String),
	/// Insert a suggestion plus its convenience space. The platform removes any
	/// spaces that already followed the old word, so no duplicate is left.
	CommitSuggestion(String),
	/// Delete one character, or one word when `word` is true.
	Delete { word: bool },
	/// Delete an exact number of characters (used when replacing a suggestion).
	DeleteChars(u32),
	/// Replace the word around the caret, with `before`/`after` characters on
	/// each side (used when re-correcting existing text).
	ReplaceSurrounding { before: u32, after: u32, text: String },
	/// Move the caret by `delta` characters (negative is left).
	MoveCursor(i32),
	/// Start a zero-width selection at the caret.
	StartSelection,
	/// Extend the selection by `delta` characters (negative is left).
	ExtendSelection(i32),
	/// Delete the current selection.
	DeleteSelection,
	/// Trigger an editor action (single-line fields only).
	Ime(ImeActionKind),
}

impl Action {
	pub fn list_json(actions: &[Action]) -> String {
		let mut out = String::from("[");
		for (i, action) in actions.iter().enumerate() {
			if i > 0 {
				out.push(',');
			}
			match action {
				Action::Commit(text) => {
					out.push_str("{\"t\":\"commit\",\"v\":");
					json::string_into(&mut out, text);
					out.push('}');
				}
				Action::CommitSuggestion(text) => {
					out.push_str("{\"t\":\"suggest\",\"v\":");
					json::string_into(&mut out, text);
					out.push('}');
				}
				Action::Delete { word } => {
					out.push_str("{\"t\":\"delete\",\"word\":");
					json::bool_into(&mut out, *word);
					out.push('}');
				}
				Action::DeleteChars(count) => {
					out.push_str("{\"t\":\"delete\",\"n\":");
					out.push_str(&count.to_string());
					out.push('}');
				}
				Action::ReplaceSurrounding {
					before,
					after,
					text,
				} => {
					out.push_str("{\"t\":\"replace\",\"b\":");
					out.push_str(&before.to_string());
					out.push_str(",\"a\":");
					out.push_str(&after.to_string());
					out.push_str(",\"v\":");
					json::string_into(&mut out, text);
					out.push('}');
				}
				Action::MoveCursor(delta) => {
					out.push_str("{\"t\":\"cursor\",\"v\":");
					out.push_str(&delta.to_string());
					out.push('}');
				}
				Action::ExtendSelection(delta) => {
					out.push_str("{\"t\":\"select\",\"v\":");
					out.push_str(&delta.to_string());
					out.push('}');
				}
				Action::StartSelection => {
					out.push_str("{\"t\":\"startSelection\"}");
				}
				Action::DeleteSelection => {
					out.push_str("{\"t\":\"deleteSelection\"}");
				}
				Action::Ime(kind) => {
					out.push_str("{\"t\":\"ime\",\"v\":");
					json::string_into(&mut out, kind.id());
					out.push('}');
				}
			}
		}
		out.push(']');
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ime_action_mapping() {
		assert_eq!(ImeActionKind::from_ime_options(0), ImeActionKind::None);
		assert_eq!(ImeActionKind::from_ime_options(6), ImeActionKind::Done);
		assert_eq!(
			ImeActionKind::from_ime_options(0x0000_0006 | 0x0000_0100),
			ImeActionKind::Done
		);
	}

	#[test]
	fn serialises_actions() {
		let json = Action::list_json(&[
			Action::Commit("é".into()),
			Action::CommitSuggestion("mot ".into()),
			Action::Delete { word: true },
			Action::ReplaceSurrounding {
				before: 2,
				after: 3,
				text: "test".into(),
			},
			Action::MoveCursor(-2),
			Action::StartSelection,
			Action::ExtendSelection(-1),
			Action::DeleteSelection,
			Action::Ime(ImeActionKind::Send),
		]);
		assert_eq!(
			json,
			r#"[{"t":"commit","v":"é"},{"t":"suggest","v":"mot "},{"t":"delete","word":true},{"t":"replace","b":2,"a":3,"v":"test"},{"t":"cursor","v":-2},{"t":"startSelection"},{"t":"select","v":-1},{"t":"deleteSelection"},{"t":"ime","v":"send"}]"#
		);
	}
}
