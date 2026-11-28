use serde_json::Value;
use type1_core::dictionary::KOREAN;
use type1_core::{Action, Engine, Profile};

const TEXT: i32 = 0x0000_0001;

fn korean_engine() -> Engine {
	let mut engine = Engine::new();
	engine.set_input_type(TEXT, 0);
	assert!(engine.set_profile_ids("sebeolsik", &["ko"]));
	engine.reset();
	engine
}

fn apply(text: &mut String, actions: &[Action]) {
	for action in actions {
		match action {
			Action::Commit(value) | Action::CommitSuggestion(value) => {
				text.push_str(value);
			}
			Action::Delete { .. } => {
				text.pop();
			}
			Action::DeleteChars(count) => {
				for _ in 0..*count {
					text.pop();
				}
			}
			_ => panic!("unexpected action {action:?}"),
		}
	}
}

fn type_ids(engine: &mut Engine, ids: &[&str]) -> String {
	let mut text = String::new();
	for id in ids {
		let actions = engine.on_key(id);
		apply(&mut text, &actions);
	}
	text
}

fn suggestions(engine: &mut Engine) -> Vec<String> {
	let json: Value = serde_json::from_str(&engine.render_json()).unwrap();
	json["suggestions"]
	. as_array()
	. unwrap()
	. iter()
	. map(|value| value.as_str().unwrap().to_string())
	. collect()
}

#[test]
fn the_spacebar_shows_the_korean_language_code() {
	let mut engine = korean_engine();
	let json: Value = serde_json::from_str(&engine.render_json()).unwrap();
	let label = (
		json["keys"]
		. as_array()
		. unwrap()
		. iter()
		. find(|key| key["id"] == "space")
		. unwrap()["label"]
		. as_str()
		. unwrap()
	);
	assert_eq!(label, "KO");
}

#[test]
fn tense_consonants_can_be_typed_from_the_popup() {
	let mut engine = korean_engine();
	assert_eq!(
		type_ids(&mut engine, &["alt:ㅆ", "ko_k"]),
		"싸"
	);

	let mut engine = korean_engine();
	assert_eq!(
		type_ids(&mut engine, &["alt:ㄲ", "ko_k"]),
		"까"
	);
}

#[test]
fn shift_variants_are_suggested_from_the_base_jamo() {
	let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
	let suggestion = profile.suggest("슨다");
	let texts: Vec<&str> = (
		suggestion
		. candidates
		. iter()
		. map(|candidate| candidate.text.as_str())
		. collect()
	);
	assert!(texts.contains(&"쓴다"), "got {texts:?}");
}

#[test]
fn tense_consonants_are_exposed_to_the_letter_popup() {
	let mut engine = korean_engine();
	let json: Value = serde_json::from_str(&engine.render_json()).unwrap();
	assert_eq!(json["mergeLetterAlts"], false);
	let letter = (
		json["letters"]
		. as_array()
		. unwrap()
		. iter()
		. find(|letter| letter["id"] == "ko_t")
		. unwrap()
	);
	assert_eq!(letter["commit"], "ㅅ");
	assert_eq!(letter["alts"][0], "ㅆ");

	engine.on_key("shift");
	let json: Value = serde_json::from_str(&engine.render_json()).unwrap();
	let letter = (
		json["letters"]
		. as_array()
		. unwrap()
		. iter()
		. find(|letter| letter["id"] == "ko_t")
		. unwrap()
	);
	assert_eq!(letter["commit"], "ㅆ");
}

#[test]
fn typing_a_consonant_then_a_vowel_composes_a_syllable() {
	let mut engine = korean_engine();
	assert_eq!(type_ids(&mut engine, &["ko_r", "ko_k"]), "가");
}

#[test]
fn a_trailing_consonant_moves_to_the_next_syllable() {
	let mut engine = korean_engine();
	assert_eq!(
		type_ids(&mut engine, &["ko_r", "ko_k", "ko_s", "ko_k"]),
		"가나"
	);
}

#[test]
fn shift_types_tense_consonants() {
	let mut engine = korean_engine();
	assert_eq!(
		type_ids(&mut engine, &["shift", "ko_r", "ko_k"]),
		"까"
	);
}

#[test]
fn the_rendered_key_label_follows_shift() {
	let mut engine = korean_engine();
	let label = |engine: &mut Engine| -> String {
		let json: Value = serde_json::from_str(&engine.render_json()).unwrap();
		json["letters"]
		. as_array()
		. unwrap()
		. iter()
		. find(|letter| letter["id"] == "ko_r")
		. unwrap()["label"]
		. as_str()
		. unwrap()
		. to_string()
	};
	assert_eq!(label(&mut engine), "ㄱ");
	engine.on_key("shift");
	assert_eq!(label(&mut engine), "ㄲ");
}

