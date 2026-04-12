//! Weak event sender for safe effect cleanup.
//!
//! [`WeakSend`] wraps a weak reference to the send function so that long-lived
//! effects (timers, observers) do not prevent the component from being garbage
//! collected.

use core::fmt::{self, Debug};

/// Weak event sender for safe effect cleanup.
///
/// `WeakSend<T>` wraps a weak reference to the send function so that
/// long-lived effects (timers, observers) do not prevent the component
/// from being garbage collected. Use [`call_if_alive`](WeakSend::call_if_alive)
/// to dispatch events — it is a no-op if the component has been unmounted.
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

impl<T: 'static> From<&alloc::sync::Arc<dyn Fn(T) + Send + Sync>> for WeakSend<T> {
    fn from(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(alloc::sync::Arc::downgrade(arc))
    }
}

/// The strong send handle passed to effect setup closures.
///
/// Adapters hold the strong `Arc` and pass it to
/// [`PendingEffect::run`](crate::PendingEffect::run). The setup closure
/// downgrades to [`WeakSend`] internally.
#[doc(hidden)]
pub type StrongSend<E> = alloc::sync::Arc<dyn Fn(E) + Send + Sync>;

#[cfg(test)]
mod tests {
    use alloc::{format, sync::Arc};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn weak_send_calls_when_strong_sender_is_alive() {
        let total = Arc::new(AtomicUsize::new(0));
        let send: StrongSend<usize> = {
            let total = Arc::clone(&total);
            Arc::new(move |value| {
                total.fetch_add(value, Ordering::SeqCst);
            })
        };

        let weak = WeakSend::from_arc(&send);
        weak.call_if_alive(2);
        weak.call_if_alive(3);

        assert_eq!(total.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn weak_send_is_noop_after_strong_sender_drops() {
        let total = Arc::new(AtomicUsize::new(0));
        let weak = {
            let send: StrongSend<usize> = {
                let total = Arc::clone(&total);
                Arc::new(move |value| {
                    total.fetch_add(value, Ordering::SeqCst);
                })
            };
            WeakSend::downgrade(&send)
        };

        weak.call_if_alive(7);
        assert_eq!(total.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn weak_send_clone_and_debug_are_stable() {
        let send: StrongSend<usize> = Arc::new(|_| {});
        let weak = WeakSend::from(&send);
        let cloned = weak.clone();

        assert!(cloned.0.upgrade().is_some());
        assert_eq!(format!("{cloned:?}"), "WeakSend(..)");
    }
}
