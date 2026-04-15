//! Testing helpers for validating ARIA output from connect surfaces.

mod keyboard;
mod validator;

pub use keyboard::{FocusZoneTestHarness, NavigationEvent, NavigationRecorder, SimulatedKeyEvent};
pub use validator::{
    AriaValidationContext, AriaValidationError, AriaValidationWarning, AriaValidator,
    required_attributes_for_role, validate_attr_map,
};
