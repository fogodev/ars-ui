//! RTL-aware directional resolution for keyboard navigation.
//!
//! Provides [`LogicalDirection`] and [`resolve_arrow_key`] so that components
//! can translate physical arrow keys into logical "forward" / "backward"
//! movement that respects the current text direction.

use ars_i18n::ResolvedDirection;

use crate::KeyboardKey;

/// Logical direction in reading order, independent of text direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalDirection {
    /// "Next" in reading order (right in LTR, left in RTL).
    Forward,
    /// "Previous" in reading order (left in LTR, right in RTL).
    Backward,
}

/// Resolve a physical arrow key to a logical direction based on text direction.
///
/// Returns `None` for non-horizontal arrow keys (`ArrowUp`, `ArrowDown`) and
/// any other key that is not `ArrowLeft` or `ArrowRight`.
pub const fn resolve_arrow_key(
    key: KeyboardKey,
    direction: ResolvedDirection,
) -> Option<LogicalDirection> {
    match (key, direction) {
        (KeyboardKey::ArrowRight, ResolvedDirection::Ltr)
        | (KeyboardKey::ArrowLeft, ResolvedDirection::Rtl) => Some(LogicalDirection::Forward),
        (KeyboardKey::ArrowLeft, ResolvedDirection::Ltr)
        | (KeyboardKey::ArrowRight, ResolvedDirection::Rtl) => Some(LogicalDirection::Backward),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- LTR tests ----

    #[test]
    fn ltr_arrow_right_is_forward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowRight, ResolvedDirection::Ltr),
            Some(LogicalDirection::Forward),
        );
    }

    #[test]
    fn ltr_arrow_left_is_backward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowLeft, ResolvedDirection::Ltr),
            Some(LogicalDirection::Backward),
        );
    }

    // ---- RTL tests ----

    #[test]
    fn rtl_arrow_right_is_backward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowRight, ResolvedDirection::Rtl),
            Some(LogicalDirection::Backward),
        );
    }

    #[test]
    fn rtl_arrow_left_is_forward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowLeft, ResolvedDirection::Rtl),
            Some(LogicalDirection::Forward),
        );
    }

    // ---- Non-horizontal keys return None ----

    #[test]
    fn arrow_up_returns_none() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowUp, ResolvedDirection::Ltr),
            None
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowUp, ResolvedDirection::Rtl),
            None
        );
    }

    #[test]
    fn arrow_down_returns_none() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowDown, ResolvedDirection::Ltr),
            None,
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowDown, ResolvedDirection::Rtl),
            None,
        );
    }

    #[test]
    fn non_arrow_key_returns_none() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::Tab, ResolvedDirection::Ltr),
            None
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::Enter, ResolvedDirection::Rtl),
            None
        );
    }
}
