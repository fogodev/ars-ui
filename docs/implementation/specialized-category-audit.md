# Specialized Category Audit

This audit covers every framework-agnostic component under
`spec/components/specialized/` after Epic #227. Adapter components are out of
scope for this pass; adapter spec references were checked only to confirm that
the agnostic component specs remain registered and discoverable.

## Disposition

| Component         | Implementation surface                             | Audit disposition                                                                                                                                                                      | Added or preserved tests                                                                                                 |
| ----------------- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| AngleSlider       | `ars-components::specialized::angle_slider`        | Matches agnostic state, keyboard, ARIA, anatomy, and message contract.                                                                                                                 | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| Clipboard         | `ars-components::specialized::clipboard`           | Matches state/effect contract; test surface was missing category-owned proptest coverage.                                                                                              | Anatomy test moved; event-sequence proptest added for state/data-attr/dispatch invariants.                               |
| ColorArea         | `ars-components::specialized::color_area`          | Matches color-area state, channel bounds, thumb ARIA, and hidden-input contract.                                                                                                       | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| ColorField        | `ars-components::specialized::color_field`         | Matches whole-color/channel input behavior, IME suppression, invalid handling, and hidden-input semantics; test surface was missing category-owned proptest coverage.                  | Anatomy test moved; event-sequence proptest added for value, spinbutton, invalid, and hidden-input invariants.           |
| ColorPicker       | `ars-components::specialized::color_picker`        | Matches composite color-picker state, composed color primitive semantics, open/drag effects, and anatomy.                                                                              | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| ColorSlider       | `ars-components::specialized::color_slider`        | Matches one-dimensional channel slider state, channel bounds, and thumb ARIA contract.                                                                                                 | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| ColorSwatch       | `ars-components::specialized::color_swatch`        | Stateless connect API matches accessible color-name and alpha rendering contract.                                                                                                      | Anatomy test moved; prop-to-attr proptest added.                                                                         |
| ColorSwatchPicker | `ars-components::specialized::color_swatch_picker` | Matches listbox-style swatch selection/focus state and hidden-input anatomy.                                                                                                           | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| ColorWheel        | `ars-components::specialized::color_wheel`         | Matches hue-wheel state, range invariants, and thumb ARIA contract.                                                                                                                    | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| ContextualHelp    | `ars-components::specialized::contextual_help`     | Composition over `popover` matches variant labeling, dialog content attrs, direction, and dismiss controls.                                                                            | Anatomy test moved; prop-to-popover/attr proptest added.                                                                 |
| FileUpload        | `ars-components::specialized::file_upload`         | Matches queue ownership, validation, drag/upload state reconciliation, file-item anatomy, and hidden native input contract; test surface was missing category-owned proptest coverage. | Anatomy test moved; event-sequence proptest added for queue, rejection, progress, dropzone, and hidden-input invariants. |
| ImageCropper      | `ars-components::specialized::image_cropper`       | Matches crop geometry, zoom/rotation/flip behavior, handle anatomy, and keyboard/focus attrs.                                                                                          | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| QrCode            | `ars-components::specialized::qr_code`             | Stateless connect API matches QR pattern, overlay, sizing, URL-aware label, and download trigger contract.                                                                             | Existing stateless prop-to-attr proptests moved to component module.                                                     |
| SignaturePad      | `ars-components::specialized::signature_pad`       | Matches drawing lifecycle, stroke invariants, export behavior, guide/hidden-input attrs, and anatomy.                                                                                  | Existing spec-conformance and proptest coverage moved to component modules.                                              |
| Timer             | `ars-components::specialized::timer`               | Matches countdown/stopwatch states, tick/reset effects, progress, and trigger anatomy.                                                                                                 | Existing spec-conformance and proptest coverage moved to component modules.                                              |

## Findings Landed

| Finding                                                                                                                                                            | Disposition                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Category-level spec-conformance file had all specialized anatomy tests in one blob.                                                                                | Split into `tests/spec_conformance/specialized/mod.rs` plus one file per component.                |
| Category-level proptest file had all specialized property tests in one blob.                                                                                       | Split into `tests/proptest_state_machines/specialized/mod.rs` plus one file per covered component. |
| Clipboard lacked specialized proptest coverage.                                                                                                                    | Added an ignored nightly event-sequence proptest for state and attr invariants.                    |
| ColorField lacked specialized proptest coverage.                                                                                                                   | Added an ignored nightly event-sequence proptest for channel/whole-field invariants.               |
| FileUpload lacked specialized proptest coverage.                                                                                                                   | Added an ignored nightly event-sequence proptest for queue and DOM-attribute invariants.           |
| Stateless ColorSwatch and compositional ContextualHelp needed explicit category visibility in the proptest tree.                                                   | Added prop-to-attr / prop-to-popover proptests instead of empty placeholder modules.               |
| The specialized lib coverage report exposed uncovered `FileUpload` public API dispatchers, item subpart attrs, validation-message arms, and `ConnectApi` dispatch. | Added focused unit coverage for those spec-facing paths.                                           |

## Audit Notes

- `cargo xtask spec info <component>` resolves all 15 specialized specs and their adapter spec references.
- `cargo xtask spec validate` validates the component/adapters registry with all specialized specs present.
- Search for `TODO`, `FIXME`, `unimplemented!`, `todo!`, `stub`, and `placeholder` in specialized specs, implementations, and moved tests found only prose/test false positives: placeholder wording in docs, an expected stub-rasterizer test assertion, and explicit panic assertions in tests.
- The focused `cargo llvm-cov` report for `specialized::` was reviewed. Remaining uncovered specialized lines are existing low-value debug/test-helper or defensive branches in already-covered components; the actionable `FileUpload` public API gaps were fixed here.
- No implementation/spec drift requiring production-code or spec-contract changes was found in this pass.

## Test Layout

The specialized test category now uses directory-backed modules:

- `crates/ars-components/tests/spec_conformance/specialized/mod.rs`
- `crates/ars-components/tests/proptest_state_machines/specialized/mod.rs`

Each component has a spec-conformance module. Components with meaningful
state-machine or prop-to-attribute invariants have a proptest module.
