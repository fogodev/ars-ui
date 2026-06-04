//! Ignored nightly property-based tests for `crates/ars-components/src/specialized/*`.
//!
//! Stateful and complex components own sibling modules containing their
//! arbitraries and `proptest!` blocks. Stateless components keep modules here
//! when they have meaningful prop-to-attribute invariants to exercise.

mod angle_slider;
mod clipboard;
mod color_area;
mod color_field;
mod color_picker;
mod color_slider;
mod color_swatch;
mod color_swatch_picker;
mod color_wheel;
mod contextual_help;
mod file_upload;
mod image_cropper;
mod qr_code;
mod signature_pad;
mod timer;
