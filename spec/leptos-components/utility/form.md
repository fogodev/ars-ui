---
adapter: leptos
component: form
category: utility
source: components/utility/form.md
source_foundation: foundation/08-adapter-leptos.md
---

# Form — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Form`](../../components/utility/form.md) and canonical forms behavior from `spec/foundation/07-forms.md` to Leptos 0.8.x. The adapter must render both the root form element and the hidden status region.

## 2. Public Adapter API

```rust
#[component]
pub fn Form(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] validation_behavior: Option<form::ValidationBehavior>,
    #[prop(optional)] validation_errors: Option<BTreeMap<String, Vec<String>>>,
    #[prop(optional)] action: Option<String>,
    #[prop(optional)] role: Option<String>,
    children: Children,
) -> impl IntoView
```

The adapter surfaces the full core prop set. Submission callbacks or server action wrappers may be layered on top without changing the core contract.

## 3. Mapping to Core Component Contract

- Props parity: full parity with `validation_behavior`, `validation_errors`, `action`, and `role`.
- Structure parity: `Root` and `StatusRegion` are both concrete adapter-rendered nodes.
- Event parity: submit, reset, prop-sync, and status announcement behavior all remain adapter-driven on top of the core forms machine.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target        | Ownership      | Attr source                 | Notes                                                                 |
| --------------------- | --------- | ------------------------------- | -------------- | --------------------------- | --------------------------------------------------------------------- |
| `Root`                | required  | native `<form>`                 | adapter-owned  | `api.root_attrs()`          | Must preserve `novalidate`, `aria-busy`, `action`, and optional role. |
| `StatusRegion`        | required  | hidden `<div>` inside `Root`    | adapter-owned  | `api.status_region_attrs()` | Structural live region for submission or validation announcements.    |
| form children         | required  | consumer children inside `Root` | consumer-owned | none                        | Descendants usually consume `Context`.                                |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                                                            | Adapter-owned attrs                                                     | Consumer attrs                                                          | Merge order                                                                                                                                                                              | Ownership notes                   |
| -------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| `Root`         | `api.root_attrs()` including validation, busy, action, and role attrs | adapter submit/reset handlers and structural `data-*` helpers if needed | consumer attrs on the form root                                         | core submission, validation, and accessibility attrs win when conflict would break the contract; `class`/`style` merge additively; handlers compose around normalized submit/reset logic | adapter-owned native `<form>`     |
| `StatusRegion` | `api.status_region_attrs()`                                           | status wrapper visibility helpers if needed                             | no direct consumer attrs unless a dedicated override slot is documented | core live-region attrs win; consumer content cannot replace the region node                                                                                                              | always adapter-owned              |
| form children  | none directly                                                         | none                                                                    | consumer field/tree content                                             | consumer children live inside the form root and consume context as needed                                                                                                                | descendants remain consumer-owned |

- Consumer overrides must not remove `novalidate`, busy semantics, or required live-region attrs when the core contract requires them.
- Form root event handlers are composed around normalized submit/reset handling; consumer handlers may observe normalized state but must not re-enable blocked submission.

## 6. Composition / Context Contract

`Form` provides form context to descendant field, fieldset, and submission-aware utilities. Required descendants use `use_context::<Context>().expect(...)`; optional ones use `use_context::<Context>()`.

## 7. Prop Sync and Event Mapping

Controlled/uncontrolled switching is not supported for validation state sources after mount unless a higher-level wrapper documents reinitialization. Default values are read at initialization; `validation_errors` and other reactive inputs follow effect-based sync.

| Adapter prop          | Mode                      | Sync trigger            | Machine event / update path                          | Visible effect                                                     | Notes                                                                    |
| --------------------- | ------------------------- | ----------------------- | ---------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| `validation_behavior` | controlled                | prop change after mount | `SetValidationBehavior` or equivalent machine update | switches between native and ARIA-managed validation behavior       | sync is immediate and effect-based                                       |
| `validation_errors`   | controlled                | prop change after mount | `SetValidationErrors` / server-error sync path       | updates field error state and status messaging                     | deterministic prop-to-machine path                                       |
| `action`              | non-reactive adapter prop | render time only        | included in root props                               | changes form submission target semantics                           | post-mount changes should be treated as unsupported unless reinitialized |
| `role`                | non-reactive adapter prop | render time only        | included in root props                               | affects root semantics when non-native role is explicitly required | must not break native form semantics                                     |

| UI event      | Preconditions                                   | Machine event / callback path         | Ordering notes                                                      | Notes                                                         |
| ------------- | ----------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------- |
| `submit`      | form submitted and not blocked by machine state | normalized submit handler -> `Submit` | normalization runs before consumer notification callbacks           | may prevent default when the machine blocks native submission |
| `reset`       | form reset triggered                            | normalized reset handler -> `Reset`   | reset dispatch occurs before descendant notification-only callbacks | registered fields restore canonical defaults                  |
| status update | machine status message changes                  | update status-region content          | live-region update occurs after machine transition                  | repeated messages follow the documented announcement timing   |

