//! Weak event sender for safe effect cleanup.
//!
//! [`WeakSend`] wraps a weak reference to the send function so that long-lived
//! effects (timers, observers) do not prevent the component from being garbage
//! collected.

extern crate alloc;

use core::fmt::{self, Debug};

/// Weak event sender for safe effect cleanup.
///
/// `WeakSend<T>` wraps a weak reference to the send function so that
/// long-lived effects (timers, observers) do not prevent the component
/// from being garbage collected. Use [`call_if_alive`](WeakSend::call_if_alive)
/// to dispatch events — it is a no-op if the component has been unmounted.
#[cfg(target_arch = "wasm32")]
pub struct WeakSend<T>(alloc::rc::Weak<dyn Fn(T)>);

/// Weak event sender for safe effect cleanup.
///
/// `WeakSend<T>` wraps a weak reference to the send function so that
/// long-lived effects (timers, observers) do not prevent the component
/// from being garbage collected. Use [`call_if_alive`](WeakSend::call_if_alive)
/// to dispatch events — it is a no-op if the component has been unmounted.
#[cfg(not(target_arch = "wasm32"))]
pub struct WeakSend<T>(alloc::sync::Weak<dyn Fn(T) + Send + Sync>);

impl<T> WeakSend<T> {
    /// Attempt to send an event if the component is still alive.
    ///
    /// Returns silently if the strong reference has been dropped.
    pub fn call_if_alive(&self, value: T) {
        if let Some(f) = self.0.upgrade() {
            f(value);
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<T> Clone for WeakSend<T> {
    fn clone(&self) -> Self {
        WeakSend(alloc::rc::Weak::clone(&self.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> Clone for WeakSend<T> {
    fn clone(&self) -> Self {
        WeakSend(alloc::sync::Weak::clone(&self.0))
    }
}

impl<T> Debug for WeakSend<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WeakSend(..)")
    }
}

/// Convenience constructors for [`WeakSend`] on wasm targets.
#[cfg(target_arch = "wasm32")]
impl<T: 'static> WeakSend<T> {
    /// Create a `WeakSend` by downgrading the given `Rc`.
    pub fn from_rc(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        WeakSend(alloc::rc::Rc::downgrade(rc))
    }

    /// Alias for [`from_rc`](Self::from_rc) — more discoverable name.
    pub fn downgrade(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        Self::from_rc(rc)
    }
}

/// Convenience constructors for [`WeakSend`] on native targets.
#[cfg(not(target_arch = "wasm32"))]
impl<T: 'static> WeakSend<T> {
    /// Create a `WeakSend` by downgrading the given `Arc`.
    pub fn from_arc(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(alloc::sync::Arc::downgrade(arc))
    }

    /// Alias for [`from_arc`](Self::from_arc) — more discoverable name.
    pub fn downgrade(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        Self::from_arc(arc)
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: 'static> From<&alloc::rc::Rc<dyn Fn(T)>> for WeakSend<T> {
    fn from(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        WeakSend(alloc::rc::Rc::downgrade(rc))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: 'static> From<&alloc::sync::Arc<dyn Fn(T) + Send + Sync>> for WeakSend<T> {
    fn from(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(alloc::sync::Arc::downgrade(arc))
    }
}

/// The strong send handle passed to effect setup closures.
///
/// Adapters hold the strong `Rc`/`Arc` and pass it to
/// [`PendingEffect::run`](crate::PendingEffect::run). The setup closure
/// downgrades to [`WeakSend`] internally.
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub type StrongSend<E> = alloc::rc::Rc<dyn Fn(E)>;

/// The strong send handle passed to effect setup closures.
#[doc(hidden)]
#[cfg(not(target_arch = "wasm32"))]
pub type StrongSend<E> = alloc::sync::Arc<dyn Fn(E) + Send + Sync>;
