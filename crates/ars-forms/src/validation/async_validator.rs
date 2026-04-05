//! Asynchronous validation trait.
//!
//! Defines the [`AsyncValidator`] trait and [`BoxedAsyncValidator`] type alias
//! for type-erased async validators. These are used by
//! [`FormContext`](crate::FormContext) to support async validation (e.g.,
//! server-side uniqueness checks).

use core::{future::Future, pin::Pin};

use super::{result::Result, validator::Context};
use crate::field::Value;

/// Async validation trait. On non-WASM targets, returned futures must be `Send`.
/// On WASM (single-threaded), the `Send` bound is relaxed.
#[cfg(not(target_arch = "wasm32"))]
pub trait AsyncValidator: Send + Sync {
    /// Validates the given value asynchronously.
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>>;
}

/// Async validation trait (WASM variant without `Send` bounds).
#[cfg(target_arch = "wasm32")]
pub trait AsyncValidator {
    /// Validates the given value asynchronously.
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<dyn Future<Output = Result> + 'a>>;
}

/// A type-erased async validator.
///
/// Uses `Arc` on native targets for thread-safe sharing, `Rc` on WASM.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedAsyncValidator = std::sync::Arc<dyn AsyncValidator + Send + Sync>;

/// A type-erased async validator (WASM variant using `Rc`).
#[cfg(target_arch = "wasm32")]
pub type BoxedAsyncValidator = std::rc::Rc<dyn AsyncValidator>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that `BoxedAsyncValidator` can hold a trait object.
    struct AlwaysValidAsync;

    impl AsyncValidator for AlwaysValidAsync {
        fn validate_async<'a>(
            &'a self,
            _value: &'a Value,
            _ctx: &'a Context<'a>,
        ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn async_validator_trait_object_compiles() {
        let _boxed: BoxedAsyncValidator = std::sync::Arc::new(AlwaysValidAsync);
    }
}
