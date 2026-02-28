---
adapter: dioxus
component: drop-zone
category: utility
source: components/utility/drop-zone.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DropZone — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DropZone`](../../components/utility/drop-zone.md) machine to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct DropZoneProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub accept: Option<Vec<String>>,
    #[props(optional)]
    pub max_files: Option<usize>,
    #[props(optional)]
    pub max_file_size: Option<u64>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub label: Option<String>,
    #[props(optional)]
    pub allowed_operations: Option<Vec<DropOperation>>,
    #[props(optional)]
    pub name: Option<String>,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(default = false)]
    pub read_only: bool,
    #[props(default = 500)]
    pub activate_delay_ms: u32,
    #[props(optional)]
    pub on_drop: Option<EventHandler<Vec<DragItem>>>,
    #[props(optional)]
    pub on_drop_enter: Option<EventHandler<DragData>>,
    #[props(optional)]
    pub on_drop_exit: Option<EventHandler<()>>,
    #[props(optional)]
    pub on_drop_activate: Option<EventHandler<()>>,
    pub children: Element,
}

#[component]
pub fn DropZone(props: DropZoneProps) -> Element
```

The adapter surfaces the full core prop set including accept filters, limits, label, operations, form props, locale/messages, and callbacks.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core drop-zone props.
- Event parity: drag, drop, focus, blur, reset, and delayed activation are adapter-driven.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target               | Ownership      | Attr source        | Notes                                                            |
| --------------------- | --------- | -------------------------------------- | -------------- | ------------------ | ---------------------------------------------------------------- |
| `Root`                | required  | wrapper `<div>` or drop target element | adapter-owned  | `api.root_attrs()` | Carries role, tabindex, invalid/read-only state, and data attrs. |
| children content      | required  | consumer children inside `Root`        | consumer-owned | none               | Visible instructional or status content.                         |

## 5. Attr Merge and Ownership Rules

| Target node                                  | Core attrs                            | Adapter-owned attrs             | Consumer attrs                 | Merge order                                                                        | Ownership notes    |
| -------------------------------------------- | ------------------------------------- | ------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------- | ------------------ |
| `Root`                                       | drag-and-drop attrs from the core API | temporary drag-state data attrs | consumer root attrs            | core drag acceptance and accessibility attrs win; `class`/`style` merge additively | adapter-owned root |
| optional hidden form-participation structure | core submission attrs when supported  | structural hidden-input attrs   | none unless explicitly exposed | core attrs win                                                                     | adapter-owned      |

## 6. Composition / Context Contract

No required external context. Form helpers may consume dropped data from a wrapper context if provided.

## 7. Prop Sync and Event Mapping

Drop-zone configuration props are usually non-reactive after mount unless a wrapper reinitializes drop behavior.

| Adapter prop                | Mode                      | Sync trigger     | Machine event / update path                                          | Visible effect                                         | Notes                                      |
| --------------------------- | ------------------------- | ---------------- | -------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------ |
| accepted kinds / options    | non-reactive adapter prop | render time only | initial machine props                                                | determines accepted drag payloads                      | dynamic changes require reinitialization   |
| `name` / form participation | non-reactive adapter prop | render time only | adapter form-bridge setup reads `api.form_data()` during form submit | dropped payload is appended to `FormData` when enabled | disabled state returns an empty form slice |

