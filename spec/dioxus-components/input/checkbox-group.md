---
adapter: dioxus
component: checkbox-group
category: input
source: components/input/checkbox-group.md
source_foundation: foundation/09-adapter-dioxus.md
---

# CheckboxGroup — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`CheckboxGroup`](../../components/input/checkbox-group.md) contract onto a Dioxus 0.7.x component. The adapter must preserve group-level labeling, validation wiring, shared disabled propagation, and child-checkbox delegation through context.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct CheckboxGroupProps {
    #[props(optional)]
    pub value: Option<BTreeSet<String>>,
    #[props(optional)]
    pub default_value: Option<BTreeSet<String>>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(optional)]
    pub orientation: Option<Orientation>,
    #[props(optional)]
    pub name: Option<String>,
    pub children: Element,
}

#[component]
pub fn CheckboxGroup(props: CheckboxGroupProps) -> Element
```

The adapter also forwards shared group props from the core contract, including locale or messages, description and error content, and maximum-selection policies. Plain props are preferred; wrappers may layer post-mount synchronization when needed.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core checkbox-group contract, including orientation, selection limits, and form naming.
- Event parity: `Toggle`, `SetValue`, `SetDisabled`, `SetReadonly`, `SetRequired`, and `SetInvalid` remain machine-owned.
- Core machine ownership: `use_machine::<checkbox_group::Machine>(...)` owns selection state, group-level ARIA attrs, and child delegation rules.

## 4. Part Mapping

| Core part / structure     | Required? | Adapter rendering target    | Ownership     | Attr source                 | Notes                                                               |
| ------------------------- | --------- | --------------------------- | ------------- | --------------------------- | ------------------------------------------------------------------- |
| `Root`                    | required  | `<div>`                     | adapter-owned | `api.root_attrs()`          | owns `role="group"` semantics                                       |
| `Label`                   | required  | `<span>` or `<label>`       | adapter-owned | `api.label_attrs()`         | group label only                                                    |
| `Description`             | optional  | `<div>`                     | adapter-owned | `api.description_attrs()`   | described-by content                                                |
| `ErrorMessage`            | optional  | `<div>`                     | adapter-owned | `api.error_message_attrs()` | invalid-only content                                                |
| child checkbox delegation | required  | child `Checkbox` components | shared        | group context               | each child checkbox remains responsible for its own control anatomy |

## 5. Attr Merge and Ownership Rules

- Group-level `role`, orientation, invalid, required, disabled, and described-by attrs on `Root` always win.
- Consumer `class` and `style` merge additively on `Root`.
- Child checkboxes may decorate their own structure, but they must not bypass group context when they are rendered inside the group.

## 6. Composition / Context Contract

`CheckboxGroup` is context-providing. Child checkboxes read group context to delegate toggle behavior, inherit disabled or readonly state, and participate in grouped form submission. There is no group-owned repeated item wrapper beyond what children render themselves.

## 7. Prop Sync and Event Mapping

| Adapter prop            | Mode          | Sync trigger              | Machine event / update path   | Visible effect                                   |
| ----------------------- | ------------- | ------------------------- | ----------------------------- | ------------------------------------------------ |
| `value`                 | controlled    | prop change               | `SetValue`                    | updates selected set and group validation        |
| `disabled` / `readonly` | controlled    | prop change               | `SetDisabled` / `SetReadonly` | updates group guards and child context           |
| `required` / `invalid`  | controlled    | prop change               | `SetRequired` / `SetInvalid`  | updates group ARIA and error wiring              |
| child toggle            | machine-owned | child checkbox activation | `Toggle(item)`                | updates selected set and hidden submission state |

Child checkboxes must not maintain an independent checked source of truth when group context is present.

## 8. Registration and Cleanup Contract

- No explicit descendant registry is required; child checkboxes delegate by reading the provided context.
- Group-owned resources are limited to the machine and context publication.
- Unmounting the group drops the context; children outside the subtree must not retain stale references.

## 9. Ref and Node Contract

- `Root` may own a live node ref for group-level focus or measurement helpers.
- Child checkbox refs remain child-owned.
- Group-level validation and label wiring always target `Root`, not child controls.

## 10. State Machine Boundary Rules

- The machine owns the selected set and selection-limit enforcement.
- Child checkboxes inside the group must delegate toggles to the group machine rather than mutating their own standalone state.
- Form submission semantics for grouped checkboxes are derived from the machine-owned selected set.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed `BTreeSet<String>` selection snapshot.
- Validation callbacks observe group state after invalid and described-by attrs are settled.
- Child checkbox callbacks, if exposed by wrappers, must reflect the group-committed state rather than a pre-commit local toggle.

## 12. Failure and Degradation Rules

| Condition                                                                      | Policy             | Notes                              |
| ------------------------------------------------------------------------------ | ------------------ | ---------------------------------- |
| controlled and uncontrolled value props are mixed after mount                  | warn and ignore    | first mode wins                    |
| child checkbox fails to consume required group context in a group-only wrapper | fail fast          | protects delegated state ownership |
| selection-limit diagnostics are not available in the current host              | degrade gracefully | state still guards max selection   |

## 13. Identity and Key Policy

The group instance owns one `Root` identity and one machine-owned selected-set identity. Child checkbox identities remain child-owned and must be keyed stably by their item value.

## 14. SSR and Client Boundary Rules

- SSR renders the same `Root`, label, description, and error structure as the client.
- Group context becomes active after hydration; child checkboxes must not render a different structure because of that activation.
- Selection state must hydrate from the same initial value on both server and client.

## 15. Performance Constraints

- Do not clone the selected set more often than necessary for callback emission or controlled sync.
- Context publication should be stable so unrelated child renders are not retriggered unnecessarily.
- Group-level attrs should derive from machine state, not from ad hoc recomputation across children.

## 16. Implementation Dependencies

| Dependency | Required? | Dependency type               | Why it must exist first                                    |
| ---------- | --------- | ----------------------------- | ---------------------------------------------------------- |
| `checkbox` | required  | child composition contract    | child checkboxes must know how to delegate inside a group  |
| `field`    | required  | label and validation contract | group-level description and error wiring must stay uniform |

## 17. Recommended Implementation Sequence

1. Initialize the group machine and derive group-level attrs.
2. Publish group context for child checkboxes.
3. Render `Root`, `Label`, and optional status parts around `children`.
4. Wire controlled prop synchronization and group-level callbacks.
5. Add selection-limit diagnostics and validation checks.

## 18. Anti-Patterns

- Do not let child checkboxes inside the group keep independent standalone state.
- Do not make `Root` itself the per-item interactive node.
- Do not hide group-level validation wiring inside child checkboxes only.

## 19. Consumer Expectations and Guarantees

- Consumers may assume child checkboxes delegate through group context when rendered inside the group.
- Consumers may assume `Root` owns group-level label and described-by semantics.
- Consumers must not assume the group creates extra item wrappers beyond what children render.

## 20. Platform Support Matrix

| Capability / behavior                                                | Web          | Desktop      | Mobile       | SSR          | Notes                             |
| -------------------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | --------------------------------- |
| documented group semantics, child delegation, and grouped submission | full support | full support | full support | full support | context activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                                  | Debug build behavior | Production behavior | Notes                            |
| ---------------------------------------------------------- | -------------------- | ------------------- | -------------------------------- |
| controlled/uncontrolled mode switch after mount            | debug warning        | warn and ignore     | preserves current mode           |
| child delegation contract violated in a group-only wrapper | fail fast            | fail fast           | protects grouped state ownership |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required? | Responsibility                                           | Notes                            |
| -------------------- | --------- | -------------------------------------------------------- | -------------------------------- |
| group context helper | required  | publish selected set, guards, and name to children       | shared with other grouped inputs |
| field merge helper   | required  | merge group-level label, described-by, and invalid state | shared by form-bound groups      |

## 23. Framework-Specific Behavior

Dioxus should publish a stable group context object with `use_context_provider`, keep controlled set synchronization narrow, and let child checkboxes read context only when mounted inside the group subtree.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<checkbox_group::Machine>(props);
use_context_provider(|| Context::new(machine));

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        span { ..machine.derive(|api| api.label_attrs()).read().clone(), {children} }
    }
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine, publish group context, and register only the controlled watchers that exist.
- Keep child checkbox delegation context-only; do not add redundant item wrappers unless another contract requires them.

## 26. Adapter Invariants

- Group-owned selection state always lives in the group machine.
- Child checkboxes inside the group always delegate toggles through context.
- Group-level described-by and invalid semantics always target `Root`.
- Group naming for submission remains stable across all children.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for group-level `aria-describedby`.
- `aria-required` belongs at the group level even though individual child controls expose their own checkbox semantics.
- SSR must preserve the same root-level label and validation structure used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core checkbox-group contract without intentional divergence.
- Promotes context publication, child delegation, and group-level validation ownership into Dioxus-facing guidance.

## 29. Test Scenarios

- Child checkbox toggles update the machine-owned selected set and group validation state.
- Controlled set updates synchronize without children falling back to standalone checkbox state.
- Required and invalid state update group-level described-by ordering correctly.
- Selection limits disable or guard additional child toggles as documented.

## 30. Test Oracle Notes

- Inspect group-level ARIA attrs on `Root` and child checkbox delegation behavior in the DOM.
- Assert selected-set changes from machine-driven callback logs rather than from raw child click counts.
- Use hydration tests to confirm stable group structure and initial selection state.

## 31. Implementation Checklist

- [ ] Keep group selection state machine-owned.
- [ ] Publish stable group context for child checkbox delegation.
- [ ] Keep group-level label and described-by semantics on `Root`.
- [ ] Synchronize controlled selected-set, disabled, readonly, required, and invalid props.
- [ ] Preserve stable child item identity by item value.
- [ ] Do not create undocumented item wrappers.
