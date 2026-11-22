use type1_core::{Action, Engine, ImeActionKind, Page};

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
fn auto_capitalises_sentences() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | CAP_SENTENCES, 0);
	assert_eq!(commits(&engine.on_key("z")), vec!["Z"]);
	assert_eq!(commits(&engine.on_key("o")), vec!["o"]);
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
fn multiline_field_keeps_newline_even_with_action() {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT | 0x0002_0000, 0x0000_0006);
	assert_eq!(engine.on_key("enter"), vec![Action::Commit("\n".into())]);
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
fn suggests_after_two_letters() {
	let mut engine = plain();
	engine.on_key("e");
	engine.on_key("c");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	let suggestions = json["suggestions"].as_array().unwrap();
	assert!(!suggestions.is_empty());
	assert!(suggestions.iter().any(|s| s == "écouter"));
}

#[test]
fn applying_a_suggestion_replaces_the_word() {
	let mut engine = plain();
	engine.on_key("e");
	engine.on_key("c");
	let actions = engine.on_key("sug:0");
	assert_eq!(
		actions,
		vec![
			Action::DeleteChars(2),
			Action::Commit("écouter ".into())
		]
	);
	// The word buffer is now empty: no suggestions until new letters arrive.
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn suggestions_reset_after_space() {
	let mut engine = plain();
	engine.on_key("e");
	engine.on_key("c");
	engine.on_key("space");
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert!(json["suggestions"].as_array().unwrap().is_empty());
}

#[test]
fn render_exposes_three_bands() {
	let engine = plain();
	let json: serde_json::Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["page"], "letters");
	assert_eq!(json["letters"].as_array().unwrap().len(), 26);
	assert_eq!(json["keys"].as_array().unwrap().len(), 4);
	assert_eq!(json["keys"][0]["id"], "page");
	assert_eq!(json["keys"][3]["id"], "enter");
	assert_eq!(json["strip"][0]["id"], "shift");
	assert_eq!(json["strip"][1]["id"], "backspace");
}

#[test]
fn symbol_row_is_valid_json_with_quotes() {
	let mut engine = plain();
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
