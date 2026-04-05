//! Validation errors, results, validator traits, and context.

mod async_validator;
mod error;
mod result;
mod validator;

pub use async_validator::{AsyncValidator, BoxedAsyncValidator};
pub use error::{Error, ErrorCode, Errors};
pub use result::{Result, ResultExt};
pub use validator::{BoxedValidator, Context, OwnedContext, Validator, boxed_validator};
