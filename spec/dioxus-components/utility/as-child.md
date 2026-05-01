---
adapter: dioxus
component: as-child
category: utility
source: components/utility/as-child.md
source_foundation: foundation/09-adapter-dioxus.md
---

# AsChild — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`AsChild`](../../components/utility/as-child.md) pattern to Dioxus 0.7.x. It documents root reassignment and attr or handler merging for components that expose `as_child`.

## 2. Public Adapter API

```rust,no_check
#[derive(Clone, Debug, PartialEq)]
pub struct AsChildRenderProps {
    pub attrs: Vec<Attribute>,
}

#[derive(Props, Clone, PartialEq)]
pub struct AsChildSlotProps {
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
    pub render: Callback<AsChildRenderProps, Element>,
}

#[component]
pub fn AsChildSlot(props: AsChildSlotProps) -> Element
```

`as_child: bool` is surfaced by the hosting component, not by a standalone core component.
The explicit `render` callback is required intentionally: Dioxus does not expose a stable
API for arbitrary `VNode` template mutation. The consumer or hosting component owns the root
element and must spread the provided attrs onto that root.
`attrs` is already converted to Dioxus attributes before it reaches the slot; hosting
components choose the active style strategy and own any CSSOM or nonce-style side effects.
The `attrs` field uses `#[props(extends = GlobalAttributes)]` so callers may pass native
Dioxus global attributes directly to `AsChildSlot` when they are already operating in
Dioxus attr space.
Hosting components should use `attr_map_to_dioxus(attrs, strategy, element_id)` so
inline, CSSOM, and nonce style strategy payloads are handled consistently before the slot
receives native attrs.

## 3. Mapping to Core Component Contract

- Pattern parity: the adapter must preserve root reassignment semantics.
- Attr parity: hosting components merge component attrs onto consumer-child attrs before converting to Dioxus attributes.
- Event parity: event handler composition belongs to the hosting component adapter, not to `AsChildSlot`.

## 4. Part Mapping

| Core part / structure      | Required?                     | Adapter rendering target      | Ownership        | Attr source            | Notes                                                             |
| -------------------------- | ----------------------------- | ----------------------------- | ---------------- | ---------------------- | ----------------------------------------------------------------- |
| reassigned root            | required when `as_child=true` | single consumer child element | consumer-owned   | converted Dioxus attrs | The conceptual root remains; only the rendering target changes.   |
| suppressed default wrapper | conditional                   | no DOM output                 | adapter behavior | none                   | Must be documented whenever default wrapper rendering is skipped. |

## 5. Attr Merge and Ownership Rules

| Target node           | Core attrs                                                                     | Adapter-owned attrs                                                           | Consumer attrs                                        | Merge order                                                                                                                                                                                                                                                                                                                                   | Ownership notes                                                                                 |
| --------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| reassigned root child | converted Dioxus attrs produced from the hosting component's merged root attrs | any adapter-local structural `data-*` markers needed by the hosting component | the render callback's root element attrs and handlers | hosting components must merge framework-agnostic attrs before conversion; core required ARIA/state attrs win when conflict would break the contract; child `class`/`style` and ARIA token lists merge through the core `AsChildMerge` rules before reaching the slot; handler order remains the hosting component's normalized event contract | rendered node is consumer-owned, but semantic root ownership remains with the hosting component |

- The render callback must return exactly one root element and spread `..attrs` onto that root.
- No wrapper node may be introduced in `as_child` mode.
- If the child already has `role`, `tabindex`, or `aria-*`, the merge result must preserve required core semantics instead of blindly preferring the child value.
- If a child handler calls `prevent_default()`, later notification-only handlers may observe that state but must not re-enable a blocked action; this composition is owned by the hosting component.
- The slot passes final converted Dioxus attrs to the callback; it must not mutate arbitrary `VNode` templates.
- Inline attr conversion through `attr_map_to_dioxus_inline_attrs` is only a convenience for simple callers and tests; production component adapters must preserve their active style strategy before calling the slot.

## 6. Composition / Context Contract

Exactly one render callback root element is required. Context behavior of the hosting component does not change under root reassignment.

## 7. Prop Sync and Event Mapping

`AsChild` itself has no independent machine state. The hosting component remains responsible for prop sync. This adapter slot only passes forwarded native attrs to the render callback.

| Adapter prop      | Mode                      | Sync trigger                         | Machine event / update path | Visible effect                                                                         | Notes                                           |
| ----------------- | ------------------------- | ------------------------------------ | --------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------- |
| forwarded `attrs` | non-reactive adapter prop | each render of the hosting component | passed to `render`          | callback root receives the hosting component's root contract when it spreads `..attrs` | attrs are already converted to `Vec<Attribute>` |

