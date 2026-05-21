//! Shared navigation key encoding helpers.

use alloc::{format, string::String};
use core::fmt::Write as _;

use ars_collections::Key;

/// Returns a DOM-id-safe token for a component item key.
pub(crate) fn dom_safe_key_token(key: &Key) -> String {
    match key {
        Key::Int(value) => format!("i-{value}"),

        #[cfg(feature = "uuid")]
        Key::Uuid(value) => format!("u-{value}"),

        Key::String(value) => {
            let mut token = String::from("s-");

            for byte in value.as_bytes() {
                write!(token, "{byte:02x}").expect("writing to a String cannot fail");
            }

            token
        }
    }
}
