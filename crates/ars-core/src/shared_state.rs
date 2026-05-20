//! Shared interior-mutable state container.
//!
//! [`SharedState`] wraps a value in a platform-appropriate shared mutable
//! container: `Arc<Mutex<T>>` with `std` and `Rc<RefCell<T>>` in `no_std`.

use core::fmt::{self, Debug};

/// Shared interior-mutable state container.
///
/// Uses `Arc<Mutex<T>>` with `std` and `Rc<RefCell<T>>` in `no_std`, mirroring
/// the [`SharedFlag`](crate::SharedFlag) shared ownership model. Cloning shares
/// the same underlying state.
///
/// Unlike [`SharedFlag`](crate::SharedFlag) (which stores a single `bool`),
/// `SharedState<T>` stores an arbitrary value, enabling shared mutable state
/// for interaction result types such as `HoverResult` and `PressResult`.
#[cfg(not(feature = "std"))]
pub struct SharedState<T>(alloc::rc::Rc<core::cell::RefCell<T>>);

/// Shared interior-mutable state container.
///
/// Uses `Arc<Mutex<T>>` with `std` and `Rc<RefCell<T>>` in `no_std`, mirroring
/// the [`SharedFlag`](crate::SharedFlag) shared ownership model. Cloning shares
/// the same underlying state.
///
/// Unlike [`SharedFlag`](crate::SharedFlag) (which stores a single `bool`),
/// `SharedState<T>` stores an arbitrary value, enabling shared mutable state
/// for interaction result types such as `HoverResult` and `PressResult`.
#[cfg(feature = "std")]
pub struct SharedState<T>(alloc::sync::Arc<std::sync::Mutex<T>>);

impl<T> SharedState<T> {
    /// Creates a new shared state with the given initial value.
    #[must_use]
    pub fn new(value: T) -> Self {
        #[cfg(not(feature = "std"))]
        {
            Self(alloc::rc::Rc::new(core::cell::RefCell::new(value)))
        }

        #[cfg(feature = "std")]
        {
            Self(alloc::sync::Arc::new(std::sync::Mutex::new(value)))
        }
    }

    /// Reads the current value by cloning it.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(Clone::clone)
    }

    /// Replaces the stored value.
    pub fn set(&self, value: T) {
        #[cfg(not(feature = "std"))]
        {
            *self.0.borrow_mut() = value;
        }

        #[cfg(feature = "std")]
        {
            *self.0.lock().expect("SharedState mutex poisoned") = value;
        }
    }

    /// Borrows the inner value and applies `f`, returning the result.
    ///
    /// In `no_std` this borrows the `RefCell`; with `std` this locks the
    /// `Mutex`. The lock/borrow is released when `f` returns.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        #[cfg(not(feature = "std"))]
        {
            f(&self.0.borrow())
        }

        #[cfg(feature = "std")]
        {
            let guard = self.0.lock().expect("SharedState mutex poisoned");
            f(&guard)
        }
    }
}

#[cfg(not(feature = "std"))]
impl<T> Clone for SharedState<T> {
    fn clone(&self) -> Self {
        SharedState(alloc::rc::Rc::clone(&self.0))
    }
}

#[cfg(feature = "std")]
impl<T> Clone for SharedState<T> {
    fn clone(&self) -> Self {
        SharedState(alloc::sync::Arc::clone(&self.0))
    }
}

impl<T: Debug> Debug for SharedState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|value: &T| f.debug_tuple("SharedState").field(value).finish())
    }
}

impl<T> PartialEq for SharedState<T> {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "std"))]
        {
            alloc::rc::Rc::ptr_eq(&self.0, &other.0)
        }

        #[cfg(feature = "std")]
        {
            alloc::sync::Arc::ptr_eq(&self.0, &other.0)
        }
    }
}

impl<T: Default> Default for SharedState<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_state_new_and_get() {
        let state = SharedState::new(42u32);

        assert_eq!(state.get(), 42);
    }

    #[test]
    fn shared_state_set_updates_value() {
        let state = SharedState::new(1u32);

        state.set(99);

        assert_eq!(state.get(), 99);
    }

    #[test]
    fn shared_state_with_reads_value() {
        #[cfg(not(feature = "std"))]
        use alloc::string::String;

        let state = SharedState::new(String::from("hello"));

        let len = state.with(String::len);

        assert_eq!(len, 5);
    }

    #[test]
    fn shared_state_clone_shares_state() {
        let state1 = SharedState::new(10u32);
        let state2 = state1.clone();

        state2.set(20);

        assert_eq!(state1.get(), 20);
    }

    #[test]
    fn shared_state_debug_shows_inner_value() {
        let state = SharedState::new(42u32);

        let debug = alloc::format!("{state:?}");

        assert_eq!(debug, "SharedState(42)");
    }

    #[test]
    fn shared_state_default() {
        let state = SharedState::<u32>::default();

        assert_eq!(state.get(), 0);
    }

    #[test]
    fn shared_state_partial_eq_by_pointer_identity() {
        let state1 = SharedState::new(42u32);
        let state2 = state1.clone();
        let state3 = SharedState::new(42u32);

        // Same allocation
        assert_eq!(state1, state2);
        // Different allocation (same value but different pointer)
        assert_ne!(state1, state3);
    }
}