| UI event             | Preconditions                     | Machine event / callback path           | Ordering notes                                                                                                                                                   | Notes                                                                                       |
| -------------------- | --------------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| child event handlers | exactly one callback root element | composed hosting-component handler path | guard logic runs before child-only behavior when the hosting component needs to prevent invalid activation; notification-only callbacks run after child handlers | event composition is defined and implemented by the hosting component, not by `AsChildSlot` |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond invoking the render callback.
- Root ownership is explicit in the render callback; opaque child-count validation is intentionally not performed through vnode mutation.
- Cleanup is ordinary vnode disposal; no persistent listeners or timers belong to the slot helper itself.

| Registered entity      | Registration trigger | Identity key               | Cleanup trigger                  | Cleanup action          | Notes                                                                         |
| ---------------------- | -------------------- | -------------------------- | -------------------------------- | ----------------------- | ----------------------------------------------------------------------------- |
| render callback result | each render          | hosting component instance | next render or component cleanup | ordinary vnode disposal | the callback is responsible for one root and for spreading the provided attrs |

## 9. Ref and Node Contract

| Target part / node                         | Ref required?                                                                          | Ref owner                                             | Node availability                  | Composition rule                                                 | Notes                                                                              |
| ------------------------------------------ | -------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| primary rendered node or provider boundary | no when the component is purely structural or provider-only; otherwise adapter-defined | adapter-owned unless part mapping says consumer-owned | always structural, handle optional | composed only when a consumer-owned node receives the core attrs | Use a live node handle only when the component's interaction contract requires it. |

## 10. State Machine Boundary Rules

- machine-owned state: all core interaction, accessibility, and controlled-state values defined by the component machine or derived API.
- adapter-local derived bookkeeping: minimal ephemeral data such as pointer modality, mount status, or observer handles when the core machine does not model them directly.
- forbidden local mirrors: do not fork controlled props, accessibility attrs, or machine-visible state into an unsynchronized local source of truth.
- allowed snapshot-read contexts: render-time derivation, event handlers, effects, and cleanup only when reading snapshots does not bypass required machine events.

## 11. Callback Payload Contract

| Callback                     | Payload source         | Payload shape          | Timing         | Cancelable?        | Notes                                                                    |
| ---------------------------- | ---------------------- | ---------------------- | -------------- | ------------------ | ------------------------------------------------------------------------ |
| `render` structural callback | converted native attrs | [`AsChildRenderProps`] | during render  | no                 | Owns the root element and must spread the provided attrs onto that root. |
| component event callbacks    | hosting component      | component-specific     | event dispatch | component-specific | Event callback composition is outside the slot helper.                   |

## 12. Failure and Degradation Rules

| Condition                                                                  | Policy             | Notes                                                                             |
| -------------------------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------- |
| unsupported platform capability or missing browser-only API during SSR     | degrade gracefully | Render structural output and defer behavior until client-only APIs are available. |
| impossible prop combinations not explicitly supported by the core contract | fail fast          | Prefer an explicit contract violation over silently inventing behavior.           |

## 13. Identity and Key Policy

| Registered or repeated structure   | Identity source | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                  | Notes                                                                                             |
| ---------------------------------- | --------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| none beyond the component instance | not applicable  | not applicable      | not applicable                           | stable root structure required where the component renders on the server | Use a component-instance identity only for cleanup bookkeeping when no repeated structure exists. |

## 14. SSR and Client Boundary Rules

- The server must render every structural node required by the part-mapping table unless the component is explicitly provider-only or client-gated.
- Client-only listeners, timers, measurements, and node-handle work must wait until hydration or mount.
- Any node that participates in hydration-sensitive semantics must preserve the same structural identity across server and client render paths.

## 15. Performance Constraints

- Attr maps derived from the machine should be memoized or read through adapter derivation helpers instead of rebuilt eagerly on every render.
- Listener, timer, and observer registration must be stable across rerenders and must not churn unless the governing configuration actually changes.
- Cleanup must release only the resources owned by the current component instance and must avoid repeated quadratic teardown work.

## 16. Implementation Dependencies

| Dependency                                  | Required?   | Dependency type | Why it must exist first                                                                                         | Notes                                                                 |
| ------------------------------------------- | ----------- | --------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| none beyond the documented utility contract | recommended | conceptual      | The component may still rely on shared adapter patterns even when no specific prerequisite utility is required. | State `not applicable` explicitly if there is no stronger dependency. |

## 17. Recommended Implementation Sequence

1. Establish the machine/context boundary and confirm the documented part mapping.
2. Establish any required refs, provider context, or registration surfaces.
3. Derive attrs and render the documented structural nodes.
4. Wire prop sync and normalized event handling.
5. Add SSR/client-only guards, cleanup, and verify the documented test oracles.

## 18. Anti-Patterns

