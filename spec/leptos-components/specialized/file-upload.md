---
adapter: leptos
component: file-upload
category: specialized
source: components/specialized/file-upload.md
source_foundation: foundation/08-adapter-leptos.md
---

# FileUpload — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`FileUpload`](../../components/specialized/file-upload.md) contract onto Leptos `0.8.x`. The adapter preserves dropzone selection, hidden native file input bridging, validation and rejection reporting, upload progress wiring, and drag-and-drop announcements.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn FileUpload(
    #[prop(optional)] files: Option<RwSignal<Vec<file_upload::Item>>>,
    #[prop(optional)] default_files: Vec<file_upload::Item>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] multiple: bool,
    #[prop(optional)] accept: Vec<String>,
    #[prop(optional)] max_file_size: Option<u64>,
    #[prop(optional)] min_file_size: Option<u64>,
    #[prop(optional)] max_files: Option<usize>,
    #[prop(optional)] auto_upload: bool,
    #[prop(optional)] directory: bool,
) -> impl IntoView
```

The adapter renders the full dropzone surface, item list, progress indicators, and native hidden file input.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable file list, validation rules, disabled/read-only state, and upload policy.
- Part parity: full parity with `Root`, `Label`, `Dropzone`, `Trigger`, `ItemGroup`, repeated item parts, and `HiddenInput`.
- Adapter additions: explicit drag-and-drop plumbing, native file-input bridging, and live-region announcement policy.

## 4. Part Mapping

`Root`, `Label`, `Dropzone`, `Trigger`, `ItemGroup`, repeated `Item`, `ItemName`, `ItemSizeText`, `ItemDeleteTrigger`, `ItemProgress`, and `HiddenInput` each render as adapter-owned DOM nodes. `Dropzone` and `HiddenInput` are the two browser-API integration surfaces.

## 5. Attr Merge and Ownership Rules

Dropzone button semantics, item-list semantics, item removal labels, and native file-input attrs always win. Consumer decoration must not remove `tabindex`, list semantics, or the hidden input.

## 6. Composition / Context Contract

`FileUpload` is a single integrated surface. Consumers do not render item parts separately in the base adapter contract.

## 7. Prop Sync and Event Mapping

Controlled file-list sync flows through the writable signal. `Dropzone` drag events dispatch enter/leave/over/drop machine events. `Trigger` and `Dropzone` keyboard activation forward to the hidden input. Native input change events normalize `FileList` data into core `RawFile` values before validation.

## 8. Registration and Cleanup Contract

Register drag-over nesting state, upload handles, and live-region work per component instance. Cleanup must cancel any in-flight upload bookkeeping owned by the adapter, clear drag state, and detach the hidden-input event bridge on unmount.

## 9. Ref and Node Contract

The hidden input requires a live node for programmatic click. `Dropzone` may require a live node for focus and drag boundary handling. No consumer refs are part of the base surface.

## 10. State Machine Boundary Rules

- machine-owned state: files, rejections, drag-over state, focused part, and upload status per item.
- adapter-local derived bookkeeping: native input node handle, drag nesting counter, and any upload transport handles only.
- forbidden local mirrors: do not keep a second file list outside the machine binding.

## 11. Callback Payload Contract

No dedicated public callback is required beyond the bindable file list and the rendered item state.

## 12. Failure and Degradation Rules

If drag-and-drop is unavailable, degrade gracefully to trigger-plus-hidden-input selection only. If directory upload or capture attributes are unsupported, ignore that capability while preserving basic file selection.

## 13. Identity and Key Policy

Each file item uses its machine-defined file id as the stable identity. Item order must remain consistent with the core file list.

## 14. SSR and Client Boundary Rules

SSR renders the structural dropzone, item list, and hidden input. Drag-and-drop, native file selection, upload progress, and live announcements are client-only.

## 15. Performance Constraints

Do not rebuild the whole list when only one item’s progress changes. Keep drag-over announcements and upload bookkeeping instance-scoped and cancellable.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type      | Why it must exist first                                            | Notes                            |
| ------------------------ | ----------- | -------------------- | ------------------------------------------------------------------ | -------------------------------- |
| hidden file-input helper | required    | browser helper       | native browsing path depends on a real file input                  | adapter-owned bridge             |
| DnD normalization helper | required    | browser helper       | dropped files and selected files must normalize into one core path | shared with drop surfaces        |
| live-region helper       | recommended | accessibility helper | drag and rejection announcements should stay consistent            | reuse from announcing components |

## 17. Recommended Implementation Sequence

1. Render the structural dropzone, trigger, item list, and hidden input.
2. Wire hidden-input selection into the core file path.
3. Add drag-and-drop handling and announcements.
4. Add upload progress and retry/cancel bookkeeping.

## 18. Anti-Patterns

- Do not treat drag-and-drop and hidden-input selection as separate validation pipelines.
- Do not leave upload or announcement handles alive after file removal or unmount.

## 19. Consumer Expectations and Guarantees

- Consumers may assume hidden-input browsing remains available even when drag-and-drop is unsupported.
- Consumers may assume the rendered list order matches the current file list.
- Consumers must not assume upload transport is provided by the adapter beyond the documented state wiring.

## 20. Platform Support Matrix

| Capability / behavior                        | Browser client | SSR            | Notes                    |
| -------------------------------------------- | -------------- | -------------- | ------------------------ |
| selection, list rendering, and validation UI | full support   | full support   | structural parity on SSR |
| drag-and-drop and upload progress            | full support   | client-only    | browser runtime only     |
| directory or capture hints                   | fallback path  | not applicable | ignored when unsupported |

## 21. Debug Diagnostics and Production Policy

Missing hidden-input bridge or duplicate item identity is fail-fast. Unsupported DnD capabilities are debug-warning and graceful fallback.

## 22. Shared Adapter Helper Notes

Use one hidden-input bridge, one DnD normalization helper, and one live-region helper so selection, drop, and announcement paths stay aligned.

## 23. Framework-Specific Behavior

Leptos should keep the hidden input mounted, trigger its click from the documented gestures only, and clean up any upload-side resources in effect cleanup before state reset.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn FileUpload() -> impl IntoView {
    let machine = use_machine::<file_upload::Machine>(file_upload::Props::default());
    view! { <div {..machine.derive(|api| api.root_attrs()).get()} /> }
}
```

