//! Debounced async validation utilities.
//!
//! Provides [`TimerHandle`] for platform-specific timer cancellation and
//! [`DebouncedAsyncValidator`] for debouncing rapid-fire async validation
//! (e.g., server-side uniqueness checks on each keystroke).

use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
    sync::Arc,
};

use ars_i18n::Locale;

use super::{async_validator::BoxedAsyncValidator, validator::OwnedContext};
use crate::field::Value;

/// Timer handle returned by the adapter's platform timer abstraction.
///
/// On WASM, wraps `setTimeout`; on native, wraps `tokio::time::sleep` or
/// similar. Call [`cancel`](Self::cancel) to abort the pending timer.
pub struct TimerHandle {
    /// The cancellation closure provided by the platform timer.
    cancel_fn: Box<dyn FnOnce() + Send + Sync>,
}

impl TimerHandle {
    /// Creates a new timer handle wrapping the given cancellation closure.
    pub fn new(cancel_fn: Box<dyn FnOnce() + Send + Sync>) -> Self {
        Self { cancel_fn }
    }

    /// Cancels the pending timer by invoking the cancellation closure.
    pub fn cancel(self) {
        (self.cancel_fn)();
    }
}

impl Debug for TimerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimerHandle").finish_non_exhaustive()
    }
}

/// Debounced async validator — waits for `delay_ms` of inactivity before
/// delegating to the inner [`AsyncValidator`](super::async_validator::AsyncValidator).
///
/// Each call to [`validate_debounced`](Self::validate_debounced) cancels any
/// pending timer and starts a fresh one. When the timer fires, the adapter's
/// `spawn_async_validation` callback drives the inner validator to completion.
pub struct DebouncedAsyncValidator {
    /// The inner async validator to delegate to after the debounce delay.
    pub validator: BoxedAsyncValidator,

    /// Debounce delay in milliseconds.
    pub delay_ms: u32,

    /// Adapter-provided callback that spawns an async future to completion.
    ///
    /// On native, this typically wraps `tokio::spawn`; on WASM, it wraps
    /// `wasm_bindgen_futures::spawn_local`. The callback takes ownership of
    /// the validator, value, and context so the spawned future can outlive the
    /// borrow scope.
    pub spawn_async_validation: Arc<dyn Fn(BoxedAsyncValidator, Value, OwnedContext) + Send + Sync>,

    /// Handle to the currently pending debounce timer, if any.
    pending_timer: Option<TimerHandle>,
}

impl DebouncedAsyncValidator {
    /// Creates a new debounced validator wrapping the given inner validator.
    #[must_use]
    pub fn new(
        validator: BoxedAsyncValidator,
        delay_ms: u32,
        spawn_async_validation: Arc<dyn Fn(BoxedAsyncValidator, Value, OwnedContext) + Send + Sync>,
    ) -> Self {
        Self {
            validator,
            delay_ms,
            spawn_async_validation,
            pending_timer: None,
        }
    }

    /// Cancel any pending debounce timer and start a new one.
    ///
    /// After `delay_ms`, delegates to the inner
    /// [`AsyncValidator::validate_async`](super::async_validator::AsyncValidator::validate_async).
    /// The adapter provides [`TimerHandle`] via its platform timer abstraction
    /// (e.g., `setTimeout` on WASM, `tokio::time::sleep` on native).
    ///
    /// **Design note:** The `spawn_async_validation` callback receives owned
    /// data (validator, value, context) rather than a pre-built future. This
    /// avoids the lifetime problem where `validate_async` returns
    /// `Future + 'a` tied to a borrowed `Context<'a>` — the callback
    /// constructs the `Context` from the owned data inside the spawned task,
    /// where the borrow can live for the task's duration.
    pub fn validate_debounced(
        &mut self,
        value: &Value,
        name: &str,
        form_values: &BTreeMap<String, Value>,
        locale: Option<&Locale>,
        spawn_timer: impl FnOnce(u32, Box<dyn FnOnce()>) -> TimerHandle,
    ) {
        // Cancel previous pending validation.
        if let Some(handle) = self.pending_timer.take() {
            handle.cancel();
        }

        let validator = Arc::clone(&self.validator);

        let value = value.clone();

        let owned_ctx = OwnedContext {
            field_name: name.to_string(),
            form_values: form_values.clone(),
            locale: locale.cloned(),
        };

        let spawn_async = Arc::clone(&self.spawn_async_validation);

        self.pending_timer = Some(spawn_timer(
            self.delay_ms,
            Box::new(move || {
                // After delay, spawn the async validator. The spawn callback
                // takes ownership of all data and constructs the Context
                // internally, ensuring the future's lifetime is satisfied.
                spawn_async(validator, value, owned_ctx);
            }),
        ));
    }
}

