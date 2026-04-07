//! RTL-aware directional resolution for keyboard navigation.
//!
//! Provides [`LogicalDirection`] and [`resolve_arrow_key`] so that components
//! can translate physical arrow keys into logical "forward" / "backward"
//! movement that respects the current text direction.

use ars_i18n::Direction;

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
///
/// # Panics
///
/// Debug-asserts that `direction` is not [`Direction::Auto`]. Callers must
/// resolve `Auto` to a concrete `Ltr` or `Rtl` before calling this function.
pub fn resolve_arrow_key(key: KeyboardKey, direction: Direction) -> Option<LogicalDirection> {
    debug_assert!(
        direction != Direction::Auto,
        "resolve_arrow_key requires a resolved direction"
    );
    match (key, direction) {
        (KeyboardKey::ArrowRight, Direction::Ltr) | (KeyboardKey::ArrowLeft, Direction::Rtl) => {
            Some(LogicalDirection::Forward)
        }
        (KeyboardKey::ArrowLeft, Direction::Ltr) | (KeyboardKey::ArrowRight, Direction::Rtl) => {
            Some(LogicalDirection::Backward)
        }
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
            resolve_arrow_key(KeyboardKey::ArrowRight, Direction::Ltr),
            Some(LogicalDirection::Forward),
        );
    }

    #[test]
    fn ltr_arrow_left_is_backward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowLeft, Direction::Ltr),
            Some(LogicalDirection::Backward),
        );
    }

    // ---- RTL tests ----

    #[test]
    fn rtl_arrow_right_is_backward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowRight, Direction::Rtl),
            Some(LogicalDirection::Backward),
        );
    }

    #[test]
    fn rtl_arrow_left_is_forward() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowLeft, Direction::Rtl),
            Some(LogicalDirection::Forward),
        );
    }

    // ---- Non-horizontal keys return None ----

    #[test]
    fn arrow_up_returns_none() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowUp, Direction::Ltr),
            None
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowUp, Direction::Rtl),
            None
        );
    }

    #[test]
    fn arrow_down_returns_none() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowDown, Direction::Ltr),
            None,
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowDown, Direction::Rtl),
            None,
        );
    }

    #[test]
    fn non_arrow_key_returns_none() {
        assert_eq!(resolve_arrow_key(KeyboardKey::Tab, Direction::Ltr), None);
        assert_eq!(resolve_arrow_key(KeyboardKey::Enter, Direction::Rtl), None);
    }

    // ---- Debug assertion on Direction::Auto ----

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "resolve_arrow_key requires a resolved direction")]
    fn auto_direction_panics_in_debug() {
        let _ = resolve_arrow_key(KeyboardKey::ArrowRight, Direction::Auto);
    }

    /// In release builds the `debug_assert!` is a no-op, so `Auto` gracefully
    /// falls through to `None` via the wildcard arm.
    #[test]
    #[cfg(not(debug_assertions))]
    fn auto_direction_returns_none_in_release() {
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowRight, Direction::Auto),
            None,
        );
        assert_eq!(
            resolve_arrow_key(KeyboardKey::ArrowLeft, Direction::Auto),
            None,
        );
    }
}