| UI event           | Preconditions                                                   | Machine event / callback path                                                               | Ordering notes                                                          | Notes                                                                  |
| ------------------ | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `dragenter`        | acceptable drag candidate                                       | drag-enter path                                                                             | normalization runs before consumer callbacks                            | may set active styling                                                 |
| `dragover`         | active drag candidate                                           | drag-over path and `dropEffect` update                                                      | browser prevention must happen before callbacks when accepting the drop | controls drop affordance                                               |
| `dragleave`        | previously active drag leaves root                              | drag-leave path                                                                             | may clear active styling after containment check                        | nested-target handling must be normalized                              |
| `drop`             | acceptable drop and root active                                 | drop path then public callback                                                              | normalization and payload extraction run before public callback         | payload is finalized before callback                                   |
| reset/clear action | payload currently stored                                        | clear path                                                                                  | clear occurs before notification-only callbacks                         | removes stored payload state                                           |
| parent form submit | `name` set and the drop-zone participates in form serialization | adapter form-submit bridge reads `api.form_data()` and appends accepted items to `FormData` | serialization runs after current machine payload is finalized           | read-only preserves serialization; disabled contributes an empty slice |

## 8. Registration and Cleanup Contract

- Temporary drag state exists only while a drag session is active.
- Any stored object references, file handles, or preview URLs must be released on clear and on component cleanup.
- If hidden form-participation structure exists, it is owned by the drop-zone instance and removed with it.
- When `name` is set, the adapter-owned form bridge must append `api.form_data()` into the parent form's `FormData` and must send the reset path when the parent form resets.

| Registered entity         | Registration trigger       | Identity key       | Cleanup trigger                            | Cleanup action                           | Notes                                                 |
| ------------------------- | -------------------------- | ------------------ | ------------------------------------------ | ---------------------------------------- | ----------------------------------------------------- |
| active drag session state | first accepted `dragenter` | drop-zone instance | `drop`, `dragleave` final exit, or cleanup | clear drag-active bookkeeping            | prevents stuck hover state                            |
| stored payload resources  | successful drop            | payload identity   | reset, replacement, or cleanup             | release object references / preview URLs | platform-specific release stays in framework behavior |

## 9. Ref and Node Contract

| Target part / node                      | Ref required?     | Ref owner     | Node availability    | Composition rule                         | Notes                                                     |
| --------------------------------------- | ----------------- | ------------- | -------------------- | ---------------------------------------- | --------------------------------------------------------- |
| `Root` drop target                      | yes               | adapter-owned | required after mount | compose only if wrappers expose the root | Drag events require a live target node.                   |
| optional hidden form-participation node | yes when rendered | adapter-owned | required after mount | no composition                           | Form submission helpers belong to the drop-zone instance. |

## 10. State Machine Boundary Rules

- machine-owned state: accepted/rejected drop semantics, stored dropped payload, invalid/readonly state, and normalized drag lifecycle.
- adapter-local derived bookkeeping: transient drag-enter depth, object URL handles, and root-node handle.
- forbidden local mirrors: do not keep a second accepted-payload state outside the machine.
- allowed snapshot-read contexts: drag/drop handlers, render derivation, and cleanup for payload resources.

## 11. Callback Payload Contract

| Callback                          | Payload source             | Payload shape                                                   | Timing                                                                   | Cancelable? | Notes                                                 |
| --------------------------------- | -------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------ | ----------- | ----------------------------------------------------- |
| drop callback when exposed        | normalized adapter payload | `{ accepted_items, rejected_items, operation, original_event }` | after payload normalization and before observational follow-up callbacks | no          | Payload must already reflect accept-filter decisions. |
| clear/reset callback when exposed | machine-derived snapshot   | `{ had_payload: bool }`                                         | after canonical clear/reset                                              | no          | Observational only.                                   |

## 12. Failure and Degradation Rules

| Condition                                                                   | Policy             | Notes                                                                                                        |
| --------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------ |
| browser drag APIs unavailable                                               | degrade gracefully | Render the inert root and skip drag behavior.                                                                |
| unsupported payload kind                                                    | warn and ignore    | Preserve the drop zone while rejecting the unsupported payload.                                              |
| root ref missing after mount                                                | fail fast          | Drag interaction requires a concrete target node.                                                            |
| Dioxus target lacks web `DataTransfer` but provides native file-drop events | degrade gracefully | Normalize native Desktop or Mobile drop payloads into the shared `DragData` shape instead of failing closed. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?                | DOM order must match registration order? | SSR/hydration stability                                          | Notes                                                 |
| -------------------------------- | ---------------- | ---------------------------------- | ---------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------- |
| stored payload resources         | composite        | yes for distinct payload resources | not applicable                           | SSR renders no live payload handles                              | Identity is payload identity plus drop-zone instance. |
| hidden form-participation node   | instance-derived | not applicable                     | not applicable                           | hidden node structure must remain stable when initially rendered | Owned by the drop-zone instance.                      |

