# Utility Category Audit

This audit covers every entry under `spec/components/utility/` after epic #10.
The implementation disposition is recorded here so the category-level test split
has an explicit component-by-component audit trail.

## Disposition

| Component       | Implementation surface                                                       | Audit disposition                                                                     | Added or preserved tests                                           |
| --------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| ActionGroup     | `ars-components::utility::action_group`                                      | Matches agnostic anatomy/state-machine coverage.                                      | Existing spec-conformance and proptest moved to component modules. |
| ArsProvider     | `ars-core::ArsContext`, `ars-leptos::ArsProvider`, `ars-dioxus::ArsProvider` | Adapter-owned/context provider; no `ars-components` module needed.                    | Core provider default contract test added.                         |
| AsChild         | `ars-components::utility::as_child`                                          | Pattern primitive; no `Part`/`ConnectApi` expected.                                   | Props/merge spec-conformance and proptest coverage added.          |
| Button          | `ars-components::utility::button`                                            | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| ClientOnly      | `ars-components::utility::client_only`                                       | Logical boundary; no `Part`/`ConnectApi` expected.                                    | Props/default contract and fallback proptest added.                |
| Dismissable     | `ars-components::utility::dismissable`                                       | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance and dispatch proptest added.              |
| DownloadTrigger | `ars-components::utility::download_trigger`                                  | Existing coverage matched the stateless download contract.                            | Existing spec-conformance and proptest moved to component modules. |
| DropZone        | `ars-components::utility::drop_zone`                                         | Existing coverage matched current spec attrs, including stale issue regression guard. | Existing spec-conformance and proptest moved to component modules. |
| ErrorBoundary   | `ars-components::utility::error_boundary`                                    | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| Field           | `ars-components::utility::field`                                             | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| Fieldset        | `ars-components::utility::fieldset`                                          | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| FocusRing       | `ars-components::utility::focus_ring`                                        | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance and focus-visible proptest added.         |
| FocusScope      | `ars-components::utility::focus_scope`                                       | Existing coverage matched state/event/effect and attrs contract.                      | Existing spec-conformance and proptest moved to component modules. |
| Form            | `ars-components::utility::form`                                              | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| FormSubmit      | `ars-components::utility::form_submit`                                       | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| Group           | `ars-components::utility::group`                                             | Existing anatomy coverage preserved; proptest was missing.                            | Group context proptest added.                                      |
| Heading         | `ars-components::utility::heading`                                           | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance and level-clamping proptest added.        |
| Highlight       | `ars-components::utility::highlight`                                         | Existing i18n-gated coverage matched chunking invariants.                             | Existing spec-conformance and proptest moved to component modules. |
| Keyboard        | `ars-components::utility::keyboard`                                          | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance and decorative aria proptest added.       |
| Landmark        | `ars-components::utility::landmark`                                          | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance and labelledby-precedence proptest added. |
| LiveRegion      | `ars-components::utility::live_region`                                       | Existing coverage matched state-machine queue invariants.                             | Existing spec-conformance and proptest moved to component modules. |
| Separator       | `ars-components::utility::separator`                                         | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| Swap            | `ars-components::utility::swap`                                              | Existing coverage matched state-machine invariants.                                   | Existing spec-conformance and proptest moved to component modules. |
| Toggle          | `ars-components::utility::toggle`                                            | Existing coverage matched state-machine invariants.                                   | Existing spec-conformance and proptest moved to component modules. |
| ToggleButton    | `ars-components::utility::toggle_button`                                     | Anatomy coverage existed; proptest was missing.                                       | Event-sequence proptest added with value-sync semantics respected. |
| ToggleGroup     | `ars-components::utility::toggle_group`                                      | Existing coverage matched registration/selection invariants.                          | Existing spec-conformance and proptest moved to component modules. |
| VisuallyHidden  | `ars-components::utility::visually_hidden`                                   | Anatomy coverage was missing from spec-conformance.                                   | Anatomy spec-conformance added; proptest preserved.                |
| ZIndexAllocator | `ars-components::utility::z_index_allocator`                                 | Context provider; no `Part`/`ConnectApi` expected.                                    | Props/context contract and monotonic allocation proptest added.    |

## Test Layout

The utility test category now uses directory-backed modules:

- `crates/ars-components/tests/spec_conformance/utility/mod.rs`
- `crates/ars-components/tests/proptest_state_machines/utility/mod.rs`

Each utility component has a sibling test module in both trees, even when the
component has no meaningful proptest surface today.
