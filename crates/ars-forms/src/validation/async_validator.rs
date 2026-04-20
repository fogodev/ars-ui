//! Asynchronous validation trait.
//!
//! Defines the [`AsyncValidator`] trait and [`BoxedAsyncValidator`] type alias
//! for type-erased async validators. These are used by
//! [`FormContext`](crate::FormContext) to support async validation (e.g.,
//! server-side uniqueness checks).

use std::{
    fmt::{self, Debug},
    pin::Pin,
    sync::Arc,
};

use super::{
    result::Result,
    validator::{Context, OwnedContext},
};
use crate::field::Value;

/// The future type returned by [`AsyncValidator::validate_async`].
type AsyncValidationFuture<'a> = dyn Future<Output = Result> + Send + 'a;

/// Async validation trait.
///
/// Requires `Send + Sync` on all targets. On wasm32 (single-threaded),
/// `Send + Sync` is trivially satisfied — the same convention used by
/// [`PlatformEffects`](ars_core::PlatformEffects) and
/// [`ModalityContext`](ars_core::ModalityContext).
pub trait AsyncValidator: Send + Sync {
    /// Validates the given value asynchronously.
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<AsyncValidationFuture<'a>>>;
}

/// A type-erased async validator.
///
/// Uses [`Arc`](std::sync::Arc) on all targets for cheap shared ownership.
pub type BoxedAsyncValidator = Arc<dyn AsyncValidator>;

/// Closure-backed async validator for custom asynchronous validation logic.
///
/// Wraps a function `F(String, OwnedContext) -> Future<Output = Result>` as an
/// [`AsyncValidator`]. The closure receives owned data (the stringified value
/// and a snapshot of the context) so the returned future can outlive the
/// original borrows.
pub struct AsyncFnValidator<F> {
    /// The closure implementing async validation behavior.
    pub f: F,
}

impl<F> Debug for AsyncFnValidator<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncFnValidator").finish_non_exhaustive()
    }
}

impl<F, Fut> AsyncValidator for AsyncFnValidator<F>
where
    F: Fn(String, OwnedContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result> + Send + 'static,
{
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<AsyncValidationFuture<'a>>> {
        let text = value.to_string_for_validation();

        let owned_ctx = ctx.snapshot();

        Box::pin((self.f)(text, owned_ctx))
    }
}