#[test]
fn compound_vowels_and_finals_are_composed_and_undone() {
	let mut engine = korean_engine();
	assert_eq!(
		type_ids(&mut engine, &["ko_d", "ko_h", "ko_k"]),
		"와"
	);

	let mut engine = korean_engine();
	let mut text = type_ids(&mut engine, &["ko_r", "ko_k", "ko_q", "ko_t"]);
	assert_eq!(text, "값");
	apply(&mut text, &engine.on_key("backspace"));
	assert_eq!(text, "갑");
	apply(&mut text, &engine.on_key("backspace"));
	assert_eq!(text, "가");
}

#[test]
fn backspace_removes_one_jamo_at_a_time() {
	let mut engine = korean_engine();
	let mut text = type_ids(&mut engine, &["ko_r", "ko_k"]);
	assert_eq!(text, "가");
	apply(&mut text, &engine.on_key("backspace"));
	assert_eq!(text, "ㄱ");
	apply(&mut text, &engine.on_key("backspace"));
	assert_eq!(text, "");
}

#[test]
fn suggestions_compose_jamo_and_rank_by_frequency() {
	let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
	let suggestion = profile.suggest("한굴");
	assert!(suggestion.apply);
	let texts: Vec<&str> = (
		suggestion
		. candidates
		. iter()
		. map(|candidate| candidate.text.as_str())
		. collect()
	);
	assert_eq!(texts.first().copied(), Some("한국"));
	assert!(texts.contains(&"한글"), "got {texts:?}");
	for text in texts {
		assert!(KOREAN.exact(text).is_some(), "{text} is not a word");
	}
}

#[test]
fn multi_slip_korean_words_are_suggested() {
	let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
	let suggestion = profile.suggest("댈개");
	let texts: Vec<&str> = (
		suggestion
		. candidates
		. iter()
		. map(|candidate| candidate.text.as_str())
		. collect()
	);
	assert!(texts.contains(&"딸기"), "got {texts:?}");

	let mut engine = korean_engine();
	let text = type_ids(
		&mut engine,
		&["ko_e", "ko_o", "ko_f", "ko_r", "ko_o"],
	);
	assert_eq!(text, "댈개");
	let strip = suggestions(&mut engine);
	assert!(strip.contains(&"딸기".to_string()), "got {strip:?}");
}

#[test]
fn a_rare_known_word_shows_the_frequent_shift_variant_in_the_strip() {
	let mut engine = korean_engine();
	let text = type_ids(
		&mut engine,
		&["ko_e", "ko_k", "ko_f", "ko_r", "ko_l"],
	);
	assert_eq!(text, "달기");
	let strip = suggestions(&mut engine);
	assert!(strip.contains(&"딸기".to_string()), "got {strip:?}");
}

#[test]
fn a_known_korean_word_is_not_auto_corrected() {
	let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
	let suggestion = profile.suggest("한국");
	assert!(!suggestion.apply);
	assert!(
		suggestion
		. candidates
		. iter()
		. all(|candidate| candidate.text != "한국")
	);
}

#[test]
fn a_known_rare_word_still_suggests_a_frequent_shift_variant() {
	let profile = Profile::from_ids("sebeolsik", &["ko"]).unwrap();
	let suggestion = profile.suggest("달기");
	assert!(!suggestion.apply);
	let texts: Vec<&str> = (
		suggestion
		. candidates
		. iter()
		. map(|candidate| candidate.text.as_str())
		. collect()
	);
	assert!(texts.contains(&"딸기"), "got {texts:?}");
}

#[test]
fn finishing_a_word_applies_the_best_korean_correction() {
	let mut engine = korean_engine();
	let mut text = type_ids(
		&mut engine,
		&["ko_g", "ko_k", "ko_s", "ko_r", "ko_n", "ko_f"],
	);
	assert_eq!(text, "한굴");
	apply(&mut text, &engine.on_key("space"));
	assert_eq!(text, "한국 ");
}

#[test]
fn korean_suggestions_are_visible_before_a_boundary() {
	let mut engine = korean_engine();
	type_ids(
		&mut engine,
		&["ko_g", "ko_k", "ko_s", "ko_r", "ko_n", "ko_f"],
	);
	let strip = suggestions(&mut engine);
	assert!(strip.contains(&"한국".to_string()), "got {strip:?}");
	assert!(strip.contains(&"한글".to_string()), "got {strip:?}");
}