## 14. SSR and Client Boundary Rules

- SSR must render the inert root structure and any hidden form-participation structure required by the initial state.
- Drag events, object URLs, and payload handles are client-only.
- Root refs are server-safe absent and required after mount.

## 15. Performance Constraints

- Drag-enter/depth bookkeeping should be instance-local and must not allocate shared listeners.
- Object URLs or preview resources must only be created when accepted payloads actually change.
- Reset/clear should release resources incrementally instead of rebuilding the whole component subtree.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type  | Why it must exist first                                                 | Notes                                                          |
| ---------- | ----------- | ---------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------- |
| `form`     | recommended | context contract | Hidden submission structure should align with form participation rules. | Only relevant when dropped payloads participate in submission. |

## 17. Recommended Implementation Sequence

1. Establish the root ref and inert SSR structure.
2. Wire drag-enter, drag-over, drag-leave, and drop normalization.
3. Normalize accepted/rejected payloads and create any temporary resources.
4. Add reset/clear behavior and form-participation wiring if enabled.
5. Verify cleanup of stored resources and drag-active state.

## 18. Anti-Patterns

- Do not leak object URLs or stored payload resources across clears or unmount.
- Do not trust raw transfer data as the normalized accepted payload.
- Do not run drag-specific behavior on the server.

## 19. Consumer Expectations and Guarantees

- Consumers may assume normalized drop payloads have already passed through the adapter-specific drag-source abstraction.
- Consumers may assume reset or form-bridge cleanup clears adapter-owned retained payload resources.
- Consumers must not assume raw browser `DataTransfer` or native desktop payloads are forwarded unchanged.

## 20. Platform Support Matrix

| Capability / behavior  | Web          | Desktop       | Mobile        | SSR            | Notes                                                                     |
| ---------------------- | ------------ | ------------- | ------------- | -------------- | ------------------------------------------------------------------------- |
| drop payload ingestion | full support | fallback path | fallback path | client-only    | Web maps DOM drag events; Desktop and Mobile may map native drop sources. |
| form-data bridge       | full support | fallback path | fallback path | SSR-safe empty | Bridged payloads only exist after client/runtime drops occur.             |

## 21. Debug Diagnostics and Production Policy

| Condition                                                           | Debug build behavior | Production behavior | Notes                                                                        |
| ------------------------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------------------- |
| documented platform capability is unavailable on the active runtime | debug warning        | degrade gracefully  | Use the documented fallback path instead of inventing browser-only behavior. |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?      | Responsibility                                                                     | Reused by                                      | Notes                                                                  |
| -------------------------- | -------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| platform capability helper | required       | Map DOM drag events or native drop APIs into the normalized payload contract.      | `drop-zone`, `download-trigger`, `dismissable` | Keeps web-only `DataTransfer` assumptions out of the machine boundary. |
| measurement helper         | not applicable | No dedicated layout measurement helper is required beyond normal node bookkeeping. | not applicable                                 | Payload normalization and cleanup matter more than geometry.           |

## 23. Framework-Specific Behavior

On Dioxus Web, drag handling uses browser `DataTransfer` events and the adapter-owned form bridge appends `api.form_data()` into `FormData` during parent form submission. On Dioxus Desktop or other non-web targets, the adapter must normalize native file-drop events into the shared `DragData` shape instead of relying on `DataTransfer`, and parent form reset must still dispatch the documented reset event back into the drop-zone.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct DropZoneSketchProps {
    pub children: Element,
}