## 8. Registration and Cleanup Contract

- Descendant field-like utilities may register with form context for reset, validation, or submission coordination.
- The status region persists for the component lifetime and updates its content in place.
- Pending validation, submission, or status-announcement work must be cancelled before cleanup completes.

| Registered entity             | Registration trigger                  | Identity key      | Cleanup trigger                     | Cleanup action                                          | Notes                                           |
| ----------------------------- | ------------------------------------- | ----------------- | ----------------------------------- | ------------------------------------------------------- | ----------------------------------------------- |
| descendant field registration | descendant mount through form context | field instance ID | descendant cleanup or form cleanup  | remove field from the form coordination set             | required for reset and server-error propagation |
| status-announcement work      | status message enqueue                | form instance     | message replacement or form cleanup | clear queued work / timers and keep only current status | prevents repeated stale announcements           |

## 9. Ref and Node Contract

| Target part / node                    | Ref required?                                       | Ref owner      | Node availability                  | Composition rule                                     | Notes                                                            |
| ------------------------------------- | --------------------------------------------------- | -------------- | ---------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------- |
| `Root` form element                   | yes                                                 | adapter-owned  | required after mount               | compose if a wrapper also needs the native form node | Submit/reset normalization requires a concrete form handle.      |
| `StatusRegion`                        | yes for announcement updates                        | adapter-owned  | required after mount               | no composition unless explicitly exposed             | The live-region node must exist for status updates.              |
| descendant field registration targets | no direct shared ref requirement from the form spec | consumer-owned | always structural, handle optional | descendants own their own input refs                 | Registration uses form context rather than form-owned node refs. |

## 10. State Machine Boundary Rules

- machine-owned state: validation mode, submission lifecycle, server-error state, current status message, and registered-field coordination.
- adapter-local derived bookkeeping: transient form-node handle, status-region handle, and any timer handles for announcement sequencing.
- forbidden local mirrors: do not keep unsynchronized local copies of validation errors, current status text, or submission state.
- allowed snapshot-read contexts: submit/reset handlers, render derivation, status-update effects, and cleanup for pending announcement work.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape                                                | Timing                                                                             | Cancelable?                                             | Notes                                                                         |
| ----------------------------------------- | -------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------- |
| submit callback when exposed by a wrapper | normalized adapter payload | `{ form_data, validation_behavior, is_valid }`               | after normalized submit gating, before external side-effect wrappers if documented | yes when the wrapper exposes a veto layer; otherwise no | Must reflect machine-visible validation state, not ad hoc form serialization. |
| reset callback when exposed               | machine-derived snapshot   | `{ reset_to_defaults: bool, registered_field_count: usize }` | after normalized reset dispatch                                                    | no                                                      | Observational callback after the form begins canonical reset.                 |

## 12. Failure and Degradation Rules

| Condition                                                               | Policy          | Notes                                                                       |
| ----------------------------------------------------------------------- | --------------- | --------------------------------------------------------------------------- |
| invalid validation-mode combination or unsupported wrapper config       | fail fast       | The adapter should not silently invent validation semantics.                |
| missing descendant registration metadata during reset/error propagation | warn and ignore | Preserve form behavior for valid descendants while making the gap explicit. |
| browser validation APIs absent during SSR                               | no-op           | SSR renders structure only; validation execution starts on the client.      |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed?                                                       | DOM order must match registration order?                              | SSR/hydration stability                                              | Notes                                                   |
| -------------------------------- | --------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------- |
| registered fields                | composite       | no for canonical field IDs                                                | yes when reset order or focus order depends on DOM registration order | field IDs and status-region identity must be stable across hydration | Identity is field ID plus form instance.                |
| server-error entries             | data-derived    | yes across different field IDs; no duplicate keys for the same field slot | not applicable                                                        | error keys must match stable field identities                        | Use field ID or canonical field name as the key source. |

## 14. SSR and Client Boundary Rules

- SSR must render both `Root` and `StatusRegion`.
- Submit/reset listeners and validation execution are client-only.
- The status-region node must remain structurally identical across hydration so machine-driven updates land on the expected node.
- Form-node and status-region refs are server-safe absent and required after mount.

## 15. Performance Constraints

- Status-region updates should patch the existing node rather than replace the whole form subtree.
- Registered-field bookkeeping must update incrementally on mount/unmount instead of rebuilding from scratch each render.
- Validation-error sync should only dispatch when the incoming error payload actually changes.
- Pending status-announcement work must collapse stale entries rather than queueing duplicates indefinitely.

## 16. Implementation Dependencies

| Dependency    | Required?   | Dependency type         | Why it must exist first                                                                                | Notes                                                         |
| ------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| `field`       | required    | context contract        | Form-level registration and reset logic depends on canonical field identity and registration behavior. | Required for descendant field coordination.                   |
| `fieldset`    | recommended | context contract        | Grouped field inheritance and registration should align with form-level coordination.                  | Important for complex form trees.                             |
| `live-region` | recommended | behavioral prerequisite | The status region uses the same announcement timing principles as live-region utilities.               | Status messaging should not invent a parallel announce model. |

