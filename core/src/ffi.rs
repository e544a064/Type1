//! JNI surface. Keep it tiny: create/destroy, render, key, input type,
//! editor actions and keyboard profiles.
//!
//! Kotlin side: `fr.type1.keyboard.core.NativeBridge`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use jni::objects::{JIntArray, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, jsize, jstring};
use jni::JNIEnv;

use crate::action::Action;
use crate::engine::Engine;
use crate::language;
use crate::layout;
use crate::personal::PersonalWord;

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
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSetPersonalWords(
	mut env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	words: JObjectArray<'_>,
	locales: JObjectArray<'_>,
	frequencies: JIntArray<'_>,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		let count = match env.get_array_length(&words) {
			Ok(value) => value.max(0) as usize,
			Err(_) => return,
		};
		let locale_count = env.get_array_length(&locales).unwrap_or(0).max(0) as usize;
		let frequency_count = (
			env
			. get_array_length(&frequencies)
			. unwrap_or(0)
			. max(0) as usize
		);
		let mut frequency_values = vec![0i32; count];
		let copy_count = frequency_count.min(count);
		if copy_count > 0 {
			let _ = env.get_int_array_region(
				&frequencies,
				0,
				&mut frequency_values[..copy_count],
			);
		}

		let mut entries = Vec::with_capacity(count);
		for index in 0..count {
			let word = match env.get_object_array_element(&words, index as jsize) {
				Ok(object) if object.as_raw().is_null() => continue,
				Ok(object) => {
					let string = JString::from(object);
					let value = match env.get_string(&string) {
						Ok(value) => value.to_string_lossy().into_owned(),
						Err(_) => continue,
					};
					let _ = env.delete_local_ref(string);
					value
				}
				Err(_) => continue,
			};
			if word.trim().is_empty() {
				continue;
			}
			let locale = if index < locale_count {
				match env.get_object_array_element(&locales, index as jsize) {
					Ok(object) if object.as_raw().is_null() => None,
					Ok(object) => {
						let string = JString::from(object);
						let value = match env.get_string(&string) {
							Ok(value) => {
								let value = value.to_string_lossy().into_owned();
								if value.trim().is_empty() {
									None
								} else {
									Some(value)
								}
							}
							Err(_) => None,
						};
						let _ = env.delete_local_ref(string);
						value
					}
					Err(_) => None,
				}
			} else {
				None
			};
			let frequency = (
				frequency_values
				. get(index)
				. copied()
				. unwrap_or(0)
				. clamp(0, u8::MAX as i32) as u8
			);
			entries.push(PersonalWord::new(word, locale, frequency));
		}

		if let Some(engine) = lock().as_mut() {
			engine.set_personal_words(entries);
		}
	}));
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeLayouts(
	env: JNIEnv<'_>,
	_this: JObject<'_>,
) -> jstring {
	reply(&env, layout::catalog_json())
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeLanguages(
	env: JNIEnv<'_>,
	_this: JObject<'_>,
) -> jstring {
	reply(&env, language::catalog_json())
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSetProfile(
	mut env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
	layout_id: JString<'_>,
	languages: JString<'_>,
) -> jboolean {
	let found = catch_unwind(AssertUnwindSafe(|| {
		let layout_id = match env.get_string(&layout_id) {
			Ok(value) => value.to_string_lossy().into_owned(),
			Err(_) => return false,
		};
		let languages = match env.get_string(&languages) {
			Ok(value) => value.to_string_lossy().into_owned(),
			Err(_) => return false,
		};
		let language_ids: Vec<&str> = (
			languages
			. split(',')
			. map(str::trim)
			. filter(|id| !id.is_empty())
			. collect()
		);
		lock()
		. as_mut()
		. is_some_and(|engine| engine.set_profile_ids(&layout_id, &language_ids))
	}))
	. unwrap_or(false);
	if found {
		1
	} else {
		0
	}
}

#[no_mangle]
pub extern "system" fn Java_fr_type1_keyboard_core_NativeBridge_nativeSetDefaultProfile(
	_env: JNIEnv<'_>,
	_this: JObject<'_>,
	_handle: jlong,
) {
	let _ = catch_unwind(AssertUnwindSafe(|| {
		if let Some(engine) = lock().as_mut() {
			engine.set_default_profile();
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
