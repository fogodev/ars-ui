//! Validation errors, results, validator traits, and context.

mod async_validator;
mod builder;
mod built_in;
mod debounced;
mod error;
mod result;
mod validator;

pub use async_validator::{AsyncFnValidator, AsyncValidator, BoxedAsyncValidator};
pub use builder::{ChainValidator, Validators, ValidatorsBuilder};
pub use built_in::{
    EmailValidator, FnValidator, MaxLengthValidator, MaxValidator, MinLengthValidator,
    MinValidator, PatternValidator, PatternValidatorError, RequiredValidator, StepValidator,
    UrlValidator,
};
pub use debounced::{DebouncedAsyncValidator, TimerHandle};
pub use error::{Error, ErrorCode, Errors};
pub use result::{Result, ResultExt};
pub use validator::{BoxedValidator, Context, OwnedContext, Validator, boxed_validator};