## 17. Recommended Implementation Sequence

1. Initialize the form machine and publish form context.
2. Render the native form root and persistent status region.
3. Establish descendant field registration and reset coordination.
4. Wire prop sync for validation behavior and server-validation errors.
5. Normalize submit/reset handling and status-message updates.
6. Verify cleanup of status-announcement work and field registrations.

## 18. Anti-Patterns

- Do not recreate the status region node on every status update.
- Do not bypass machine-owned validation or server-error state with ad hoc local form logic.
- Do not reset descendant fields outside the documented registration contract.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the hidden status region is always present and updated in place.
- Consumers may assume registered descendants participate in reset and server-error coordination through published form context.
- Consumers must not assume ad hoc local form state can bypass machine-owned validation, busy, or status semantics.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept                     | Required?   | Responsibility                                                                      | Reused by                   | Notes                                                               |
| ---------------------------------- | ----------- | ----------------------------------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------- |
| registry helper for repeated items | required    | Track descendant field registration, reset participation, and server-error routing. | `form`, `field`, `fieldset` | The registry owns canonical field identity, not the visual subtree. |
| debug-warning helper               | recommended | Surface missing descendant registration metadata without inventing form behavior.   | `form`, `field`, `fieldset` | Debug diagnostics should not bypass machine-owned validation state. |

## 23. Framework-Specific Behavior

Leptos can keep the status message in a memo so only the hidden live region updates. Form context is published with `provide_context`.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Form(children: Children) -> impl IntoView {
    let machine = use_machine::<form::Machine>(form::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    let status_attrs = machine.derive(|api| api.status_region_attrs());
    let status_message = machine.derive(|api| api.status_message().to_string());

    provide_context(Context {
        send: machine.send,
        service: machine.service,
        context_version: machine.context_version,
    });

    view! {
        <form
            {..root_attrs.get()}
            on:submit=move |ev| {
                ev.prevent_default();
                machine.send.run(form::Event::Submit);
            }
            on:reset=move |_| machine.send.run(form::Event::Reset)
        >
            {children()}
            <div {..status_attrs.get()}>
                {move || status_message.get()}
            </div>
        </form>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<form::Machine>(props);
let form_ref = create_form_ref();
let status_ref = create_status_region_ref();
let registry = create_field_registration_helper();

publish_form_context(machine, registry);
render_form_root_and_persistent_status_region(form_ref, status_ref);
sync_validation_behavior_and_server_errors(machine, props);
normalize_submit_and_reset_handlers(form_ref, machine, registry);
update_status_region_in_place(status_ref, machine.status_message());

on_cleanup(|| {
    registry.release_all();
    cancel_pending_status_work();
});
```

## 26. Adapter Invariants

- `StatusRegion` must remain a real structural node even when rendered visually hidden.
- Submit and reset wiring must preserve native form semantics unless the core machine explicitly blocks the action.
- Server-side error synchronization must follow a deterministic prop-to-machine path.
- Validation handling must distinguish native browser validation from ARIA-managed validation modes.
- SSR output must preserve the structural nodes needed for hydration-safe status and error messaging.
- Status announcements must preserve the documented timing sequence so repeated messages still announce reliably.

## 27. Accessibility and SSR Notes

- `StatusRegion` must always exist in the documented structure.
- It must remain hidden visually but available to assistive technology.
- SSR must render the same root/status-region structure used during hydration.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, structure, and lifecycle parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for persistent status-region structure, descendant field registration, validation and server-error sync, and normalized submit or reset behavior.

## 29. Test Scenarios

- `Root` and `StatusRegion` both rendered
- ARIA vs native validation mode
- server-error prop sync
- submit/reset behavior
- live-region announcement updates

## 30. Test Oracle Notes

| Behavior                            | Preferred oracle type | Notes                                                                           |
| ----------------------------------- | --------------------- | ------------------------------------------------------------------------------- |
| validation and submission lifecycle | machine state         | Assert canonical machine state transitions, not only superficial DOM changes.   |
| form and status-region attrs        | DOM attrs             | Assert busy, validation, and live-region attrs on the actual form/status nodes. |
| submit/reset sequencing             | callback order        | Verify normalized submit/reset handling precedes observational callbacks.       |
| descendant registration             | context registration  | Assert field registration changes on mount/unmount.                             |

Cheap verification recipe:

1. Render the form root with a persistent status region and assert both structural nodes before testing validation or submission.
2. Register at least one descendant field, then drive submit and reset through the form element and verify machine-state transitions before observational callbacks.
3. Update server-error input and status messaging, then confirm the status region is updated in place rather than recreated.

## 31. Implementation Checklist

- [ ] The form root and status region render as persistent documented structures.
- [ ] Descendant field registration and unregister behavior is wired through form context.
- [ ] Validation behavior and server-error sync follow the documented machine paths.
- [ ] Submit/reset normalization and callback order are verified.
- [ ] Test oracles cover machine state, DOM attrs, and context registration.
