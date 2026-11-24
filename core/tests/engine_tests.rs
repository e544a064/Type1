use type1_core::language::{ENGLISH, FRENCH};
use type1_core::layout;
use type1_core::{Action, Engine, ImeActionKind, Page, Profile};

const TEXT: i32 = 0x0000_0001;
const CAP_SENTENCES: i32 = 0x0000_4000;

fn plain() -> Engine {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT, 0);
	engine
}

fn commits(actions: &[Action]) -> Vec<String> {
	actions
	. iter()
	. filter_map(|action| match action {
		Action::Commit(text) => Some(text.clone()),
		_ => None,
	})
	. collect()
}

fn space_label(engine: &mut Engine) -> String {
	let json: serde_json::Value =
		serde_json::from_str(&engine.render_json()).unwrap();
	json["keys"]
	. as_array()
	. unwrap()
	. iter()
	. find(|key| key["id"] == "space")
	. unwrap()["label"]
	. as_str()
	. unwrap()
	. to_string()
}

fn capitalize(word: &str) -> String {
	let mut chars = word.chars();
	match chars.next() {
		Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
		None => String::new(),
	}
}

/// What the strip should show when the caret re-enters a changed word.
fn original_suggestions(
	typed: &str,
	visible: &str,
	caps: bool,
	limit: usize,
) -> Vec<String> {
	let visible_key = visible.to_lowercase();
	let typed_key = typed.to_lowercase();
	(
		type1_core::correction::suggest(typed)
		. candidates
		. iter()
		. map(|candidate| {
			if caps {
				capitalize(&candidate.text)
			} else {
				candidate.text.clone()
			}
		})
		. filter(|text| {
			let key = text.to_lowercase();
			key != visible_key && key != typed_key
		})
		. take(limit)
		. collect()
	)
}

fn suggestion_texts(json: &serde_json::Value) -> Vec<String> {
	(
		json["suggestions"]
		. as_array()
		. unwrap()
		. iter()
		. map(|value| value.as_str().unwrap().to_string())
		. collect()
	)
}

#[test]
fn shift_applies_once() {
	let mut engine = plain();
	assert_eq!(engine.on_key("shift"), vec![]);
	assert_eq!(commits(&engine.on_key("a")), vec!["A"]);
	assert_eq!(commits(&engine.on_key("a")), vec!["a"]);
}

#[test]
fn shift_locks_after_second_tap() {
	let mut engine = plain();
	engine.on_key("shift");
	engine.on_key("shift");
	assert_eq!(commits(&engine.on_key("z")), vec!["Z"]);
	assert_eq!(commits(&engine.on_key("e")), vec!["E"]);
	engine.on_key("shift");
	assert_eq!(commits(&engine.on_key("r")), vec!["r"]);
}

#[test]
fn shift_resets_when_changing_page() {
	let mut engine = plain();
	engine.on_key("shift");
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Numbers);
	engine.on_key("page");
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Letters);
	assert_eq!(commits(&engine.on_key("a")), vec!["a"]);
}

#[test]
fn profiles_can_be_selected_from_layout_and_languages() {
	let mut engine = Engine::new();
	assert_eq!(engine.profile().layout.id, "azerty");
	assert_eq!(engine.profile().languages[0].id, "fr");
	assert!(engine.set_profile_ids("azerty", &["fr"]));
	assert!(!engine.set_profile_ids("azerty", &["missing"]));
	assert!(!engine.set_profile_ids("missing", &["fr"]));
	assert_eq!(engine.profile().layout.id, "azerty");
	assert!(engine.set_profile_ids("qwerty", &["fr"]));
	assert_eq!(engine.profile().layout.id, "qwerty");
}

#[test]
fn spacebar_shows_the_active_language_codes() {
	let mut engine = Engine::new();
	assert_eq!(space_label(&mut engine), "FR");
	engine.set_profile(Profile::new(&layout::AZERTY, vec![&FRENCH, &ENGLISH]));
	assert_eq!(space_label(&mut engine), "FR-EN");
}