- Do not mirror machine-owned state in unsynchronized local adapter state.
- Do not bypass the documented attr merge order or replace required structural nodes with equivalent-looking wrappers.
- Do not re-register listeners, timers, observers, or registries on every render when the governing configuration has not changed.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior                     | Web          | Desktop      | Mobile       | SSR          | Notes                                                                                                |
| ----------------------------------------- | ------------ | ------------ | ------------ | ------------ | ---------------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support | full support | full support | full support | This utility does not have additional platform variance beyond its existing framework and SSR rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior    | Production behavior     | Notes                                                                       |
| ---------------------------------------------- | ----------------------- | ----------------------- | --------------------------------------------------------------------------- |
| render callback does not spread provided attrs | contract violation      | contract violation      | the slot cannot repair omitted attrs without mutating opaque Dioxus vnodes. |
| arbitrary `VNode` template mutation requested  | fail fast at API review | fail fast at API review | use the explicit render callback instead.                                   |

## 22. Shared Adapter Helper Notes

| Helper concept                    | Required? | Responsibility                                                                                                    | Reused by                                                             | Notes                                                                                                   |
| --------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `as_child` render callback helper | required  | Pass already-converted `Vec<Attribute>` to a callback that owns the root element.                                 | `as-child`, `button`, `visually-hidden`, any polymorphic root utility | This helper owns root reassignment without deleting the conceptual root part or mutating opaque vnodes. |
| semantic-warning helper           | optional  | Emit semantic-mismatch diagnostics in debug builds when the hosting component has enough explicit child metadata. | `button`, `download-trigger`, `action-group`                          | Warnings are host-level diagnostics, not slot behavior.                                                 |

## 23. Framework-Specific Behavior

Dioxus needs an adapter-local render callback helper because arbitrary `VNode` template
mutation is not a stable API. The supported mechanism is to pass already-converted Dioxus
attributes as `AsChildRenderProps { attrs }` and require the callback to spread `..attrs`
onto exactly one root element. Hosting components convert the merged `AttrMap` with the
active style strategy before calling the slot, and they own any CSSOM synchronization or
nonce-style injection.

## 24. Canonical Implementation Sketch

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct AsChildRenderProps {
    pub attrs: Vec<Attribute>,
}

#[derive(Props, Clone, PartialEq)]
pub struct AsChildSlotSketchProps {
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
    pub render: Callback<AsChildRenderProps, Element>,
}

#[component]
pub fn AsChildSlot(props: AsChildSlotSketchProps) -> Element {
    props.render.call(AsChildRenderProps { attrs: props.attrs })
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Exactly one consumer-owned callback root is required whenever root reassignment is used.
- Root reassignment must not delete the conceptual root part; it only changes ownership of the rendered node.
- Handler composition order must be explicit in each hosting component so consumer handlers and adapter handlers do not race unpredictably.
- Role, ARIA, and state attr merge rules must remain explicit when attrs are forwarded onto the child.
- Arbitrary `VNode` template mutation is forbidden for this helper.
- The slot must not force inline styles; style-strategy conversion happens before attrs reach the slot.

## 27. Accessibility and SSR Notes

Semantic correctness of the final root element remains the responsibility of the hosting component and consumer. Slot-level code cannot infer whether the callback root spread the attrs or whether the selected tag has equivalent keyboard behavior.

## 28. Parity Summary and Intentional Deviations

Parity summary: full pattern parity.

Intentional deviations: opaque-child vnode mutation is not part of parity; an explicit render
callback is the Dioxus-supported way to provide the same root reassignment contract. The
slot accepts native attrs so hosting components can preserve inline, CSSOM, and nonce style
strategies outside the slot.

## 29. Test Scenarios

- root reassignment
- suppressed wrapper documentation
- render callback receives converted attrs
- `#[props(extends = GlobalAttributes)]` collects native Dioxus global attrs for the callback
- consumer spreads attrs onto one root without a wrapper
- merged class, style, and ARIA preservation through the hosting component's converted attrs
- `attr_map_to_dioxus` preserves CSSOM and nonce style payloads for hosting components
- SSR output with the same root element shape and attrs as the client path

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                                                                             |
| -------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.                                                        |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                                                                    |
| style-strategy preservation            | component integration | Verify concrete hosting components convert attrs before calling the slot and handle CSSOM or nonce side effects.                  |
| concrete callback spreading            | component integration | Every concrete Dioxus component adopting `as_child` must prove its render callback spreads provided attrs onto the intended root. |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
- [ ] Arbitrary `VNode` mutation is not used; root ownership stays in the explicit render callback.
- [ ] The slot accepts native Dioxus attrs through explicit `attrs` and `#[props(extends = GlobalAttributes)]` and does not force inline-only conversion.
- [ ] Hosting components use `attr_map_to_dioxus` before calling the slot.
- [ ] Each concrete Dioxus component adopting `as_child` tests that its callback spreads attrs onto the intended root.