impl<F, Fut> AsyncFnValidator<F>
where
    F: Fn(String, OwnedContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result> + Send + 'static,
{
    /// Wraps a closure as an async validator value.
    #[must_use]
    pub const fn new(f: F) -> Self {
        Self { f }
    }

    /// Boxes the validator behind the standard shared pointer type.
    #[must_use]
    pub fn boxed(self) -> BoxedAsyncValidator {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use core::{
        pin::Pin,
        task::{Context as TaskContext, Poll, Waker},
    };
    use std::sync::Arc;

    use super::*;
    use crate::{
        field::Value,
        validation::{
            error::{Error, ErrorCode, Errors},
            validator::OwnedContext,
        },
    };

    fn block_on_ready<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let mut context = TaskContext::from_waker(Waker::noop());

        let mut future = Pin::from(Box::new(future));

        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test future unexpectedly pending"),
        }
    }

    /// Verifies that `BoxedAsyncValidator` can hold a trait object.
    struct AlwaysValidAsync;

    impl AsyncValidator for AlwaysValidAsync {
        fn validate_async<'a>(
            &'a self,
            _value: &'a Value,
            _ctx: &'a Context<'a>,
        ) -> Pin<Box<AsyncValidationFuture<'a>>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn async_validator_trait_object_compiles() {
        let _boxed: BoxedAsyncValidator = Arc::new(AlwaysValidAsync);
    }

    #[test]
    fn async_validator_validate_async_returns_ok() {
        let validator = AlwaysValidAsync;

        let value = Value::Text("hello".to_string());

        let ctx = Context::standalone("email");

        let result = block_on_ready(validator.validate_async(&value, &ctx));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn async_fn_validator_compiles() {
        let validator = AsyncFnValidator::new(|_text: String, _ctx: OwnedContext| async { Ok(()) });

        let _boxed: BoxedAsyncValidator = Arc::new(validator);
    }

    #[test]
    fn async_fn_validator_validate() {
        let validator = AsyncFnValidator::new(|text: String, _ctx: OwnedContext| async move {
            if text == "valid" {
                Ok(())
            } else {
                Err(Errors(vec![Error {
                    code: ErrorCode::Custom("invalid".to_string()),
                    message: "not valid".to_string(),
                }]))
            }
        });

        let valid = Value::Text("valid".to_string());

        let invalid = Value::Text("invalid".to_string());

        let ctx = Context::standalone("test");

        assert_eq!(
            block_on_ready(validator.validate_async(&valid, &ctx)),
            Ok(())
        );
        assert!(block_on_ready(validator.validate_async(&invalid, &ctx)).is_err());
    }

    #[test]
    fn async_fn_validator_converts_value_and_context() {
        use std::sync::Mutex;

        let captured = Arc::new(Mutex::new(None::<(String, OwnedContext)>));

        let captured_clone = Arc::clone(&captured);

        let validator = AsyncFnValidator::new(move |text: String, ctx: OwnedContext| {
            let captured = Arc::clone(&captured_clone);
            async move {
                *captured.lock().expect("lock poisoned") = Some((text, ctx));
                Ok(())
            }
        });

        let value = Value::Text("hello".to_string());

        let ctx = Context::standalone("email");

        let result = block_on_ready(validator.validate_async(&value, &ctx));

        assert_eq!(result, Ok(()));

        let guard = captured.lock().expect("lock poisoned");

        let (text, owned_ctx) = guard.as_ref().expect("closure not called");

        assert_eq!(text, "hello");
        assert_eq!(owned_ctx.field_name, "email");
    }

    #[test]
    fn async_fn_validator_boxed() {
        let validator = AsyncFnValidator::new(|_text: String, _ctx: OwnedContext| async { Ok(()) });

        let _boxed: BoxedAsyncValidator = validator.boxed();
    }

    #[test]
    fn async_fn_validator_with_number_value() {
        use std::sync::Mutex;

        let captured = Arc::new(Mutex::new(None::<String>));

        let captured_clone = Arc::clone(&captured);

        let validator = AsyncFnValidator::new(move |text: String, _ctx: OwnedContext| {
            let captured = Arc::clone(&captured_clone);
            async move {
                *captured.lock().expect("lock poisoned") = Some(text);
                Ok(())
            }
        });

        let value = Value::Number(Some(42.5));

        let ctx = Context::standalone("age");

        let result = block_on_ready(validator.validate_async(&value, &ctx));

        assert_eq!(result, Ok(()));

        let guard = captured.lock().expect("lock poisoned");

        assert_eq!(
            guard.as_deref(),
            Some("42.5"),
            "Number value should be stringified via to_string_for_validation"
        );
    }

    #[test]
    fn async_fn_validator_with_none_number_value() {
        use std::sync::Mutex;

        let captured = Arc::new(Mutex::new(None::<String>));

        let captured_clone = Arc::clone(&captured);

        let validator = AsyncFnValidator::new(move |text: String, _ctx: OwnedContext| {
            let captured = Arc::clone(&captured_clone);
            async move {
                *captured.lock().expect("lock poisoned") = Some(text);
                Ok(())
            }
        });

        let value = Value::Number(None);

        let ctx = Context::standalone("quantity");

        let result = block_on_ready(validator.validate_async(&value, &ctx));

        assert_eq!(result, Ok(()));

        let guard = captured.lock().expect("lock poisoned");

        assert_eq!(
            guard.as_deref(),
            Some(""),
            "Number(None) should produce empty string"
        );
    }

    #[test]
    fn async_fn_validator_debug() {
        let validator = AsyncFnValidator::new(|_text: String, _ctx: OwnedContext| async { Ok(()) });

        let debug = format!("{validator:?}");

        assert!(
            debug.contains("AsyncFnValidator"),
            "Debug output should contain type name"
        );
    }
}
