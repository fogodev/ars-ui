//! Validation result type.
//!
//! [`Result`] is a type alias for `core::result::Result<(), Errors>`, giving
//! every validation outcome the full power of Rust's standard `Result` —
//! including the `?` operator, combinators, and pattern matching with
//! `Ok(())` / `Err(errors)`.
//!
//! The [`ResultExt`] extension trait adds domain-specific helpers like
//! [`merge`](ResultExt::merge) and [`without_server_errors`](ResultExt::without_server_errors).

use super::error::{ErrorCode, Errors};

/// The result of validating a field value.
///
/// - `Ok(())` — the value passed validation.
/// - `Err(Errors)` — the value failed with one or more errors.
pub type Result = core::result::Result<(), Errors>;

/// Extension methods for [`Result`].
pub trait ResultExt {
    /// Returns the validation errors, if any.
    fn errors(&self) -> Option<&Errors>;

    /// Merges two validation results. If both are invalid, their errors
    /// are combined into one `Err`.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when either or both inputs are invalid.
    fn merge(self, other: Result) -> Result;

    /// Returns the first error message, if any.
    fn first_error_message(&self) -> Option<&str>;

    /// Returns a new `Result` with server-sourced and async errors removed.
    ///
    /// Preserves client-side validation errors. Returns `Ok(())` if no
    /// errors remain.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when client-side errors remain after filtering.
    fn without_server_errors(&self) -> Result;
}

impl ResultExt for Result {
    fn errors(&self) -> Option<&Errors> {
        self.as_ref().err()
    }

    fn merge(self, other: Result) -> Result {
        match (self, other) {
            (Ok(()), other) => other,
            (Err(mut e1), Err(e2)) => {
                e1.0.extend(e2.0);
                Err(e1)
            }
            (err, Ok(())) => err,
        }
    }

    fn first_error_message(&self) -> Option<&str> {
        self.errors().and_then(|e| e.first_message())
    }

    fn without_server_errors(&self) -> Result {
        match self {
            Ok(()) => Ok(()),
            Err(errors) => {
                let filtered = errors
                    .0
                    .iter()
                    .filter(|e| !matches!(&e.code, ErrorCode::Server(_) | ErrorCode::Async(_)))
                    .cloned()
                    .collect::<Vec<_>>();
                if filtered.is_empty() {
                    Ok(())
                } else {
                    Err(Errors(filtered))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{super::Error, *};

    #[test]
    fn valid_result_is_ok() {
        let result = Result::Ok(());
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_result_is_err() {
        let result = Result::Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "This field is required".to_string(),
        }]));
        assert!(result.is_err());
    }

    #[test]
    fn merge_both_valid() {
        let result = Ok(()).merge(Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn merge_first_invalid() {
        let invalid = Result::Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));
        let result = invalid.merge(Ok(()));
        assert!(result.is_err());
        assert_eq!(result.errors().expect("has errors").len(), 1);
    }

    #[test]
    fn merge_second_invalid() {
        let invalid = Result::Err(Errors(vec![Error {
            code: ErrorCode::Email,
            message: "bad email".to_string(),
        }]));
        let result = Ok(()).merge(invalid);
        assert!(result.is_err());
        assert_eq!(result.first_error_message(), Some("bad email"));
    }

    #[test]
    fn merge_both_invalid_combines_errors() {
        let a = Result::Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));
        let b = Result::Err(Errors(vec![Error {
            code: ErrorCode::Email,
            message: "bad email".to_string(),
        }]));
        let result = a.merge(b);
        assert_eq!(result.errors().expect("has errors").len(), 2);
    }

    #[test]
    fn first_error_message_valid() {
        let result = Result::Ok(());
        assert_eq!(result.first_error_message(), None);
    }

    #[test]
    fn first_error_message_invalid() {
        let result = Result::Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));
        assert_eq!(result.first_error_message(), Some("required"));
    }

    #[test]
    fn without_server_errors_keeps_client() {
        let result = Result::Err(Errors(vec![
            Error {
                code: ErrorCode::Required,
                message: "required".to_string(),
            },
            Error::server("duplicate"),
        ]));
        let filtered = result.without_server_errors();
        assert!(filtered.is_err());
        let errors = filtered.errors().expect("has errors");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors.0[0].code, ErrorCode::Required);
    }

    #[test]
    fn without_server_errors_removes_all_server() {
        let result = Result::Err(Errors(vec![
            Error::server("err1"),
            Error {
                code: ErrorCode::Async("check".to_string()),
                message: "async err".to_string(),
            },
        ]));
        let filtered = result.without_server_errors();
        assert!(filtered.is_ok());
    }

    #[test]
    fn without_server_errors_valid_stays_valid() {
        let result = Result::Ok(());
        assert!(result.without_server_errors().is_ok());
    }
}
