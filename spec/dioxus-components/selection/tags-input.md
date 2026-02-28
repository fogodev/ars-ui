---
adapter: dioxus
component: tags-input
category: selection
source: components/selection/tags-input.md
source_foundation: foundation/09-adapter-dioxus.md
---

# TagsInput — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TagsInput`](../../components/selection/tags-input.md) contract onto Dioxus 0.7.x. The adapter must preserve editable tag list plus input, hidden form bridge, and inline editing while making tag creation and removal, optional inline editing, hidden-input form participation, focus restoration, and live announcements explicit at the framework boundary.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TagsInputProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Vec<String>>>,
    #[props(optional, default)]
    pub default_value: Vec<String>,
    #[props(optional)]
    pub max: Option<usize>,
    #[props(optional)]
    pub delimiter: Option<String>,
    #[props(optional, default = true)]
    pub add_on_paste: bool,
    #[props(optional, default = false)]
    pub allow_duplicates: bool,
    #[props(optional, default = false)]
    pub editable: bool,
    #[props(optional)]
    pub blur_behavior: Option<tags_input::BlurBehavior>,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Vec<String>>>,
    pub children: Element,
}

#[component]
pub fn TagsInput(props: TagsInputProps) -> Element

/// Tag component: a single committed tag host.
#[derive(Props, Clone, PartialEq)]
pub struct TagProps {
    pub index: usize,
    pub value: String,
    pub children: Element,
}

#[component]
pub fn Tag(props: TagProps) -> Element

/// TagLabel component: the visible label inside a tag.
#[derive(Props, Clone, PartialEq)]
pub struct TagLabelProps {
    pub children: Element,
}

#[component]
pub fn TagLabel(props: TagLabelProps) -> Element

/// TagRemove component: button that removes a committed tag.
#[derive(Props, Clone, PartialEq)]
pub struct TagRemoveProps {
    pub index: usize,
    pub children: Element,
}

#[component]
pub fn TagRemove(props: TagRemoveProps) -> Element

/// Input component: the text input for pending tag text.
#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    #[props(optional)]
    pub placeholder: Option<String>,
    pub children: Element,
}

#[component]
pub fn Input(props: InputProps) -> Element

/// HiddenInput component: bridges native form submission.
#[component]
pub fn HiddenInput() -> Element

/// Description component: descriptive text wired to aria-describedby.
#[derive(Props, Clone, PartialEq)]
pub struct DescriptionProps {
    pub children: Element,
}

#[component]
pub fn Description(props: DescriptionProps) -> Element

/// EditInput component: inline edit surface for a committed tag.
#[derive(Props, Clone, PartialEq)]
pub struct EditInputProps {
    pub index: usize,
    pub children: Element,
}

#[component]
pub fn EditInput(props: EditInputProps) -> Element

