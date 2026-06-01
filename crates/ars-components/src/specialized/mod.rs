//! Specialized component machines.

/// `AngleSlider` circular angle-input machine and connect API.
pub mod angle_slider;

/// Clipboard component machine.
pub mod clipboard;

/// `ColorArea` 2D color-picker machine and connect API.
pub mod color_area;

/// `ColorField` color-value text input machine and connect API.
pub mod color_field;

/// `ColorPicker` complex orchestrator machine and connect API.
pub mod color_picker;

/// `ColorSlider` 1D single-channel color slider machine and connect API.
pub mod color_slider;

/// `ColorSwatch` stateless color-preview connect API.
pub mod color_swatch;

/// `ColorSwatchPicker` listbox-of-swatches machine and connect API.
pub mod color_swatch_picker;

/// `ColorWheel` circular hue-picker machine and connect API.
pub mod color_wheel;

/// Contextual help composition API over [`crate::overlay::popover`].
pub mod contextual_help;

/// File upload component machine.
pub mod file_upload;

/// `QrCode` stateless QR-matrix rendering connect API.
pub mod qr_code;
