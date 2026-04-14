//! Testing helpers for validating ARIA output from connect surfaces.

mod validator;

pub use validator::{
    AriaValidationError, AriaValidationWarning, AriaValidator, required_attributes_for_role,
    validate_attr_map,
};
