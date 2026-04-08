//! Ephemeral reference wrapper that prevents borrowed data from escaping into signals.

use std::{fmt, marker::PhantomData, rc::Rc};

/// A non-cloneable, non-copyable wrapper that prevents storing borrowed data in signals.
///
/// Used by [`with_api_ephemeral()`](crate::UseMachineReturn::with_api_ephemeral) to wrap
/// the borrowed connect API, ensuring it cannot be stored in a `Signal<T>` or `Memo<T>`
/// (which require `T: 'static`).
///
/// The `PhantomData<(Rc<()>, &'a ())>` marker provides three compile-time guarantees:
/// - **`!Send` and `!Sync`**: `Rc<()>` is neither `Send` nor `Sync`
/// - **Not `'static`**: the `&'a ()` prevents coercion to `'static`
/// - **`!Clone` and `!Copy`**: no derive, so duplication is impossible
///
/// This eliminates the use-after-free class of bugs where `Api<'a>` might outlive the
/// `Service` reference it borrows from.
///
/// ```rust,compile_fail
/// # use ars_dioxus::EphemeralRef;
/// // Cannot send across threads:
/// fn assert_send<T: Send>(_: T) {}
/// let e = EphemeralRef::new(42);
/// assert_send(e); // ERROR: Rc<()> is not Send
/// ```
///
/// ```rust,compile_fail
/// # use ars_dioxus::EphemeralRef;
/// // Cannot share across threads:
/// fn assert_sync<T: Sync>(_: T) {}
/// let e = EphemeralRef::new(42);
/// assert_sync(e); // ERROR: Rc<()> is not Sync
/// ```
///
/// ```rust,compile_fail
/// # use ars_dioxus::EphemeralRef;
/// // Cannot clone:
/// let e = EphemeralRef::new(42);
/// let _ = e.clone(); // ERROR: no Clone impl
/// ```
pub struct EphemeralRef<'a, T> {
    value: T,
    _marker: PhantomData<(Rc<()>, &'a ())>,
}

impl<'a, T> EphemeralRef<'a, T> {
    /// Creates a new ephemeral reference wrapping the given value.
    ///
    /// Only callable within `with_api_ephemeral()` closures where the borrow
    /// lifetime `'a` is tied to the `Service` access scope.
    pub const fn new(value: T) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// Returns a shared reference to the inner value.
    pub const fn get(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for EphemeralRef<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EphemeralRef").field(&self.value).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_ref_provides_access_to_inner_value() {
        let e = EphemeralRef::new(42);
        assert_eq!(*e.get(), 42);
    }

    #[test]
    fn ephemeral_ref_debug_shows_inner_value() {
        let e = EphemeralRef::new("hello");
        let debug = format!("{e:?}");
        assert!(debug.contains("hello"));
    }
}
