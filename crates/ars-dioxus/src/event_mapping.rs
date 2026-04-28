//! Keyboard event mapping for Dioxus adapter components.

use ars_core::KeyboardKey;
use dioxus::prelude::Key;

/// Maps a Dioxus keyboard key to the framework-agnostic keyboard key.
#[must_use]
pub fn dioxus_key_to_keyboard_key(key: &Key) -> (KeyboardKey, Option<char>) {
    match key {
        Key::Character(character) => {
            if character == " " {
                return (KeyboardKey::Space, Some(' '));
            }

            (
                KeyboardKey::from_key_str(character),
                single_character(character),
            )
        }

        Key::Enter => (KeyboardKey::Enter, None),

        Key::Escape => (KeyboardKey::Escape, None),

        Key::Tab => (KeyboardKey::Tab, None),

        Key::ArrowUp => (KeyboardKey::ArrowUp, None),

        Key::ArrowDown => (KeyboardKey::ArrowDown, None),

        Key::ArrowLeft => (KeyboardKey::ArrowLeft, None),

        Key::ArrowRight => (KeyboardKey::ArrowRight, None),

        Key::Home => (KeyboardKey::Home, None),

        Key::End => (KeyboardKey::End, None),

        Key::PageUp => (KeyboardKey::PageUp, None),

        Key::PageDown => (KeyboardKey::PageDown, None),

        Key::Backspace => (KeyboardKey::Backspace, None),

        Key::Delete => (KeyboardKey::Delete, None),

        Key::F1 => (KeyboardKey::F1, None),

        Key::F2 => (KeyboardKey::F2, None),

        Key::F3 => (KeyboardKey::F3, None),

        Key::F4 => (KeyboardKey::F4, None),

        Key::F5 => (KeyboardKey::F5, None),

        Key::F6 => (KeyboardKey::F6, None),

        Key::F7 => (KeyboardKey::F7, None),

        Key::F8 => (KeyboardKey::F8, None),

        Key::F9 => (KeyboardKey::F9, None),

        Key::F10 => (KeyboardKey::F10, None),

        Key::F11 => (KeyboardKey::F11, None),

        Key::F12 => (KeyboardKey::F12, None),

        _ => (KeyboardKey::Unidentified, None),
    }
}

fn single_character(value: &str) -> Option<char> {
    let mut chars = value.chars();

    let character = chars.next()?;

    chars.next().is_none().then_some(character)
}

#[cfg(test)]
mod tests {
    use ars_core::KeyboardKey;
    use dioxus::prelude::Key;

    use super::dioxus_key_to_keyboard_key;

    #[test]
    fn maps_named_keyboard_keys() {
        let cases = [
            (Key::Enter, KeyboardKey::Enter),
            (Key::Escape, KeyboardKey::Escape),
            (Key::Tab, KeyboardKey::Tab),
            (Key::ArrowUp, KeyboardKey::ArrowUp),
            (Key::ArrowDown, KeyboardKey::ArrowDown),
            (Key::ArrowLeft, KeyboardKey::ArrowLeft),
            (Key::ArrowRight, KeyboardKey::ArrowRight),
            (Key::Home, KeyboardKey::Home),
            (Key::End, KeyboardKey::End),
            (Key::PageUp, KeyboardKey::PageUp),
            (Key::PageDown, KeyboardKey::PageDown),
            (Key::Backspace, KeyboardKey::Backspace),
            (Key::Delete, KeyboardKey::Delete),
        ];

        for (key, expected) in cases {
            assert_eq!(dioxus_key_to_keyboard_key(&key), (expected, None));
        }
    }

    #[test]
    fn maps_character_keys() {
        assert_eq!(
            dioxus_key_to_keyboard_key(&Key::Character(String::from(" "))),
            (KeyboardKey::Space, Some(' '))
        );
        assert_eq!(
            dioxus_key_to_keyboard_key(&Key::Character(String::from("a"))),
            (KeyboardKey::Unidentified, Some('a'))
        );
        assert_eq!(
            dioxus_key_to_keyboard_key(&Key::Character(String::from("é"))),
            (KeyboardKey::Unidentified, Some('é'))
        );
        assert_eq!(
            dioxus_key_to_keyboard_key(&Key::Character(String::from("ab"))),
            (KeyboardKey::Unidentified, None)
        );
    }

    #[test]
    fn maps_function_keys() {
        let cases = [
            (Key::F1, KeyboardKey::F1),
            (Key::F2, KeyboardKey::F2),
            (Key::F3, KeyboardKey::F3),
            (Key::F4, KeyboardKey::F4),
            (Key::F5, KeyboardKey::F5),
            (Key::F6, KeyboardKey::F6),
            (Key::F7, KeyboardKey::F7),
            (Key::F8, KeyboardKey::F8),
            (Key::F9, KeyboardKey::F9),
            (Key::F10, KeyboardKey::F10),
            (Key::F11, KeyboardKey::F11),
            (Key::F12, KeyboardKey::F12),
        ];

        for (key, expected) in cases {
            assert_eq!(dioxus_key_to_keyboard_key(&key), (expected, None));
        }
    }

    #[test]
    fn maps_unknown_keys_to_unidentified() {
        assert_eq!(
            dioxus_key_to_keyboard_key(&Key::Unidentified),
            (KeyboardKey::Unidentified, None)
        );
    }
}
