//! Shared callback wrapper for event handler closures.
//!
//! [`Callback`] wraps closures in a platform-appropriate smart pointer (`Rc`
//! on wasm, `Arc` on native) so they can be stored in Props structs, cloned
//! cheaply, and compared by pointer identity.

extern crate alloc;

use core::fmt::{self, Debug};

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Clones the smart pointer, NOT the closure itself. Uses `Rc` on wasm
/// (single-threaded) and `Arc` on native (multi-threaded) targets. This is
/// distinct from `CleanupFn` (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
#[cfg(target_arch = "wasm32")]
pub struct Callback<T: ?Sized>(pub(crate) alloc::rc::Rc<T>);

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Clones the smart pointer, NOT the closure itself. Uses `Rc` on wasm
/// (single-threaded) and `Arc` on native (multi-threaded) targets. This is
/// distinct from `CleanupFn` (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
#[cfg(not(target_arch = "wasm32"))]
pub struct Callback<T: ?Sized>(pub(crate) alloc::sync::Arc<T>);

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(alloc::rc::Rc::clone(&self.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(alloc::sync::Arc::clone(&self.0))
    }
}

impl<T: ?Sized> Debug for Callback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> core::ops::Deref for Callback<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> AsRef<T> for Callback<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(target_arch = "wasm32")]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + 'static) -> Self {
        Self(alloc::rc::Rc::new(f))
    }
}

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(not(target_arch = "wasm32"))]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Self {
        Self(alloc::sync::Arc::new(f))
    }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(Args) -> Out + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(alloc::rc::Rc::new(f))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(Args) -> Out + Send + Sync + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(alloc::sync::Arc::new(f))
    }
}

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