#[test]
fn corrects_english_typos_on_qwerty() {
	let mut engine = plain();
	assert!(engine.set_profile_ids("qwerty", &["en"]));
	for key in ["l", "a", "n", "g", "a", "u", "g", "e"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(8),
			Action::Commit("language".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn the_builtin_profile_can_be_restored() {
	let mut engine = Engine::new();
	assert!(engine.set_profile_ids("azerty", &["fr"]));
	engine.set_default_profile();
	assert_eq!(engine.profile().layout.id, "azerty");
	assert_eq!(engine.profile().languages[0].id, "fr");
}

#[test]
fn auto_capitalises_sentences() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	assert_eq!(commits(&engine.on_key("l")), vec!["L"]);
	assert_eq!(commits(&engine.on_key("a")), vec!["a"]);
	assert_eq!(commits(&engine.on_key("space")), vec![" "]);
	assert_eq!(commits(&engine.on_key("e")), vec!["e"]);
	engine.on_key("page");
	assert_eq!(commits(&engine.on_key("dot")), vec!["."]);
	engine.on_key("page");
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Letters);
	assert_eq!(commits(&engine.on_key("z")), vec![" ", "Z"]);
}

#[test]
fn context_sync_restores_sentence_capitalisation() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	// The caret is after the space that follows a sentence.
	engine.set_context_word("Bonjour. ", "");
	assert_eq!(commits(&engine.on_key("v")), vec!["V"]);
}

#[test]
fn context_sync_keeps_mid_sentence_text_lowercase() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	engine.set_context_word("Bonjour ", "");
	assert_eq!(commits(&engine.on_key("v")), vec!["v"]);
}

#[test]
fn context_sync_adds_the_space_after_a_period() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	engine.set_context_word("Bonjour.", "");
	assert_eq!(
		engine.on_key("v"),
		vec![Action::Commit(" ".into()), Action::Commit("V".into())]
	);
}

#[test]
fn backspacing_the_space_after_a_period_keeps_auto_capitalisation() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	for key in ["b", "o", "n", "j", "o", "u", "r"] {
		engine.on_key(key);
	}
	engine.on_key("dot");
	engine.on_key("space");
	engine.on_key("backspace");
	// The IME re-reads the editor text after the delete.
	engine.set_context_word("Bonjour.", "");
	assert_eq!(commits(&engine.on_key("v")), vec![" ", "V"]);
}

#[test]
fn context_sync_capitalises_after_a_newline() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	engine.set_context_word("Bonjour.\n", "");
	assert_eq!(commits(&engine.on_key("v")), vec!["V"]);
}

#[test]
fn context_sync_capitalises_behind_a_closing_quote() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	engine.set_context_word("« Bonjour. » ", "");
	assert_eq!(commits(&engine.on_key("v")), vec!["V"]);
}

#[test]
fn auto_spaces_after_punctuation() {
	let mut engine = plain();
	engine.on_key("a");
	engine.on_key("page");
	engine.on_key("dot");
	engine.on_key("page");
	engine.on_key("page");
	let actions = engine.on_key("z");
	assert_eq!(commits(&actions), vec![" ", "z"]);
}

#[test]
fn french_space_before_high_punctuation() {
	let mut engine = plain();
	engine.on_key("a");
	engine.on_key("page");
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Symbols);
	assert_eq!(commits(&engine.on_key("excl")), vec![" ", "!"]);
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Letters);
	assert_eq!(commits(&engine.on_key("z")), vec![" ", "z"]);
}

#[test]
fn no_space_before_punctuation_at_start() {
	let mut engine = plain();
	engine.on_key("page");
	engine.on_key("page");
	assert_eq!(commits(&engine.on_key("question")), vec!["?"]);
}

#[test]
fn page_key_cycles() {
	let mut engine = plain();
	assert_eq!(engine.page(), Page::Letters);
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Numbers);
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Symbols);
	engine.on_key("page");
	assert_eq!(engine.page(), Page::Letters);
}

#[test]
fn numeric_fields_start_on_digits() {
	let mut engine = Engine::new();
	engine.set_input_type(0x0000_0002, 0);
	assert_eq!(engine.page(), Page::Numbers);
}

