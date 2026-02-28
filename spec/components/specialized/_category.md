# Specialized Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and `Service`,
`03-accessibility.md` for focus management, ARIA patterns, and keyboard navigation,
`04-internationalization.md` for locale handling, number formatting, and RTL support,
`07-forms.md` for form participation, validation, and hidden input patterns.

---

## Table of Contents

- [ColorPicker](color-picker.md)
- [FileUpload](file-upload.md)
- [SignaturePad](signature-pad.md)
- [ImageCropper](image-cropper.md)
- [QrCode](qr-code.md)
- [Clipboard](clipboard.md)
- [Timer](timer.md)
- [ContextualHelp](contextual-help.md)
- [ColorArea](color-area.md)
- [ColorSlider](color-slider.md)
- [ColorField](color-field.md)
- [ColorSwatch](color-swatch.md)
- [ColorSwatchPicker](color-swatch-picker.md)
- [ColorWheel](color-wheel.md)
- [AngleSlider](angle-slider.md)

---

## Overview

Specialized components address use cases that fall outside the standard input, selection,
overlay, and navigation categories. They typically involve richer interaction models
(color manipulation, file drag-and-drop, canvas drawing, image transformation), browser
API integration (clipboard, timers), or visual output generation (QR codes).

| Component           | Tier      | Value Type               | Machine States                      | Key Interaction                                 |
| ------------------- | --------- | ------------------------ | ----------------------------------- | ----------------------------------------------- |
| `ColorPicker`       | complex   | `ColorValue`             | Idle / Open / Dragging              | HSL area/channel dragging, eyedropper           |
| `FileUpload`        | complex   | `Vec<file_upload::Item>` | Idle / DragOver / Uploading         | Drag-and-drop, click-to-browse, upload progress |
| `SignaturePad`      | stateful  | `SignatureData`          | Idle / Drawing / Completed          | Pointer/touch drawing on canvas                 |
| `ImageCropper`      | stateful  | `CropArea`               | Idle / Dragging / Resizing          | Drag-to-move, handle-resize crop region         |
| `QrCode`            | stateless | `String` (input)         | —                                   | Declarative QR matrix rendering                 |
| `Clipboard`         | stateful  | `String`                 | Idle / Copying / Copied / Error     | Copy-to-clipboard with feedback                 |
| `Timer`             | stateful  | `Duration`               | Idle / Running / Paused / Completed | Countdown/stopwatch with tick                   |
| `ContextualHelp`    | stateless | —                        | — (composes Popover)                | Trigger icon + non-modal popover                |
| `ColorArea`         | stateful  | `ColorValue`             | Idle / Dragging                     | 2D two-channel color area                       |
| `ColorSlider`       | stateful  | `ColorValue`             | Idle / Dragging                     | 1D single-channel color slider                  |
| `ColorField`        | stateful  | `Option<ColorValue>`     | Idle / Focused                      | Text input for color values with format parsing |
| `ColorSwatch`       | stateless | `ColorValue` (input)     | —                                   | Non-interactive color preview                   |
| `ColorSwatchPicker` | stateful  | `ColorValue`             | Idle / Focused                      | Listbox of color swatches                       |
| `ColorWheel`        | stateful  | `ColorValue`             | Idle / Dragging                     | Circular hue drag, keyboard increment           |
| `AngleSlider`       | stateful  | `f64` (degrees)          | Idle / Dragging / Focused           | Circular angle drag, keyboard increment         |

### Color Component Composition

The color components form a composition hierarchy. `ColorPicker` is the top-level aggregate that composes the standalone primitives:

```text
ColorPicker (complex — orchestrates all below)
├── ColorArea          (2D channel surface)
├── ColorSlider        (1D channel slider, ×N per visible channel)
├── ColorField         (text input for hex/rgb/hsl or single channel)
├── ColorSwatchPicker  (preset color listbox)
│   └── ColorSwatch    (single swatch display)
├── ColorWheel         (circular hue selector)
└── AngleSlider        (circular angle control)
```

Each primitive has its own independent `Machine` and can be used standalone outside of `ColorPicker`. They all share `ColorValue` as their value type (except AngleSlider which uses `f64` degrees) and communicate through `Bindable<T>` value propagation — the parent `ColorPicker` holds the authoritative `Bindable<ColorValue>` and passes it down.

All specialized components follow the standard ars-ui architecture:

- Zero framework dependencies — all logic lives in `ars-core`.
- `Bindable<T>` handles controlled and uncontrolled values identically.
- Each component defines a `Part` enum with `#[derive(ComponentPart)]` and implements `ConnectApi` with `fn part_attrs()` dispatching to per-part `*_attrs()` methods returning `AttrMap`.
- Data attributes on every part (`data-ars-scope`, `data-ars-part`, `data-ars-state`, etc.) enable CSS-first styling.

---

## Data Attributes