#[component]
pub fn DropZone(props: DropZoneSketchProps) -> Element {
    let machine = use_machine::<drop_zone::Machine>(drop_zone::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone(), {props.children} } }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine_or_payload_controller(props);
let root_ref = create_root_ref();
let payload_helper = create_drop_payload_normalizer();
let cleanup = create_payload_cleanup_helper();
let capability = resolve_drop_capability();

render_inert_root_for_ssr(root_ref);
attach_runtime_drop_handlers(root_ref, capability, |raw_event| {
    let payload = payload_helper.normalize(raw_event);
    machine.send(payload_to_event(payload));
});
wire_optional_form_bridge(machine);
wire_reset_path(machine, cleanup);

on_cleanup(|| cleanup.release_retained_payload_resources());
```

## 26. Adapter Invariants

- Drag data normalization must preserve the core distinction between acceptable payloads and ignored payloads.
- `effectAllowed` and `dropEffect` coordination must remain explicit so browser feedback matches the accepted operation.
- Reset and clear behavior must define how pending drag state and dropped payload state are torn down.
- Form-data participation must be documented explicitly wherever dropped files or values are submitted with a form.
- Parent form submit and reset integration must route through the adapter bridge rather than bypassing the machine-owned dropped payload.
- Dioxus Desktop and other non-web targets must normalize native drop payloads into the shared drag contract instead of assuming `DataTransfer`.
- Platform-specific differences belong in framework behavior, while the invariant list must preserve the shared drag-and-drop contract.

## 27. Accessibility and SSR Notes

SSR renders only the inert root structure; drag behavior is client-only.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, root-structure, and event parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for drag-payload normalization, form-submit/reset bridging, temporary resource release, and platform-specific drag capability fallback.

## 29. Test Scenarios

- root mapping
- accepted vs rejected drop behavior
- read-only and invalid state
- accepted payload form submission bridge appends current dropped items to `FormData`
- parent form reset clears dropped payload state
- Dioxus Desktop/native drop events normalize into the shared drag payload contract

## 30. Test Oracle Notes

| Behavior                                   | Preferred oracle type | Notes                                                                                                       |
| ------------------------------------------ | --------------------- | ----------------------------------------------------------------------------------------------------------- |
| accepted vs rejected payload normalization | callback payload      | Assert normalized drop payload content, not raw browser transfer data.                                      |
| drag-active state                          | DOM attrs             | Assert drag-active/invalid attrs on the real root node.                                                     |
| resource release                           | cleanup side effects  | Verify object URLs or stored payload handles are released on clear/cleanup.                                 |
| form submit bridge and reset               | cleanup side effects  | Verify submit/reset integration reads `api.form_data()` and clears state through the documented reset path. |
| native Desktop drop abstraction            | callback payload      | Verify non-web drag sources normalize to the same accepted/rejected payload shape as web `DataTransfer`.    |

Cheap verification recipe:

1. Drop one accepted payload and one rejected payload, then assert the normalized callback payload rather than raw transfer data.
2. If `name` is configured, trigger form submit and reset through the parent form and verify the adapter-owned `FormData` bridge reads or clears the current dropped payload state.
3. Unmount or clear the zone and verify any object URLs or native payload handles are released; on Dioxus non-web targets, repeat the check with the native drop abstraction path.

## 31. Implementation Checklist

- [ ] Root attrs and drag handlers match the documented drop-zone contract.
- [ ] Accepted and rejected payload normalization is verified.
- [ ] Form submit and reset integration use the adapter-owned `FormData` bridge when `name` is set.
- [ ] Non-web Dioxus targets normalize native drop events into the shared drag contract.
- [ ] Temporary resources are released on reset and cleanup.
- [ ] SSR renders only the inert documented root structure.
