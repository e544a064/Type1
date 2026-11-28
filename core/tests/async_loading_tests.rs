use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use type1_core::Engine;

fn render(engine: &mut Engine) -> Value {
	serde_json::from_str(&engine.render_json()).unwrap()
}

#[test]
fn korean_dictionary_loads_without_blocking_the_engine() {
	let mut engine = Engine::new();
	engine.set_input_type(0x0000_0001, 0);
	assert!(engine.set_profile_ids("sebeolsik", &["ko"]));
	engine.prepare();

	let first = render(&mut engine);
	assert_eq!(first["loading"], true);
	assert!(first["suggestions"].as_array().unwrap().is_empty());

	let deadline = Instant::now() + Duration::from_secs(20);
	let mut ready = false;
	while Instant::now() < deadline {
		thread::sleep(Duration::from_millis(50));
		let now = render(&mut engine);
		if now["loading"] == false {
			ready = true;
			break;
		}
	}
	assert!(ready, "Korean dictionary did not finish loading");

	for key in ["ko_g", "ko_k", "ko_s"] {
		engine.on_key(key);
	}
	let after = render(&mut engine);
	assert_eq!(after["loading"], false);
}
