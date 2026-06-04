//! Spec-conformance tests for `crates/ars-components/src/specialized/*`.
//!
//! Each component module asserts the impl's `Part` enum matches the spec's
//! declared anatomy.

use ars_components::specialized as specialized_core;

use super::helper::assert_anatomy;

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