#[test]
fn backspace_emits_delete() {
	let mut engine = plain();
	assert_eq!(
		engine.on_key("backspace"),
		vec![Action::Delete { word: false }]
	);
}

#[test]
fn cursor_moves_are_reported() {
	let mut engine = plain();
	assert_eq!(
		engine.on_key("cursor_left"),
		vec![Action::MoveCursor(-1)]
	);
	assert_eq!(
		engine.on_key("cursor_right"),
		vec![Action::MoveCursor(1)]
	);
}

#[test]
fn cursor_moves_clear_the_word() {
	let mut engine = plain();
	engine.on_key("e");
	engine.on_key("c");
	engine.on_key("cursor_left");
	let json: serde_json::Value =
		serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn selection_actions_are_reported() {
	let mut engine = plain();
	assert_eq!(
		engine.on_key("start_selection"),
		vec![Action::StartSelection]
	);
	assert_eq!(
		engine.on_key("select_left"),
		vec![Action::ExtendSelection(-1)]
	);
	assert_eq!(
		engine.on_key("select_right"),
		vec![Action::ExtendSelection(1)]
	);
	assert_eq!(
		engine.on_key("delete_selection"),
		vec![Action::DeleteSelection]
	);
}

#[test]
fn enter_commits_newline_by_default() {
	let mut engine = plain();
	assert_eq!(engine.on_key("enter"), vec![Action::Commit("\n".into())]);
}

#[test]
fn enter_triggers_editor_action_when_single_line() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT, 0x0000_0006);
	assert_eq!(engine.on_key("enter"), vec![Action::Ime(ImeActionKind::Done)]);
}

#[test]
fn typing_after_backspacing_inside_a_word_uses_the_suffix() {
	let mut engine = plain();
	// The caret sits inside the typo "bknj|pur".
	engine.set_context_word("bknj", "pur");
	assert_eq!(
		engine.on_key("backspace"),
		vec![Action::Delete { word: false }]
	);
	// The IME re-reads the text after the delete: "bkn|pur".
	engine.set_context_word("bkn", "pur");
	engine.on_key("j");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = suggestion_texts(&json);
	assert!(
		suggestions.iter().any(|text| text == "bonjour"),
		"suggestions: {suggestions:?}"
	);
}

#[test]
fn multiline_field_without_action_keeps_newline() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0);
	assert_eq!(engine.on_key("enter"), vec![Action::Commit("\n".into())]);
}

#[test]
fn multiline_field_performs_an_explicit_action() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0x0000_0006);
	assert_eq!(engine.on_key("enter"), vec![Action::Ime(ImeActionKind::Done)]);
}

#[test]
fn multiline_search_action_shows_the_search_key() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0x0000_0003);
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let enter = (
		json["keys"]
		. as_array()
		. unwrap()
		. iter()
		. find(|key| key["id"] == "enter")
		. unwrap()
	);
	assert_eq!(enter["icon"], "search");
	assert_eq!(enter["label"], "🔍");
}

#[test]
fn multiline_search_action_searches() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0x0000_0003);
	assert_eq!(engine.on_key("enter"), vec![Action::Ime(ImeActionKind::Search)]);
}

#[test]
fn no_enter_action_flag_keeps_the_newline() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0x4000_0006);
	assert_eq!(engine.on_key("enter"), vec![Action::Commit("\n".into())]);
}

#[test]
fn refreshing_ime_options_updates_the_enter_key() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT, 0);
	assert_eq!(engine.on_key("enter"), vec![Action::Commit("\n".into())]);
	engine.set_ime_options(0x0000_0003);
	assert_eq!(engine.on_key("enter"), vec![Action::Ime(ImeActionKind::Search)]);
}

#[test]
fn newline_starts_a_new_sentence() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	assert_eq!(commits(&engine.on_key("a")), vec!["A"]);
	engine.on_key("enter");
	assert_eq!(commits(&engine.on_key("b")), vec!["B"]);
}

#[test]
fn alternates_commit_accented_letters() {
	let mut engine = plain();
	assert_eq!(commits(&engine.on_key("alt:é")), vec!["é"]);
	assert_eq!(commits(&engine.on_key("alt:à")), vec!["à"]);
}

