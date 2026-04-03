//! Input interaction state types and attribute merging utilities.
//!
//! This crate defines the shared interaction states (press, focus) used across
//! components and provides a helper for merging attribute maps from multiple sources.

use ars_core::AttrMap;

/// The press interaction state of a pressable element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PressState {
    /// The element is not being pressed (default).
    #[default]
    Idle,
    /// The element is currently being pressed by the user.
    Pressed,
}

/// The focus state of a focusable element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusState {
    /// The element does not have focus (default).
    #[default]
    Blurred,
    /// The element currently has focus.
    Focused,
}

/// Merges two attribute maps, with `overlay` values taking precedence over `base`.
///
/// Returns a new [`AttrMap`] containing all entries from both maps. When both maps
/// contain the same key, the value from `overlay` wins.
#[must_use]
pub fn merge_attrs(base: &AttrMap, overlay: &AttrMap) -> AttrMap {
    let mut merged = base.clone();
    for (key, value) in overlay {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_attrs_prefers_overlay_values() {
        let mut base = AttrMap::new();
        base.insert("role".into(), "button".into());
        let mut overlay = AttrMap::new();
        overlay.insert("role".into(), "switch".into());
        overlay.insert("data-state".into(), "on".into());

        let merged = merge_attrs(&base, &overlay);
        assert_eq!(merged.get("role").map(String::as_str), Some("switch"));
        assert_eq!(merged.get("data-state").map(String::as_str), Some("on"));
    }
}