/// LiveRegion component: announces removals, limits, and validation events.
#[component]
pub fn LiveRegion() -> Element
```

Compound helpers typically include `Tag`, `TagLabel`, `TagRemove`, `Input`, `HiddenInput`, `Description`, and optional `EditInput` or live-region helpers.

## 3. Mapping to Core Component Contract

- Props parity: full parity with tag value, delimiter, duplicate policy, max-count, blur behavior, and optional inline editing.
- Part parity: full parity for input, repeated tags, remove controls, hidden-input bridge, and descriptive content.
- Traceability note: this spec promotes hidden-input synchronization, IME suppression, paste tokenization, focus restoration, inline editing, and live announcements from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required?                       | Adapter rendering target | Ownership     | Attr source                 | Notes                                                       |
| --------------------- | ------------------------------- | ------------------------ | ------------- | --------------------------- | ----------------------------------------------------------- |
| Root                  | required                        | wrapper element          | adapter-owned | api.root_attrs()            | Owns compound context and group semantics.                  |
| Input                 | required                        | native text input        | adapter-owned | api.input_attrs()           | Captures pending tag text.                                  |
| Tag                   | repeated                        | adapter-owned tag host   | adapter-owned | api.tag_attrs(index)        | One host per committed tag.                                 |
| TagRemove             | optional repeated               | native button            | adapter-owned | api.tag_remove_attrs(index) | Removes a committed tag.                                    |
| HiddenInput           | required when `name` is present | native hidden input      | adapter-owned | api.hidden_input_attrs()    | Bridges form submission and reset behavior.                 |
| Description           | optional                        | descriptive node         | shared        | api.description_attrs()     | Participates in described-by wiring.                        |
| LiveRegion            | optional                        | status node              | adapter-owned | adapter-local attrs         | Announces removals, limits, and validation-adjacent events. |

## 5. Attr Merge and Ownership Rules

- Core attrs win for tag state, disabled or readonly state, and descriptive linkage on the root and input.
- The adapter owns the hidden-input `name`, serialized value, and reset synchronization policy.
- Consumers may decorate tag content, but they must not replace the tag host, remove button, or hidden-input bridge.

## 6. Composition / Context Contract

The root publishes required context to tag, remove, and optional edit-input helpers. The adapter consumes environment and field contracts, and it must fail fast when child parts render outside the root context.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode           | Sync trigger                                          | Machine event / update path                       | Notes                                                           |
| -------------------- | -------------- | ----------------------------------------------------- | ------------------------------------------------- | --------------------------------------------------------------- |
| `value`              | controlled     | signal change after mount                             | value sync event                                  | Updates committed tags and hidden-input serialization.          |
| native input events  | machine-owned  | input, paste, delimiter, Enter, or blur               | `InputChange`, `AddTag`, or equivalent events     | Pending text remains separate from committed tag values.        |
| inline editing       | adapter event  | Enter, Escape, click, or blur inside the edit surface | edit-specific transition path                     | Only enabled when `editable=true`.                              |
| form reset           | adapter bridge | native form reset event                               | restore default value and pending input           | The hidden-input bridge must stay aligned with reset semantics. |
| IME composition      | adapter event  | compositionstart / compositionend                     | suppresses delimiter and Enter-based tag creation | Intermediate composition text must not create a tag.            |

## 8. Registration and Cleanup Contract

- The adapter owns hidden-input synchronization, live-region helpers, and any edit-surface bookkeeping.
- Tag registration and focus-restoration helpers must be removed immediately when tags unmount or reorder.
- Form-reset subscriptions and announcement timers must be released on cleanup.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule                           | Notes                                                       |
| ------------------ | ------------- | ------------- | ---------------------------------- | ------------------------------------------ | ----------------------------------------------------------- |
| Input              | yes           | adapter-owned | required after mount               | compose only through the documented helper | Used for focus return after tag removal or edit completion. |
| Tag hosts          | recommended   | adapter-owned | required after mount               | no consumer composition by default         | Needed for predictable focus restoration between tags.      |
| HiddenInput        | no            | adapter-owned | always structural, handle optional | no composition                             | Exists only for form bridging.                              |

## 10. State Machine Boundary Rules

- Machine-owned state: committed tag list, pending input text, edit mode, and composition flag.
- Adapter-local derived bookkeeping: hidden-input serialization, focus-restoration targets, and live-region announcement throttling.
- Forbidden local mirrors: do not keep a second mutable tag vector or pending text buffer outside the machine state.
- Allowed snapshot reads: tag add or remove handlers, edit lifecycle handlers, reset listeners, and announcement effects.

## 11. Callback Payload Contract

| Callback          | Payload source           | Payload shape | Timing                           | Cancelable? | Notes                                                |
| ----------------- | ------------------------ | ------------- | -------------------------------- | ----------- | ---------------------------------------------------- |
| `on_value_change` | machine-derived snapshot | `Vec<String>` | after committed tag-list updates | no          | Fires after add, remove, or edit commit transitions. |

## 12. Failure and Degradation Rules

| Condition                                                   | Policy             | Notes                                                                         |
| ----------------------------------------------------------- | ------------------ | ----------------------------------------------------------------------------- |
| hidden-input bridge cannot attach during form participation | degrade gracefully | Keep interactive tag behavior but document missing native submission support. |
| duplicate tag requested when duplicates are disallowed      | no-op              | Leave the current tag list unchanged and optionally announce the rejection.   |
| controlled/uncontrolled mode switch after mount             | warn and ignore    | Preserve the first resolved value mode.                                       |

## 13. Identity and Key Policy

- Committed tag identity is composite: value plus stable index or key policy documented by the implementation.
- Hidden-input identity is instance-derived and tied to one tags-input root.
- Edit-surface and announcement resources are instance-derived and must not outlive the component.

## 14. SSR and Client Boundary Rules

- SSR renders the root, committed tag hosts, input, and hidden input with the initial serialized value.
- IME handling, form reset listeners, and live announcements are client-only.
- Hydration must preserve tag order and hidden-input value so native form submission stays stable.

## 15. Performance Constraints

- Do not rescan the full tag list on every keypress when only the pending input changes.
- Hidden-input serialization should update only when committed tags change.
- Focus-restoration lookups should use stable tag identity rather than DOM index scans when possible.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type | Why it must exist first                                                 | Notes                                                             |
| ------------------- | ----------- | --------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------- |
| hidden-input helper | required    | shared helper   | Form submission and reset semantics are adapter-owned responsibilities. | Shared with other form-participating selection controls.          |
| IME helper          | required    | shared helper   | Delimiter and Enter behavior must respect composition state.            | Shared with `autocomplete` and `combobox`.                        |
| live-region helper  | recommended | shared helper   | Removal and max-limit announcements should share a cleanup-safe helper. | Optional only when the product surface truly omits announcements. |

## 17. Recommended Implementation Sequence

1. Initialize the machine, publish root context, and render tags plus input in stable order.
2. Wire tag add, remove, paste, blur, and IME paths before adding optional inline editing.
3. Attach hidden-input synchronization and form-reset bridging.
4. Finish focus restoration and live-announcement behavior, then verify cleanup on tag removal and unmount.

## 18. Anti-Patterns

- Do not serialize pending input text into the hidden input before a tag is committed.
- Do not restore focus by raw DOM index alone when tag order or editing can change during the transition.
- Do not allow IME composition text to create tags through delimiter or Enter handling.

## 19. Consumer Expectations and Guarantees

- Consumers may assume committed tag changes are reflected in the hidden-input bridge when form participation is enabled.
- Consumers may assume remove controls and edit surfaces are adapter-owned and cleanup-safe.
- Consumers must not assume tag announcements or focus restoration happen implicitly; they are documented adapter behavior.

## 20. Platform Support Matrix

| Capability / behavior                                    | Web          | Desktop       | Mobile        | SSR            | Notes                                                                                   |
| -------------------------------------------------------- | ------------ | ------------- | ------------- | -------------- | --------------------------------------------------------------------------------------- |
| tag editing, add/remove flows, and hidden-input bridging | full support | fallback path | fallback path | full support   | Web has native form bridging; non-web targets may serialize for host forms differently. |
| IME composition and reset listeners                      | full support | fallback path | fallback path | SSR-safe empty | Normalize per-target event availability.                                                |
| live announcements                                       | client-only  | fallback path | fallback path | SSR-safe empty | Use target-specific announcement helpers when available.                                |

## 21. Debug Diagnostics and Production Policy

| Condition                                                        | Debug build behavior | Production behavior | Notes                                      |
| ---------------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------ |
| stale hidden-input serialization after tag change                | fail fast            | fail fast           | Form submission correctness depends on it. |
| remove or edit target no longer resolvable for focus restoration | debug warning        | degrade gracefully  | Fall back to the root input.               |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                                           | Reused by                 | Notes                                              |
| ------------------------ | ----------- | ------------------------------------------------------------------------ | ------------------------- | -------------------------------------------------- |
| hidden-input helper      | required    | Serialize committed tags and handle reset semantics.                     | `segment-group`, `select` | Keep form bridging consistent across the category. |
| focus-restoration helper | required    | Choose the correct tag or input target after removal or edit completion. | `menu-bar`                | Use stable identity, not incidental DOM order.     |
| announcement helper      | recommended | Announce removals, duplicate rejections, and max-count limits.           | `combobox`, `listbox`     | Must be cleanup-safe.                              |

## 23. Framework-Specific Behavior

Dioxus should keep tag lists and pending input in machine-owned state, bridge form participation differently on non-web targets when native hidden inputs are unavailable, and isolate focus-restoration effects to mounted nodes only.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TagsInputProps { /* ... */ }

#[component]
pub fn TagsInput(props: TagsInputProps) -> Element {
    let machine = use_machine::<tags_input::Machine>(tags_input::Props { /* ... */ });

    rsx! {
        div {
            ..machine.derive(|api| api.root_attrs())(),
            {props.children}
            input { ..machine.derive(|api| api.input_attrs())() }
            input { ..machine.derive(|api| api.hidden_input_attrs())() }
        }
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one hidden-input helper, one focus-restoration helper, and one optional announcement helper. Tag creation, removal, editing, and reset handling always read the committed machine snapshot after the transition completes.

## 26. Adapter Invariants

- Committed tags, pending input, and edit state remain machine-owned.
- Hidden-input serialization reflects committed tags only and stays reset-safe.
- Tag removal and edit completion restore focus predictably to a live node or the root input.

## 27. Accessibility and SSR Notes

- Description and error-message linkage must include only rendered nodes and remain synchronized with the root input.
- Remove controls require explicit accessible labels that include the tag value or another documented localized label.
- Live announcements should describe removals and max-count rejections without duplicating visible text unnecessarily.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of hidden-input bridging, edit lifecycle, and announcements.
- Intentional deviations: non-web form participation may use documented fallback paths instead of native browser reset events.

## 29. Test Scenarios

1. Delimiter, Enter, and paste create tags only when policy permits.
2. IME composition suppresses tag creation until the composition ends.
3. Removing a tag updates the hidden input and restores focus to the documented neighbor or input.
4. Inline editing commits or cancels correctly and fires one value-change callback per committed change.

## 30. Test Oracle Notes

- Preferred oracle for form participation: `DOM attrs` on the hidden input and `machine state` for committed tag changes.
- Preferred oracle for focus behavior: `callback order` plus DOM focus assertions after remove or edit transitions.
- Preferred oracle for announcements: `rendered structure` of the live-region node and `cleanup side effects` for timer disposal.

## 31. Implementation Checklist

- [ ] Hidden-input bridging, IME handling, and focus restoration are explicit adapter contracts.
- [ ] Tag add/remove/edit flows are machine-driven and cleanup-safe.
- [ ] Announcements and non-web fallback behavior are documented in platform, invariant, and test sections.