#[test]
fn alternates_follow_shift() {
	let mut engine = plain();
	engine.on_key("shift");
	assert_eq!(commits(&engine.on_key("alt:é")), vec!["É"]);
}

#[test]
fn short_tokens_show_more_suggestions() {
	for (typed, expected) in [("ue", 6), ("pqr", 4), ("ropp", 5), ("commant", 2)] {
		let mut engine = plain();
		for key in typed.chars() {
			engine.on_key(&key.to_string());
		}
		let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
		let count = json["suggestions"].as_array().unwrap().len();
		assert_eq!(count, expected, "typing {typed}");
	}
}

#[test]
fn moving_into_a_corrected_word_reopens_the_original_suggestions() {
	let mut engine = plain();
	engine.on_key("u");
	engine.on_key("e");
	engine.on_key("space");
	// The editor text is "je ". The caret moves back onto the word.
	engine.set_context_word("je", " ");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = suggestion_texts(&json);
	let expected = original_suggestions("ue", "je", false, 6);
	assert!(!expected.is_empty());
	assert_eq!(suggestions, expected);
	assert!(!suggestions.iter().any(|text| text.eq_ignore_ascii_case("ue")));
	assert_eq!(
		engine.on_key("sug:0"),
		vec![Action::ReplaceSurrounding {
			before: 2,
			after: 0,
			text: expected[0].clone(),
		}]
	);
}

#[test]
fn a_tapped_suggestion_is_remembered_too() {
	let mut engine = plain();
	engine.on_key("u");
	engine.on_key("e");
	assert_eq!(
		engine.on_key("sug:0"),
		vec![
			Action::DeleteChars(2),
			Action::CommitSuggestion("je ".into()),
		]
	);
	engine.set_context_word("je", " ");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = suggestion_texts(&json);
	let expected = original_suggestions("ue", "je", false, 6);
	assert!(!expected.is_empty());
	assert_eq!(suggestions, expected);
	assert!(!suggestions.iter().any(|text| text.eq_ignore_ascii_case("ue")));
}

#[test]
fn correction_memory_reopens_suggestions_in_the_visible_case() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	engine.on_key("u");
	engine.on_key("e");
	engine.on_key("space");
	engine.set_context_word("Je", " ");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = suggestion_texts(&json);
	let expected = original_suggestions("ue", "Je", true, 6);
	assert!(!expected.is_empty());
	assert_eq!(suggestions, expected);
	assert!(!suggestions.iter().any(|text| text.eq_ignore_ascii_case("Ue")));
}