## 25. Reference Implementation Skeleton

- Bind the file list.
- Normalize hidden-input and dropped files through the same adapter helper.
- Render item list state from the machine only.
- Clean up upload and announcement resources eagerly.

## 26. Adapter Invariants

- Hidden-input selection and DnD selection share one normalization and validation path.
- Item identity stays stable while the file remains present.
- Adapter-owned upload and announcement resources never survive unmount.

## 27. Accessibility and SSR Notes

`Dropzone` remains keyboard-activatable and labeled. Announcements for drops, rejections, and status changes remain additive to the visible list rather than replacing it.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- hidden-input file selection
- drag-enter/leave/drop behavior with nested drag counters
- validation rejection and announcement path
- item removal and progress updates
- unsupported DnD fallback to trigger-only selection

## 30. Test Oracle Notes

| Behavior                  | Preferred oracle type | Notes                                           |
| ------------------------- | --------------------- | ----------------------------------------------- |
| list and button semantics | DOM attrs             | assert dropzone, list, and item attrs           |
| hidden-input bridge       | mocked file input     | assert click and change normalization           |
| cleanup                   | cleanup side effects  | assert upload/announcement handles are canceled |

## 31. Implementation Checklist

- [ ] Hidden-input and DnD selection use one normalization path.
- [ ] Item identity and list order stay stable.
- [ ] Adapter-owned upload and announcement cleanup is explicit.
