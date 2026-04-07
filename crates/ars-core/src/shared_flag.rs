//! Shared boolean flag for cross-interaction state coordination.
//!
//! [`SharedFlag`] wraps an [`AtomicBool`](core::sync::atomic::AtomicBool) in an
//! [`ArsRc`](crate::ArsRc), providing a cloneable, shared boolean that is
//! thread-safe on native targets and zero-overhead on wasm.

use core::fmt::{self, Debug};

use crate::ArsRc;

/// Shared boolean flag for cross-interaction state coordination.
///
/// Cloning shares the same underlying flag. Uses [`AtomicBool`](core::sync::atomic::AtomicBool)
/// for the value (zero overhead on wasm) and [`ArsRc`] for shared ownership.
///
/// Primary uses:
/// - `PressConfig::long_press_cancel_flag` — `LongPress` sets, `Press` reads
/// - `PressEvent::continue_propagation` — shared across cloned events
#[derive(Clone)]
pub struct SharedFlag(ArsRc<core::sync::atomic::AtomicBool>);

impl SharedFlag {
    /// Creates a new shared flag with the given initial value.
    #[must_use]
    pub fn new(value: bool) -> Self {
        Self(ArsRc::new(core::sync::atomic::AtomicBool::new(value)))
    }

    /// Reads the current flag value.
    #[must_use]
    pub fn get(&self) -> bool {
        self.0.load(core::sync::atomic::Ordering::Acquire)
    }

    /// Sets the flag value.
    pub fn set(&self, value: bool) {
        self.0.store(value, core::sync::atomic::Ordering::Release);
    }
}

impl Default for SharedFlag {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Debug for SharedFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SharedFlag").field(&self.get()).finish()
    }
}

impl PartialEq for SharedFlag {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_flag_new_stores_initial_value() {
        let flag_false = SharedFlag::new(false);
        assert!(!flag_false.get());

        let flag_true = SharedFlag::new(true);
        assert!(flag_true.get());
    }

    #[test]
    fn shared_flag_default_is_false() {
        let flag = SharedFlag::default();
        assert!(!flag.get());
    }

    #[test]
    fn shared_flag_set_updates_value() {
        let flag = SharedFlag::new(false);
        assert!(!flag.get());

        flag.set(true);
        assert!(flag.get());

        flag.set(false);
        assert!(!flag.get());
    }

    #[test]
    fn shared_flag_clone_shares_state() {
        let flag1 = SharedFlag::new(false);
        let flag2 = flag1.clone();

        // Both start false
        assert!(!flag1.get());
        assert!(!flag2.get());

        // Setting on one affects the other
        flag1.set(true);
        assert!(flag2.get());
    }

    #[test]
    fn shared_flag_debug_shows_value() {
        let flag = SharedFlag::new(true);
        let debug = alloc::format!("{flag:?}");
        assert!(debug.contains("SharedFlag"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn shared_flag_partial_eq_by_pointer_identity() {
        let flag1 = SharedFlag::new(false);
        let flag2 = flag1.clone();
        let flag3 = SharedFlag::new(false);

        // Same allocation
        assert_eq!(flag1, flag2);
        // Different allocation (same value but different pointer)
        assert_ne!(flag1, flag3);
    }
}
