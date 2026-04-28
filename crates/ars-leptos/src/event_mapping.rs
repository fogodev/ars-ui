//! Keyboard event mapping for Leptos adapter components.

use ars_core::KeyboardKey;
use leptos::web_sys;

/// Maps a Leptos DOM keyboard event to the framework-agnostic keyboard key.
#[must_use]
pub fn leptos_key_to_keyboard_key(event: &web_sys::KeyboardEvent) -> (KeyboardKey, Option<char>) {
    let key_str = event.key();

    let character = single_character(&key_str);

    (KeyboardKey::from_key_str(&key_str), character)
}

fn single_character(value: &str) -> Option<char> {
    let mut chars = value.chars();

    let character = chars.next()?;

    chars.next().is_none().then_some(character)
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use ars_core::KeyboardKey;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::leptos_key_to_keyboard_key;

    wasm_bindgen_test_configure!(run_in_browser);

    fn keyboard_event(key: &str) -> web_sys::KeyboardEvent {
        let init = web_sys::KeyboardEventInit::new();

        init.set_key(key);

        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("KeyboardEvent must construct")
    }

    #[wasm_bindgen_test]
    fn maps_dom_keyboard_event_keys() {
        let cases = [
            ("Enter", KeyboardKey::Enter, None),
            (" ", KeyboardKey::Space, Some(' ')),
            ("ArrowUp", KeyboardKey::ArrowUp, None),
            ("Escape", KeyboardKey::Escape, None),
            ("a", KeyboardKey::Unidentified, Some('a')),
            ("é", KeyboardKey::Unidentified, Some('é')),
            ("ab", KeyboardKey::Unidentified, None),
        ];

        for (key, expected_key, expected_char) in cases {
            let event = keyboard_event(key);

            assert_eq!(
                leptos_key_to_keyboard_key(&event),
                (expected_key, expected_char)
            );
        }
    }
}
