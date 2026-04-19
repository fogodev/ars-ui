//! Validation errors, results, validator traits, and context.

mod async_validator;
mod built_in;
mod error;
mod result;
mod validator;

pub use async_validator::{AsyncValidator, BoxedAsyncValidator};
pub use built_in::{
    EmailValidator, FnValidator, MaxLengthValidator, MaxValidator, MinLengthValidator,
    MinValidator, PatternValidator, PatternValidatorError, RequiredValidator, StepValidator,
    UrlValidator,
};
pub use error::{Error, ErrorCode, Errors};
pub use result::{Result, ResultExt};
pub use validator::{BoxedValidator, Context, OwnedContext, Validator, boxed_validator};