impl Drop for DebouncedAsyncValidator {
    fn drop(&mut self) {
        if let Some(handle) = self.pending_timer.take() {
            handle.cancel();
        }
    }
}

impl Debug for DebouncedAsyncValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebouncedAsyncValidator")
            .field("delay_ms", &self.delay_ms)
            .field("has_pending_timer", &self.pending_timer.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    };

    use super::*;
    use crate::{field::Value, validation::validator::Context};

    #[test]
    fn timer_handle_cancel_calls_fn() {
        let called = Arc::new(AtomicBool::new(false));

        let called_clone = Arc::clone(&called);

        let handle = TimerHandle::new(Box::new(move || {
            called_clone.store(true, Ordering::Relaxed);
        }));

        assert!(!called.load(Ordering::Relaxed));

        handle.cancel();

        assert!(called.load(Ordering::Relaxed));
    }

    #[test]
    fn debounced_cancels_previous() {
        let cancel_count = Arc::new(AtomicU32::new(0));

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawn_async = Arc::new(|_v: BoxedAsyncValidator, _val: Value, _ctx: OwnedContext| {});

        let mut debounced = DebouncedAsyncValidator::new(validator, 200, spawn_async);

        let value = Value::Text("a".to_string());

        let ctx = Context::standalone("email");

        // First call — no previous timer to cancel
        let cancel_count_1 = Arc::clone(&cancel_count);

        debounced.validate_debounced(
            &value,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, _callback| {
                let cc = Arc::clone(&cancel_count_1);
                TimerHandle::new(Box::new(move || {
                    cc.fetch_add(1, Ordering::Relaxed);
                }))
            },
        );

        assert_eq!(
            cancel_count.load(Ordering::Relaxed),
            0,
            "first call should not cancel anything"
        );

        // Second call — should cancel previous timer
        let cancel_count_2 = Arc::clone(&cancel_count);

        debounced.validate_debounced(
            &value,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, _callback| {
                let cc = Arc::clone(&cancel_count_2);
                TimerHandle::new(Box::new(move || {
                    cc.fetch_add(1, Ordering::Relaxed);
                }))
            },
        );

        assert_eq!(
            cancel_count.load(Ordering::Relaxed),
            1,
            "second call should cancel the first timer"
        );
    }

    #[test]
    fn debounced_schedules_timer() {
        let captured_delay = Arc::new(AtomicU32::new(0));

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawn_async = Arc::new(|_v: BoxedAsyncValidator, _val: Value, _ctx: OwnedContext| {});

        let mut debounced = DebouncedAsyncValidator::new(validator, 300, spawn_async);

        let value = Value::Text("test".to_string());

        let ctx = Context::standalone("name");

        let captured = Arc::clone(&captured_delay);

        debounced.validate_debounced(
            &value,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |delay_ms, _callback| {
                captured.store(delay_ms, Ordering::Relaxed);
                TimerHandle::new(Box::new(|| {}))
            },
        );

        assert_eq!(captured_delay.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn debounced_spawn_timer_receives_correct_callback() {
        let spawned = Arc::new(Mutex::new(None::<(String, String)>));

        let spawned_clone = Arc::clone(&spawned);

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawn_async: Arc<dyn Fn(BoxedAsyncValidator, Value, OwnedContext) + Send + Sync> =
            Arc::new(move |_v, val, ctx| {
                *spawned_clone.lock().expect("lock poisoned") =
                    Some((val.to_string_for_validation(), ctx.field_name.clone()));
            });

        let mut debounced = DebouncedAsyncValidator::new(validator, 100, spawn_async);

        let value = Value::Text("hello".to_string());

        let ctx = Context::standalone("username");

        let mut timer_callback: Option<Box<dyn FnOnce()>> = None;

        debounced.validate_debounced(
            &value,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, callback| {
                timer_callback = Some(callback);
                TimerHandle::new(Box::new(|| {}))
            },
        );

        // Simulate timer firing
        timer_callback.expect("spawn_timer should have been called")();

        let guard = spawned.lock().expect("lock poisoned");

        let (val_str, field_name) = guard.as_ref().expect("spawn_async_validation not called");

        assert_eq!(val_str, "hello");
        assert_eq!(field_name, "username");
    }

    #[test]
    fn debounced_propagates_locale_and_form_values() {
        let spawned = Arc::new(Mutex::new(None::<OwnedContext>));

        let spawned_clone = Arc::clone(&spawned);

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawn_async: Arc<dyn Fn(BoxedAsyncValidator, Value, OwnedContext) + Send + Sync> =
            Arc::new(move |_v, _val, ctx| {
                *spawned_clone.lock().expect("lock poisoned") = Some(ctx);
            });

        let mut debounced = DebouncedAsyncValidator::new(validator, 100, spawn_async);

        let value = Value::Text("test".to_string());

        let locale = Locale::parse("en-US").expect("valid locale");

        let mut form_values = BTreeMap::new();

        form_values.insert("email".to_string(), Value::Text("a@b.com".to_string()));
        form_values.insert("age".to_string(), Value::Number(Some(25.0)));

        let mut timer_callback: Option<Box<dyn FnOnce()>> = None;

        debounced.validate_debounced(
            &value,
            "username",
            &form_values,
            Some(&locale),
            |_delay_ms, callback| {
                timer_callback = Some(callback);
                TimerHandle::new(Box::new(|| {}))
            },
        );

        // Simulate timer firing
        timer_callback.expect("spawn_timer should have been called")();

        let guard = spawned.lock().expect("lock poisoned");

        let ctx = guard.as_ref().expect("spawn_async_validation not called");

        assert_eq!(ctx.field_name, "username");
        assert_eq!(
            ctx.locale
                .as_ref()
                .expect("locale should be present")
                .to_bcp47(),
            "en-US"
        );
        assert_eq!(ctx.form_values.len(), 2);
        assert_eq!(
            ctx.form_values.get("email").and_then(|v| v.as_text()),
            Some("a@b.com")
        );
        assert_eq!(
            ctx.form_values.get("age").and_then(Value::as_number),
            Some(25.0)
        );
    }

    #[test]
    fn timer_handle_drop_without_cancel_does_not_invoke() {
        let called = Arc::new(AtomicBool::new(false));

        let called_clone = Arc::clone(&called);

        let handle = TimerHandle::new(Box::new(move || {
            called_clone.store(true, Ordering::Relaxed);
        }));

        drop(handle);

        assert!(
            !called.load(Ordering::Relaxed),
            "dropping a TimerHandle should not invoke the cancel closure"
        );
    }

    #[test]
    fn debounced_second_timer_works_after_cancellation() {
        let spawned = Arc::new(Mutex::new(Vec::<String>::new()));

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawned_clone = Arc::clone(&spawned);

        let spawn_async: Arc<dyn Fn(BoxedAsyncValidator, Value, OwnedContext) + Send + Sync> =
            Arc::new(move |_v, val, _ctx| {
                spawned_clone
                    .lock()
                    .expect("lock poisoned")
                    .push(val.to_string_for_validation());
            });

        let mut debounced = DebouncedAsyncValidator::new(validator, 100, spawn_async);

        let ctx = Context::standalone("field");

        // First call — capture but don't fire the timer
        let value_a = Value::Text("first".to_string());

        let mut first_callback: Option<Box<dyn FnOnce()>> = None;

        debounced.validate_debounced(
            &value_a,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, callback| {
                first_callback = Some(callback);
                TimerHandle::new(Box::new(|| {}))
            },
        );

        // Second call — cancels first, capture this timer
        let value_b = Value::Text("second".to_string());

        let mut second_callback: Option<Box<dyn FnOnce()>> = None;

        debounced.validate_debounced(
            &value_b,
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, callback| {
                second_callback = Some(callback);
                TimerHandle::new(Box::new(|| {}))
            },
        );

        // Fire the second timer — should spawn with "second"
        second_callback.expect("second timer should exist")();

        let guard = spawned.lock().expect("lock poisoned");

        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0], "second");
    }

    #[test]
    fn timer_handle_debug() {
        let handle = TimerHandle::new(Box::new(|| {}));

        let debug = format!("{handle:?}");

        assert!(
            debug.contains("TimerHandle"),
            "Debug output should contain type name"
        );
    }

    #[test]
    fn debounced_async_validator_debug() {
        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;

        let spawn_async = Arc::new(|_v: BoxedAsyncValidator, _val: Value, _ctx: OwnedContext| {});

        let debounced = DebouncedAsyncValidator::new(validator, 250, spawn_async);

        let debug = format!("{debounced:?}");

        assert!(
            debug.contains("DebouncedAsyncValidator"),
            "Debug output should contain type name"
        );
        assert!(
            debug.contains("250"),
            "Debug output should contain delay_ms"
        );
    }

    #[test]
    fn debounced_drop_cancels_pending_timer() {
        let cancelled = Arc::new(AtomicBool::new(false));

        let validator = Arc::new(StubAsyncValidator) as BoxedAsyncValidator;
        let spawn_async = Arc::new(|_v: BoxedAsyncValidator, _val: Value, _ctx: OwnedContext| {});

        let mut debounced = DebouncedAsyncValidator::new(validator, 100, spawn_async);
        let ctx = Context::standalone("field");

        let cancelled_clone = Arc::clone(&cancelled);
        debounced.validate_debounced(
            &Value::Text("test".to_string()),
            ctx.field_name,
            ctx.form_values,
            ctx.locale,
            |_delay_ms, _callback| {
                let flag = Arc::clone(&cancelled_clone);
                TimerHandle::new(Box::new(move || {
                    flag.store(true, Ordering::Relaxed);
                }))
            },
        );

        assert!(
            !cancelled.load(Ordering::Relaxed),
            "timer should not be cancelled yet"
        );

        // Drop the debounced validator — should cancel the pending timer
        drop(debounced);

        assert!(
            cancelled.load(Ordering::Relaxed),
            "dropping DebouncedAsyncValidator should cancel pending timer"
        );
    }

    #[test]
    fn stub_async_validator_validate_async_returns_ok() {
        use core::task::{Context as TaskContext, Poll, Waker};

        let validator = StubAsyncValidator;
        let value = Value::Text("test".to_string());
        let ctx = Context::standalone("field");
        let mut future = validator.validate_async(&value, &ctx);
        let mut task_ctx = TaskContext::from_waker(Waker::noop());

        assert!(matches!(
            future.as_mut().poll(&mut task_ctx),
            Poll::Ready(Ok(()))
        ));
    }

    // --- Test helpers ---

    use super::super::{
        async_validator::{AsyncValidator, BoxedAsyncValidator},
        validator::OwnedContext,
    };

    struct StubAsyncValidator;

    #[cfg(not(target_arch = "wasm32"))]
    impl AsyncValidator for StubAsyncValidator {
        fn validate_async<'a>(
            &'a self,
            _value: &'a Value,
            _ctx: &'a Context<'a>,
        ) -> std::pin::Pin<Box<dyn Future<Output = super::super::result::Result> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
    }

    #[cfg(target_arch = "wasm32")]
    impl AsyncValidator for StubAsyncValidator {
        fn validate_async<'a>(
            &'a self,
            _value: &'a Value,
            _ctx: &'a Context<'a>,
        ) -> std::pin::Pin<Box<dyn Future<Output = super::super::result::Result> + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }
}
