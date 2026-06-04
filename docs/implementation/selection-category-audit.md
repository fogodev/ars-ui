# Selection Category Audit

This audit covers every agnostic component under `spec/components/selection/`
for Epic #221. The audit scope is framework-agnostic core only:
`crates/ars-components/src/selection/*`, selection component specs, shared
selection-pattern contracts, and the category-level spec-conformance/proptest
test layout.

## Disposition

| Component    | Implementation surface                     | Audit disposition                                                                                        | Added or preserved tests                                                                    |
| ------------ | ------------------------------------------ | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Autocomplete | `ars-components::selection::autocomplete`  | Matches state, event, part, loading, debounce, filtering, and active-descendant contracts.               | Anatomy spec-conformance moved to module; active proptest preserved and broadened.          |
| Combobox     | `ars-components::selection::combobox`      | Public event list was ahead of spec for pointer-down, inline-completion, description, and prop sync.     | Anatomy spec-conformance moved to module; ignored proptest broadened for public events.     |
| ContextMenu  | `ars-components::selection::context_menu`  | Matches menu-derived action, submenu, checkbox/radio, pointer anchor, and target anatomy contracts.      | Anatomy spec-conformance moved to module; ignored proptest broadened for sync/update.       |
| Listbox      | `ars-components::selection::listbox`       | Public event list was ahead of spec for range extension, page navigation, description, and prop sync.    | Anatomy spec-conformance moved to module; ignored proptest broadened for public events.     |
| Menu         | `ars-components::selection::menu`          | Public event list was ahead of spec for prop sync; action, checkbox/radio, submenu, and typeahead match. | Anatomy spec-conformance moved to module; ignored proptest broadened for sync/update.       |
| MenuBar      | `ars-components::selection::menu_bar`      | Matches top-level menu collection, orientation-aware traversal, active menu, and part contracts.         | Anatomy spec-conformance moved to module; ignored proptest broadened for sync/update.       |
| SegmentGroup | `ars-components::selection::segment_group` | Controlled value sync accepted disabled values before this audit; fixed to sanitize external values.     | Anatomy spec-conformance moved to module; active proptest broadened; unit regression added. |
| Select       | `ars-components::selection::select`        | Public event list was ahead of spec for description and prop sync; selection/form semantics match.       | Anatomy spec-conformance moved to module; ignored proptest broadened for public events.     |
| TagsInput    | `ars-components::selection::tags_input`    | Matches tag editing, composition, hidden-input, live-region, and controlled value contracts.             | Anatomy spec-conformance moved to module; ignored proptest broadened for value/prop sync.   |

## Findings Landed

| Finding | Severity | Best outcome                                                                                           | Disposition                                                                                                       |
| ------- | -------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| S1      | Medium   | Keep the implementation event APIs and update component specs to document them.                        | Updated Combobox, Listbox, Menu, and Select specs for missing public event variants.                              |
| S2      | Medium   | Property tests should generate every public state-machine event that can preserve invariants.          | Broadened per-component proptest strategies for sync, collection update, range/page, and controlled-value events. |
| S3      | High     | Externally controlled SegmentGroup values must obey the same enabled-item invariant as user selection. | Fixed `SetValue` sanitization and added an explicit unit regression.                                              |

## Test Layout

Selection tests now use directory-backed modules:

- `crates/ars-components/tests/spec_conformance/selection/mod.rs`
- `crates/ars-components/tests/proptest_state_machines/selection/mod.rs`

Each of the 9 selection components has its own spec-conformance module and its
own proptest module. Shared proptest fixtures live in
`crates/ars-components/tests/proptest_state_machines/selection/common.rs`.

The previous ignored/non-ignored proptest behavior is preserved: Autocomplete
and SegmentGroup run in the regular test target, while the larger collection
state-machine proptests remain ignored for extended/nightly-style runs.

## Audit Notes

- Event enum names now match between all 9 component specs and implementations.
- Part contracts match the implementations and are covered by per-component
  anatomy tests. ContextMenu and MenuBar declare their parts in prose rather
  than a Rust `Part` snippet, but the declared part lists match the shipped API.
- Adapter components remain out of scope for this audit; Leptos and Dioxus
  selection epics track that work separately.
