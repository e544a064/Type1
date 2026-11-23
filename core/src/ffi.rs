//! JNI surface. Keep it tiny: create/destroy, render, key, input type.
//!
//! Kotlin side: `fr.type1.keyboard.core.NativeBridge`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use jni::objects::{JObject, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;

use crate::action::Action;
use crate::engine::Engine;

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<Engine>> {
	ENGINE
	. lock()
	. unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn reply(env: &JNIEnv<'_>, payload: String) -> jstring {
	match env.new_string(payload) {
		Ok(s) => s.into_raw(),
		Err(_) => std::ptr::null_mut(),
	}
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeCreate(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
) -> jlong {
	*lock() = Some(Engine::new());
	1
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeDestroy(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
) {
	*lock() = None;
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeReset(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		if let Some(engine) = lock().as_mut() {
			engine.reset();
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSelectionChanged(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		if let Some(engine) = lock().as_mut() {
			engine.selection_changed();
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSyncContext(
	mut env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	before: JString<'_>,
	after: JString<'_>,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		let before = match env.get_string(&before) {
			Ok(value) => value.to_string_lossy().into_owned(),
			Err(_) => return,
		};
		let after = match env.get_string(&after) {
			Ok(value) => value.to_string_lossy().into_owned(),
			Err(_) => return,
		};
		if let Some(engine) = lock().as_mut() {
			engine.set_context_word(&before, &after);
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSetInputType(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	input_type: jint,
	ime_options: jint,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		if let Some(engine) = lock().as_mut() {
			engine.set_input_type(input_type, ime_options);
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSetImeOptions(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	ime_options: jint,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		if let Some(engine) = lock().as_mut() {
			engine.set_ime_options(ime_options);
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeRender(
	env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
) -> jstring {
	let payload = (
		catch_unwind(AssertUnwindSafe(|| {
			match lock().as_mut() {
				Some(engine) => engine.render_json(),
				None => "{}".to_string(),
			}
		}))
		. unwrap_or_else(|_| "{}".to_string())
	);
	reply(&env, payload)
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeKey(
	mut env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	key_id: JString<'_>,
) -> jstring {
	let payload = (
		catch_unwind(AssertUnwindSafe(|| {
			let key = match env.get_string(&key_id) {
				Ok(s) => s.to_string_lossy().into_owned(),
				Err(_) => return "[]".to_string(),
			};
			match lock().as_mut() {
				Some(engine) => Action::list_json(&engine.on_key(&key)),
				None => "[]".to_string(),
			}
		}))
		. unwrap_or_else(|_| "[]".to_string())
	);
	reply(&env, payload)
}