#[test]
fn correction_memory_is_not_offered_while_typing() {
	let mut engine = plain();
	for key in ["b", "k", "n", "j", "p", "u", "r"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	for key in ["b", "o", "n", "j", "o", "u", "r"] {
		engine.on_key(key);
	}
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn correction_memory_is_field_scoped() {
	let mut engine = plain();
	for key in ["b", "k", "n", "j", "p", "u", "r"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	// A new field resets the memory.
	engine.set_input_type(TEXT, 0);
	engine.set_context_word("bonjour", " ");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn suggests_corrections_while_typing() {
	let mut engine = plain();
	engine.on_key("t");
	engine.on_key("s");
	engine.on_key("e");
	engine.on_key("t");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = json["suggestions"].as_array().unwrap();
	assert!(!suggestions.is_empty());
	assert!(suggestions.iter().any(|s| s == "test"));
}

#[test]
fn applying_a_suggestion_replaces_the_word() {
	let mut engine = plain();
	engine.on_key("t");
	engine.on_key("s");
	engine.on_key("e");
	engine.on_key("t");
	let actions = engine.on_key("sug:0");
	assert_eq!(
		actions,
		vec![
			Action::DeleteChars(4),
			Action::CommitSuggestion("test ".into())
		]
	);
	// The word buffer is now empty: no suggestions until new letters arrive.
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn suggestions_reset_after_space() {
	let mut engine = plain();
	engine.on_key("t");
	engine.on_key("e");
	engine.on_key("s");
	engine.on_key("t");
	engine.on_key("space");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn corrects_on_space() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(3),
			Action::Commit("j'ai".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn backspace_undoes_the_correction() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	assert_eq!(
		engine.on_key("backspace"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("jai ".into()),
			Action::MoveCursor(-1),
		]
	);
	// The word is open again, so its candidates are back.
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(!json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn context_sync_keeps_the_undo_boundary_and_guard() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.on_key("backspace");
	// The IME re-reads the context after the caret move that follows an undo.
	engine.set_context_word("jai", " ");
	assert_eq!(
		engine.on_key("comma"),
		vec![
			Action::MoveCursor(1),
			Action::DeleteChars(1),
			Action::Commit(",".into()),
		]
	);
}

#[test]
fn context_sync_after_undo_does_not_repeat_the_correction() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.on_key("backspace");
	engine.set_context_word("jai", " ");
	assert_eq!(engine.on_key("space"), vec![Action::MoveCursor(1)]);
}

#[test]
fn space_after_undo_reuses_the_boundary_and_keeps_the_typed_word() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.on_key("backspace");
	// No second space, and no second correction.
	assert_eq!(engine.on_key("space"), vec![Action::MoveCursor(1)]);
}

#[test]
fn backspace_after_a_suggestion_keeps_the_accepted_word() {
	let mut engine = plain();
	for key in ["a", "i", "m", "s"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("sug:0"),
		vec![
			Action::DeleteChars(4),
			Action::CommitSuggestion("amis ".into()),
		]
	);
	assert_eq!(
		engine.on_key("backspace"),
		vec![Action::Delete { word: false }]
	);
	// The IME re-reads the text after the delete.
	engine.set_context_word("amis", "");
	// The accepted suggestion must not turn into "mais" at the comma.
	assert_eq!(engine.on_key("comma"), vec![Action::Commit(",".into())]);
}

#[test]
fn comma_after_undo_replaces_the_kept_space() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.on_key("backspace");
	assert_eq!(
		engine.on_key("comma"),
		vec![
			Action::MoveCursor(1),
			Action::DeleteChars(1),
			Action::Commit(",".into()),
		]
	);
}

#[test]
fn space_after_undo_of_a_comma_keeps_the_comma() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("comma");
	engine.on_key("backspace");
	assert_eq!(
		engine.on_key("space"),
		vec![Action::MoveCursor(1), Action::Commit(" ".into())]
	);
}

#[test]
fn editing_after_undo_still_corrects_at_the_boundary() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.on_key("backspace");
	engine.on_key("m");
	engine.on_key("e");
	// The correction happens before stepping over the kept space.
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("j'aime".into()),
			Action::MoveCursor(1),
		]
	);
}

#[test]
fn suggestions_after_a_correction_offer_the_original() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["suggestions"][0], "jai");
	assert_eq!(
		engine.on_key("sug:0"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("jai ".into()),
		]
	);
}

#[test]
fn correction_preserves_capitalisation() {
	let mut engine = plain();
	engine.on_key("shift");
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	let actions = engine.on_key("space");
	assert_eq!(actions[1], Action::Commit("J'ai".into()));
}

#[test]
fn all_caps_words_are_not_corrected() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0000_1000, 0);
	engine.on_key("j");
	engine.on_key("a");
	engine.on_key("i");
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn password_fields_are_not_corrected() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0000_0080, 0);
	engine.on_key("j");
	engine.on_key("a");
	engine.on_key("i");
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn typed_apostrophe_is_not_doubled() {
	let mut engine = plain();
	engine.on_key("j");
	engine.on_key("page");
	engine.on_key("page");
	engine.on_key("apos");
	engine.on_key("page");
	engine.on_key("a");
	engine.on_key("i");
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn selection_change_cancels_the_undo() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	engine.selection_changed();
	assert_eq!(
		engine.on_key("backspace"),
		vec![Action::Delete { word: false }]
	);
}

