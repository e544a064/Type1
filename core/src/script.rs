//! Writing systems that need more than appending a character.
//!
//! Latin scripts commit each key as-is. Hangul composes jamo into syllables,
//! so the engine asks the script before it edits the current word.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
	Latin,
	Hangul,
}