| Component           | `data-ars-scope`      | Notable `data-ars-*` Attributes                                                                                                     |
| ------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `ColorPicker`       | `color-picker`        | `data-ars-state`, `data-ars-disabled`, `data-ars-readonly`, `data-ars-dragging`, `data-ars-channel`, `data-ars-selected`            |
| `FileUpload`        | `file-upload`         | `data-ars-state`, `data-ars-disabled`, `data-ars-dragging`, `data-ars-file-id`                                                      |
| `SignaturePad`      | `signature-pad`       | `data-ars-state`, `data-ars-disabled`, `data-ars-focus-visible`                                                                     |
| `ImageCropper`      | `image-cropper`       | `data-ars-state`, `data-ars-disabled`                                                                                               |
| `QrCode`            | `qr-code`             | (none beyond scope/part)                                                                                                            |
| `Clipboard`         | `clipboard`           | `data-ars-state`, `data-ars-disabled`                                                                                               |
| `Timer`             | `timer`               | `data-ars-state`                                                                                                                    |
| `ContextualHelp`    | `contextual-help`     | `data-ars-state`, `data-ars-variant`                                                                                                |
| `ColorArea`         | `color-area`          | `data-ars-disabled`, `data-ars-readonly`, `data-ars-dragging`, `data-ars-focus-visible`                                             |
| `ColorSlider`       | `color-slider`        | `data-ars-channel`, `data-ars-orientation`, `data-ars-disabled`, `data-ars-readonly`, `data-ars-dragging`, `data-ars-focus-visible` |
| `ColorField`        | `color-field`         | `data-ars-disabled`, `data-ars-readonly`, `data-ars-invalid`, `data-ars-focused`, `data-ars-focus-visible`                          |
| `ColorSwatch`       | `color-swatch`        | `data-ars-alpha`                                                                                                                    |
| `ColorSwatchPicker` | `color-swatch-picker` | `data-ars-state`, `data-ars-disabled`, `data-ars-selected`, `data-ars-focused`, `data-ars-focus-visible`                            |
| `ColorWheel`        | `color-wheel`         | `data-ars-disabled`, `data-ars-readonly`, `data-ars-dragging`, `data-ars-focus-visible`                                             |
| `AngleSlider`       | `angle-slider`        | `data-ars-state`, `data-ars-disabled`, `data-ars-readonly`, `data-ars-dragging`, `data-ars-focus-visible`                           |

## I18n Message Keys

| Component           | Key Prefix              | Message Count |
| ------------------- | ----------------------- | ------------- |
| `ColorPicker`       | `color_picker.*`        | 8             |
| `FileUpload`        | `file_upload.*`         | 16            |
| `SignaturePad`      | `signature_pad.*`       | 7             |
| `ImageCropper`      | `image_cropper.*`       | 7             |
| `QrCode`            | `qr_code.*`             | 2             |
| `Clipboard`         | `clipboard.*`           | 6             |
| `Timer`             | `timer.*`               | 6             |
| `ContextualHelp`    | `contextual_help.*`     | 3             |
| `ColorArea`         | `color_area.*`          | 3             |
| `ColorSlider`       | `color_slider.*`        | 2             |
| `ColorField`        | `color_field.*`         | 4             |
| `ColorSwatch`       | `color_swatch.*`        | 1             |
| `ColorSwatchPicker` | `color_swatch_picker.*` | 1             |
| `ColorWheel`        | `color_wheel.*`         | 2             |
| `AngleSlider`       | `angle_slider.*`        | 2             |

## Dependencies

All specialized components depend on:

- `ars-core`: `Machine`, `Bindable`, `AttrMap`, `Transition`, `Action`, `Effect`
- `ars-a11y`: ARIA attribute types, focus management

Additional per-component dependencies:

| Component           | Additional Crates                                                        |
| ------------------- | ------------------------------------------------------------------------ |
| `ColorPicker`       | `ars-dom` (positioning, click-outside), `ars-i18n` (number formatting)   |
| `FileUpload`        | `ars-dom` (file input, drag-and-drop), `ars-i18n` (file size formatting) |
| `SignaturePad`      | `ars-dom` (canvas API, global pointer listeners)                         |
| `ImageCropper`      | `ars-dom` (pointer listeners, canvas for export)                         |
| `QrCode`            | External QR encoding library (e.g., `qrcode-generator`)                  |
| `Clipboard`         | `ars-dom` (navigator.clipboard API)                                      |
| `Timer`             | `ars-dom` (setInterval/setTimeout timers)                                |
| `ContextualHelp`    | `ars-dom` (positioning, click-outside)                                   |
| `ColorArea`         | `ars-dom` (global pointer listeners)                                     |
| `ColorSlider`       | `ars-dom` (global pointer listeners)                                     |
| `ColorField`        | (none — pure computation, color parsing in `ars-core`)                   |
| `ColorSwatch`       | (none — pure computation)                                                |
| `ColorSwatchPicker` | (none — pure computation)                                                |
| `ColorWheel`        | `ars-dom` (global pointer listeners)                                     |
| `AngleSlider`       | `ars-dom` (global pointer listeners)                                     |