#[test]
fn web_email_fields_are_not_corrected() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0000_00d0, 0);
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn name_fields_are_not_rescued() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0000_0060, 0);
	for key in ["g", "e", "q", "n", "v", "x"] {
		engine.on_key(key);
	}
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn corrects_before_punctuation_without_doubling_the_boundary() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("comma"),
		vec![
			Action::DeleteChars(3),
			Action::Commit("j'ai".into()),
			Action::Commit(",".into()),
		]
	);
	assert_eq!(
		engine.on_key("backspace"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("jai,".into()),
			Action::MoveCursor(-1),
		]
	);
}

#[test]
fn correction_is_skipped_when_the_editor_asks_for_none() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0008_0000, 0);
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn low_confidence_words_are_left_alone() {
	let mut engine = plain();
	for key in ["m", "m", "m"] {
		engine.on_key(key);
	}
	assert_eq!(engine.on_key("space"), vec![Action::Commit(" ".into())]);
}

#[test]
fn corrects_a_transposition_instead_of_a_more_frequent_deletion() {
	let mut engine = plain();
	for key in ["t", "s", "e", "t"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(4),
			Action::Commit("test".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn corrects_an_elision_with_a_typo_in_the_remainder() {
	for typed in [["t", "a", "i", "m", "x"], ["t", "a", "i", "p", "e"]] {
		let mut engine = plain();
		for key in typed {
			engine.on_key(key);
		}
		assert_eq!(
			engine.on_key("space"),
			vec![
				Action::DeleteChars(5),
				Action::Commit("t'aime".into()),
				Action::Commit(" ".into()),
			]
		);
	}
}

#[test]
fn corrects_the_second_word_of_a_phrase() {
	let mut engine = plain();
	for key in ["j", "e"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	for key in ["t", "a", "i", "m", "x"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("t'aime".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn rescues_a_completely_misspelled_long_word() {
	let mut engine = plain();
	for key in ["g", "e", "q", "n", "v", "x"] {
		engine.on_key(key);
	}
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["suggestions"][0], "beauté");
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(6),
			Action::Commit("beauté".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn short_near_key_words_are_offered_and_corrected() {
	let mut engine = plain();
	engine.on_key("p");
	engine.on_key("q");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions: Vec<&str> = json["suggestions"]
	.as_array()
	.unwrap()
	.iter()
	.map(|value| value.as_str().unwrap())
	.collect();
	assert!(suggestions.contains(&"ma"), "suggestions: {suggestions:?}");
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(2),
			Action::Commit("la".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn rescues_an_elision_whose_clitic_is_a_neighbouring_key() {
	let mut engine = plain();
	for key in ["v", "q", "j", "m", "d"] {
		engine.on_key(key);
	}
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["suggestions"][0], "t'aime");
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(5),
			Action::Commit("t'aime".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn corrects_a_known_rare_word_to_a_neighbouring_word() {
	let mut engine = plain();
	engine.on_key("p");
	engine.on_key("a");
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(2),
			Action::Commit("la".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn corrects_a_full_word_with_nearby_keys() {
	let mut engine = plain();
	for key in ["s", "e", "c", "g"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(4),
			Action::Commit("sert".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn short_ambiguous_words_are_still_corrected() {
	let mut engine = plain();
	engine.on_key("u");
	engine.on_key("e");
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(2),
			Action::Commit("je".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn short_word_suggestions_put_likely_words_first() {
	let mut engine = plain();
	engine.on_key("u");
	engine.on_key("e");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions: Vec<&str> = json["suggestions"]
	.as_array()
	.unwrap()
	.iter()
	.map(|value| value.as_str().unwrap())
	.collect();
	let position = |word: &str| suggestions.iter().position(|s| *s == word);
	let ne = position("ne").expect("ne in suggestions");
	let eu = position("eu");
	assert!(eu.is_none() || ne < eu.unwrap(), "suggestions: {suggestions:?}");
}

#[test]
fn render_exposes_three_bands() {
	let mut engine = plain();
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["page"], "letters");
	assert_eq!(json["letters"].as_array().unwrap().len(), 26);
	assert_eq!(json["keys"].as_array().unwrap().len(), 5);
	assert_eq!(json["keys"][0]["id"], "page");
	assert_eq!(json["keys"][0]["alts"][0], "1");
	assert_eq!(json["keys"][0]["alts"][9], "0");
	assert_eq!(json["keys"][1]["id"], "comma");
	assert_eq!(json["keys"][3]["id"], "dot");
	assert_eq!(json["keys"][3]["alts"][0], "!");
	assert_eq!(json["keys"][3]["alts"][1], "?");
	assert_eq!(json["keys"][3]["alts"][2], ":");
	assert_eq!(json["keys"][4]["id"], "enter");
	assert_eq!(json["strip"][0]["id"], "shift");
	assert_eq!(json["strip"][1]["id"], "backspace");
}

#[test]
fn dot_and_punctuation_alts_commit_at_the_boundary() {
	let mut engine = plain();
	engine.on_key("a");
	assert_eq!(engine.on_key("dot"), vec![Action::Commit(".".into())]);

	for symbol in ["!", "?", ":"] {
		let mut engine = plain();
		engine.on_key("a");
		assert_eq!(
			engine.on_key(&format!("sym:{symbol}")),
			vec![
				Action::Commit(" ".into()),
				Action::Commit(symbol.into()),
			]
		);
	}
}

#[test]
fn digit_shortcut_commits_without_changing_page() {
	let mut engine = plain();
	engine.on_key("a");
	assert_eq!(engine.on_key("sym:1"), vec![Action::Commit("1".into())]);
	assert_eq!(engine.page(), Page::Letters);
	assert_eq!(commits(&engine.on_key("b")), vec!["b"]);
}

#[test]
fn auto_corrects_the_capitalized_recorrection_example() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	for key in ["r", "o", "p", "p", "x", "j", "f"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("space"),
		vec![
			Action::DeleteChars(7),
			Action::Commit("Comment".into()),
			Action::Commit(" ".into()),
		]
	);
}

#[test]
fn backspace_after_a_boundary_can_reopen_the_word() {
	let mut engine = plain();
	for key in ["v", "q", "v", "u", "s", "c", "a"] {
		engine.on_key(key);
	}
	engine.on_key("space");
	assert_eq!(
		engine.on_key("backspace"),
		vec![Action::Delete { word: false }]
	);
	// The IME re-reads the word after the delete; suggestions must come back.
	engine.set_context_word("vqvusca", "");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(!json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn punctuation_after_a_suggestion_removes_the_auto_space() {
	let mut engine = plain();
	for key in ["j", "a", "i"] {
		engine.on_key(key);
	}
	assert_eq!(
		engine.on_key("sug:0"),
		vec![
			Action::DeleteChars(3),
			Action::CommitSuggestion("j'ai ".into()),
		]
	);
	assert_eq!(
		engine.on_key("comma"),
		vec![
			Action::Delete { word: false },
			Action::Commit(",".into()),
		]
	);
}

#[test]
fn comma_removes_a_user_typed_space_too() {
	let mut engine = plain();
	engine.on_key("a");
	engine.on_key("space");
	assert_eq!(
		engine.on_key("comma"),
		vec![
			Action::Delete { word: false },
			Action::Commit(",".into()),
		]
	);
}

#[test]
fn moving_back_into_a_word_offers_a_recorrection() {	let mut engine = plain();
	engine.set_context_word("Ropp", "xjf");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["suggestions"][0], "Comment");
	assert_eq!(
		engine.on_key("sug:0"),
		vec![Action::ReplaceSurrounding {
			before: 4,
			after: 3,
			text: "Comment".into(),
		}]
	);
	// The context is consumed; the strip goes back to empty.
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn moving_back_into_a_known_word_keeps_the_strip_empty() {
	let mut engine = plain();
	engine.set_context_word("bon", "jour");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn symbol_row_is_valid_json_with_quotes() {	let mut engine = plain();
	engine.on_key("page");
	engine.on_key("page");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let labels: Vec<&str> = (
		json["letters"]
		. as_array()
		. unwrap()
		. iter()
		. map(|letter| letter["label"].as_str().unwrap())
		. collect()
	);
	assert!(labels.contains(&"\""));
	assert_eq!(json["suggestions"].as_array().unwrap().len(), 0);
}
