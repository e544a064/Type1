//! Type1 keyboard core.
//!
//! The core owns everything that is not platform specific: key layout, pages,
//! shift and capitalisation, French spacing rules. The Android layer only draws
//! what [`Engine::render_json`] returns and applies the [`Action`]s it gets back.
//!
//! Porting to another platform means writing a new UI + FFI shim; this crate
//! stays untouched.

pub mod action;
pub mod correction;
pub mod dictionary;
pub mod engine;
pub mod json;
pub mod language;
pub mod layout;
pub mod lexicon;
pub mod profile;

mod ffi;

pub use action::{Action, ImeActionKind};
pub use engine::{CapsMode, Engine};
pub use language::Language;
pub use layout::{Layout, Page, ShiftState};
pub use profile::{Profile, SpacingRules};
