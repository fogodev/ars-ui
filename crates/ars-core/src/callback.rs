//! Shared callback wrapper for event handler closures.
//!
//! [`Callback`] wraps closures in [`ArsRc`](crate::ArsRc) (platform-conditional
//! `Rc`/`Arc`) so they can be stored in Props structs, cloned cheaply, and
//! compared by pointer identity. `Clone`, `PartialEq`, `Deref`, and `AsRef`
//! all delegate to `ArsRc` with no cfg-gated code.

extern crate alloc;

use core::{
    fmt::{self, Debug},
    ops::Deref,
};

use crate::ArsRc;

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Wraps closures in [`ArsRc`] — `Rc` on WASM (single-threaded), `Arc` on
/// native (multi-threaded). This is distinct from
/// [`MessageFn`](crate::MessageFn) (used for i18n message closures) and
/// [`CleanupFn`](crate::CleanupFn) (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
pub struct Callback<T: ?Sized>(pub(crate) ArsRc<T>);

impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(self.0.clone())
    }
}

impl<T: ?Sized> Debug for Callback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
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
// These need cfg gates because the Send + Sync bounds differ by platform,
// and raw Rc/Arc construction is needed for dyn trait object coercion.

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(target_arch = "wasm32")]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + 'static) -> Self {
        Self(ArsRc(alloc::rc::Rc::new(f)))
    }
}

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(not(target_arch = "wasm32"))]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Self {
        Self(ArsRc(alloc::sync::Arc::new(f)))
    }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(Args) -> Out + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(ArsRc(alloc::rc::Rc::new(f)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(Args) -> Out + Send + Sync + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(ArsRc(alloc::sync::Arc::new(f)))
    }
}

// ── Constructors for Callback<dyn Fn()> (zero-argument) ────────────
// `dyn Fn()` and `dyn Fn(Args) -> Out` are distinct trait objects in Rust,
// so the generic `Callback::new` cannot produce `Callback<dyn Fn()>`.

/// Constructor for zero-argument `Callback<dyn Fn()>`.
#[cfg(target_arch = "wasm32")]
impl Callback<dyn Fn()> {
    /// Creates a new zero-argument callback wrapping the given closure.
    pub fn new_void(f: impl Fn() + 'static) -> Self {
        Self(ArsRc(alloc::rc::Rc::new(f)))
    }
}

/// Constructor for zero-argument `Callback<dyn Fn()>`.
#[cfg(not(target_arch = "wasm32"))]
impl Callback<dyn Fn()> {
    /// Creates a new zero-argument callback wrapping the given closure.
    pub fn new_void(f: impl Fn() + Send + Sync + 'static) -> Self {
        Self(ArsRc(alloc::sync::Arc::new(f)))
    }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn() + 'static> From<F> for Callback<dyn Fn()> {
    fn from(f: F) -> Self {
        Callback(ArsRc(alloc::rc::Rc::new(f)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn() + Send + Sync + 'static> From<F> for Callback<dyn Fn()> {
    fn from(f: F) -> Self {
        Callback(ArsRc(alloc::sync::Arc::new(f)))
    }
}

// ── Free function constructor ──────────────────────────────────────

/// Ergonomic constructor for [`Callback`] with better type inference.
///
/// The compiler can infer `Args` from the closure signature without
/// requiring turbofish syntax.
#[cfg(target_arch = "wasm32")]
pub fn callback<Args: 'static, Out: 'static>(
    f: impl Fn(Args) -> Out + 'static,
) -> Callback<dyn Fn(Args) -> Out> {
    Callback::new(f)
}

/// Ergonomic constructor for [`Callback`] with better type inference.
///
/// The compiler can infer `Args` from the closure signature without
/// requiring turbofish syntax.
#[cfg(not(target_arch = "wasm32"))]
pub fn callback<Args: 'static, Out: 'static>(
    f: impl Fn(Args) -> Out + Send + Sync + 'static,
) -> Callback<dyn Fn(Args) -> Out> {
    Callback::new(f)
}

#[cfg(test)]
mod tests {
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
        assert_eq!(cb1, cb2);
        assert_ne!(cb1, cb3);
    }
}
