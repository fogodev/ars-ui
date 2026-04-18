//! Asynchronous validation trait.
//!
//! Defines the [`AsyncValidator`] trait and [`BoxedAsyncValidator`] type alias
//! for type-erased async validators. These are used by
//! [`FormContext`](crate::FormContext) to support async validation (e.g.,
//! server-side uniqueness checks).

use std::{pin::Pin, sync::Arc};

use super::{result::Result, validator::Context};
use crate::field::Value;

#[cfg(target_arch = "wasm32")]
type AsyncValidationFuture<'a> = dyn Future<Output = Result> + 'a;

#[cfg(not(target_arch = "wasm32"))]
type AsyncValidationFuture<'a> = dyn Future<Output = Result> + Send + 'a;

/// Async validation trait.
///
/// Native targets require async validators and their returned futures to be
/// `Send + Sync`; `wasm32` preserves single-threaded browser futures.
#[cfg(target_arch = "wasm32")]
pub trait AsyncValidator {
    /// Validates the given value asynchronously.
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<AsyncValidationFuture<'a>>>;
}

/// Async validation trait.
///
/// Native targets require async validators and their returned futures to be
/// `Send + Sync`; `wasm32` preserves single-threaded browser futures.
#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(test)]
mod tests {
    use core::{
        pin::Pin,
        task::{Context as TaskContext, Poll, Waker},
    };
    use std::{sync::Arc, task::Wake};

    use super::*;
    use crate::field::Value;

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on_ready<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let waker = Waker::from(Arc::new(NoopWake));

        let mut context = TaskContext::from_waker(&waker);

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

    #[cfg(target_arch = "wasm32")]
    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "BoxedAsyncValidator intentionally preserves single-threaded wasm validators"
    )]
    fn boxed_async_validator_allows_non_send_futures_on_wasm() {
        use std::{cell::RefCell, rc::Rc};

        struct RcAsyncValidator {
            hits: Rc<RefCell<u8>>,
        }

        impl AsyncValidator for RcAsyncValidator {
            fn validate_async<'a>(
                &'a self,
                _value: &'a Value,
                _ctx: &'a Context<'a>,
            ) -> Pin<Box<AsyncValidationFuture<'a>>> {
                let hits = Rc::clone(&self.hits);
                Box::pin(async move {
                    *hits.borrow_mut() += 1;
                    Ok(())
                })
            }
        }

        let hits = Rc::new(RefCell::new(0));
        let validator: BoxedAsyncValidator = Arc::new(RcAsyncValidator {
            hits: Rc::clone(&hits),
        });

        let value = Value::Text(String::from("hello"));
        let ctx = Context::standalone("email");

        assert_eq!(
            block_on_ready(validator.validate_async(&value, &ctx)),
            Ok(())
        );
        assert_eq!(*hits.borrow(), 1);
    }
}
