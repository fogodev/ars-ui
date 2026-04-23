//! Shared callback wrapper for event handler closures.
//!
//! [`Callback`] wraps closures in [`Arc`](alloc::sync::Arc) so they can be
//! stored in Props structs, cloned cheaply, and
//! compared by pointer identity. `Clone`, `PartialEq`, `Deref`, and `AsRef`
//! all delegate to `Arc` with no cfg-gated code.

use alloc::sync::Arc;
use core::{
    fmt::{self, Debug},
    ops::Deref,
};

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Wraps closures in [`Arc`] (`Arc`) on every target. This is distinct from
/// [`MessageFn`](crate::MessageFn) (used for i18n message closures) and
/// [`CleanupFn`](crate::CleanupFn) (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
pub struct Callback<T: ?Sized>(pub(crate) Arc<T>);

impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(Arc::clone(&self.0))
    }
}

impl<T: ?Sized> Debug for Callback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Deref for Callback<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> AsRef<T> for Callback<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

// ── Constructors for Callback<dyn Fn(Args) -> Out> ─────────────────
// These use raw Arc construction for dyn trait object coercion.

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl<F: Fn(Args) -> Out + Send + Sync + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(Arc::new(f))
    }
}

// ── Constructors for Callback<dyn Fn()> (zero-argument) ────────────
// `dyn Fn()` and `dyn Fn(Args) -> Out` are distinct trait objects in Rust,
// so the generic `Callback::new` cannot produce `Callback<dyn Fn()>`.

/// Constructor for zero-argument `Callback<dyn Fn()>`.
impl Callback<dyn Fn()> {
    /// Creates a new zero-argument callback wrapping the given closure.
    pub fn new_void(f: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl<F: Fn() + Send + Sync + 'static> From<F> for Callback<dyn Fn()> {
    fn from(f: F) -> Self {
        Callback(Arc::new(f))
    }
}

// ── Free function constructor ──────────────────────────────────────

/// Ergonomic constructor for [`Callback`] with better type inference.
///
/// The compiler can infer `Args` from the closure signature without
/// requiring turbofish syntax.
pub fn callback<Args: 'static, Out: 'static>(
    f: impl Fn(Args) -> Out + Send + Sync + 'static,
) -> Callback<dyn Fn(Args) -> Out> {
    Callback::new(f)
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn callback_clone_and_invoke() {
        let cb = Callback::new(|x: i32| x * 2);

        let cloned = cb.clone();

        assert_eq!(cb(21), 42);
        assert_eq!(cloned(21), 42);
    }

    #[test]
    fn callback_pointer_equality() {
        let cb1 = Callback::new(|_x: i32| {});

        let cb2 = cb1.clone();

        let cb3 = Callback::new(|_x: i32| {});

        cb1(1);
        cb3(2);

        assert_eq!(cb1, cb2);
        assert_ne!(cb1, cb3);
    }

    #[test]
    fn callback_new_void_invokes_zero_arg_closure() {
        let flag = Arc::new(core::sync::atomic::AtomicBool::new(false));

        let cb = {
            let flag = Arc::clone(&flag);
            Callback::new_void(move || {
                flag.store(true, core::sync::atomic::Ordering::SeqCst);
            })
        };

        cb();

        assert!(flag.load(core::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn callback_as_ref_exposes_inner_closure() {
        let cb = Callback::new(|value: i32| value + 1);

        assert_eq!(cb.as_ref()(41), 42);
    }

    #[test]
    fn callback_free_function_constructor_preserves_type_inference() {
        let cb = callback(|value: (i32, i32)| value.0 + value.1);

        assert_eq!(cb((20, 22)), 42);
    }

    #[test]
    fn callback_debug_output_is_stable() {
        let cb = Callback::new(|_: i32| 0);

        assert_eq!(format!("{cb:?}"), "Callback(..)");
    }

    #[test]
    fn callback_from_closure_invokes_regular_constructor_path() {
        let cb: Callback<dyn Fn(i32) -> i32> = Callback::from(|value| value + 2);

        assert_eq!(cb(40), 42);
    }

    #[test]
    fn callback_from_zero_arg_closure_invokes_void_constructor_path() {
        let flag = Arc::new(core::sync::atomic::AtomicBool::new(false));

        let cb: Callback<dyn Fn()> = {
            let flag = Arc::clone(&flag);
            Callback::from(move || {
                flag.store(true, core::sync::atomic::Ordering::SeqCst);
            })
        };

        cb();

        assert!(flag.load(core::sync::atomic::Ordering::SeqCst));
    }
}
