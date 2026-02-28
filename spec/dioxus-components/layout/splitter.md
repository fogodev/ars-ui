---
adapter: dioxus
component: splitter
category: layout
source: components/layout/splitter.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Splitter — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Splitter`](../../components/layout/splitter.md) contract onto Dioxus `0.7.x` compound components. The adapter preserves resizable panels, draggable and keyboard-accessible handles, RTL-aware horizontal behavior, and optional persistence while defining live handle ownership, registration, platform fallback, and cleanup.

## 2. Public Adapter API

```rust
pub mod splitter {
    #[derive(Props, Clone, PartialEq)]
    pub struct SplitterProps {
        pub panels: Vec<splitter::Panel>,
        #[props(optional)]
        pub sizes: Option<Signal<Vec<f64>>>,
        #[props(optional)]
        pub default_sizes: Option<Vec<f64>>,
        pub orientation: Orientation,
        pub dir: Direction,
        pub size_unit: SizeUnit,
        pub keyboard_step: f64,
        #[props(optional)]
        pub storage_key: Option<String>,
        pub children: Element,
    }

    #[component]
    pub fn Splitter(props: SplitterProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct PanelProps {
        pub index: usize,
        pub children: Element,
    }

    #[component] pub fn Panel(props: PanelProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct HandleProps {
        pub index: usize,
        pub children: Element,
    }

    #[component] pub fn Handle(props: HandleProps) -> Element
}
```

`Splitter` owns the machine and registration contract. `Panel(index)` and `Handle(index)` map directly onto the corresponding core repeated parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core splitter props, including panels, controlled sizes, persistence, orientation, and direction.
- State parity: full parity with the core `Idle` and `Dragging` states.
- Part parity: full parity with repeated `Panel` and `Handle` parts plus the `Root`.
- Adapter additions: explicit handle ref ownership, pointer capture cleanup, persisted-size validation, and host fallback notes.

## 4. Part Mapping

| Core part / structure | Required?                        | Adapter rendering target | Ownership     | Attr source               | Notes                                             |
| --------------------- | -------------------------------- | ------------------------ | ------------- | ------------------------- | ------------------------------------------------- |
| `Root`                | required                         | host layout container    | adapter-owned | `api.root_attrs()`        | Owns the machine and layout registry.             |
| each `Panel`          | required                         | host layout child        | adapter-owned | `api.panel_attrs(index)`  | Panel order must match the `panels` prop order.   |
| each `Handle`         | required between adjacent panels | focusable host separator | adapter-owned | `api.handle_attrs(index)` | Each handle governs the panel to its left or top. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                         | Adapter-owned attrs                                    | Consumer attrs                        | Merge order                                             | Ownership notes              |
| ----------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------- | ------------------------------------------------------- | ---------------------------- |
| `Root`      | `api.root_attrs()` including orientation and drag state markers                    | keydown delegation and registry context publication    | wrapper decoration attrs when exposed | core state and orientation attrs win                    | root stays adapter-owned     |
| `Panel`     | `api.panel_attrs(index)` including computed size styles                            | panel registration metadata                            | panel decoration attrs                | required size styles and `data-ars-panel-id` win        | panel remains adapter-owned  |
| `Handle`    | `api.handle_attrs(index)` including separator role, value attrs, and state markers | drag handlers, keyboard handlers, and live-handle refs | decoration attrs or visual children   | separator semantics, tabindex, and ARIA range attrs win | handle remains adapter-owned |

## 6. Composition / Context Contract

- `Splitter` provides required splitter context containing machine access, panel metadata, persisted-size helpers, and live handle refs.
- `Panel` and `Handle` consume required context and fail fast when rendered outside `splitter::Splitter`.
- No optional external context is required beyond `dir` fallback derivation before root props are built.

## 7. Prop Sync and Event Mapping

| Adapter prop                                       | Mode       | Sync trigger                      | Machine event / update path    | Visible effect                              | Notes                                                        |
| -------------------------------------------------- | ---------- | --------------------------------- | ------------------------------ | ------------------------------------------- | ------------------------------------------------------------ |
| `sizes`                                            | controlled | signal change after mount         | `SetSizes { sizes }`           | resizes panel styles and handle value attrs | controlled/uncontrolled switching is unsupported after mount |
| `orientation`, `dir`, `size_unit`, `keyboard_step` | controlled | rerender with new props           | core prop update               | changes layout and keyboard delta behavior  | persistence and registry remain instance-scoped              |
| `storage_key`                                      | controlled | render-time and resize completion | adapter-owned persistence path | reads or writes saved sizes                 | invalid persisted data must not corrupt the machine          |

| UI event                 | Preconditions                       | Machine event / callback path     | Ordering notes                                                      | Notes                                                |
| ------------------------ | ----------------------------------- | --------------------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| handle pointer down      | handle mounted and splitter enabled | `DragStart { handle_index, pos }` | pointer capture bookkeeping begins before move events               | establishes drag session                             |
| pointer move during drag | active drag session                 | `DragMove { pos }`                | geometry reads use the current root size along the split axis       | no-op outside a drag session                         |
| pointer up or cancel     | active drag session                 | `DragEnd`                         | cleanup must release capture and transient listeners                | restores idle state                                  |
| handle keydown           | handle focused                      | `KeyDown { handle_index, event }` | RTL reversal applies before delta dispatch for horizontal splitters | Home, End, Enter, Space, and Escape remain supported |

## 8. Registration and Cleanup Contract

| Registered entity                  | Registration trigger                      | Identity key             | Cleanup trigger               | Cleanup action                                    | Notes                                               |
| ---------------------------------- | ----------------------------------------- | ------------------------ | ----------------------------- | ------------------------------------------------- | --------------------------------------------------- |
| splitter context                   | `Root` mount                              | instance-derived         | `Root` cleanup                | drop provided context and live refs               | one context per splitter                            |
| handle refs                        | each `Handle` mount                       | composite                | handle cleanup                | remove the live handle from the registry          | indices must stay aligned with panel order          |
| drag session listeners             | `DragStart`                               | instance-derived root id | `DragEnd` or cleanup          | release listeners and pointer capture bookkeeping | cleanup must run on pointer cancel too              |
| persisted-size storage entry write | size commit after drag or controlled sync | data-derived             | next commit or explicit reset | overwrite with validated current sizes            | storage writes are adapter-owned, not machine-owned |

## 9. Ref and Node Contract

| Target part / node          | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                                  |
| --------------------------- | ------------- | ------------- | ---------------------------------- | ----------------------- | ------------------------------------------------------ |
| `Root` split axis container | yes           | adapter-owned | required after mount               | no composition required | Pixel sizing and drag math depend on a live root size. |
| each `Panel`                | no            | adapter-owned | always structural, handle optional | no composition required | Panel size is represented in attrs.                    |
| each `Handle`               | yes           | adapter-owned | required after mount               | no composition required | Drag and roving focus depend on live handle nodes.     |

## 10. State Machine Boundary Rules

- machine-owned state: current sizes, drag state, focused handle index, orientation, direction, and panel constraints.
- adapter-local derived bookkeeping: live root and handle refs, pointer capture handles, persisted-size IO, and any host capability fallback state.
- forbidden local mirrors: do not track a second size vector outside the machine and controlled prop path.
- allowed snapshot-read contexts: drag math, keyboard resizing, persistence reads or writes, and cleanup.

## 11. Callback Payload Contract

| Callback                                  | Payload source           | Payload shape                                       | Timing                                             | Cancelable? | Notes                        |
| ----------------------------------------- | ------------------------ | --------------------------------------------------- | -------------------------------------------------- | ----------- | ---------------------------- |
| resize callback when exposed by a wrapper | machine-derived snapshot | `{ sizes: Vec<f64>, active_handle: Option<usize> }` | after size updates settle for the triggering event | no          | Wrapper-owned callback only. |

## 12. Failure and Degradation Rules

| Condition                                                 | Policy             | Notes                                                                                   |
| --------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------- |
| `Panel` or `Handle` rendered outside `splitter::Splitter` | fail fast          | Required splitter context is missing.                                                   |
| root or handle ref unavailable after mount                | fail fast          | Drag and keyboard resizing cannot work correctly.                                       |
| persisted size data is malformed or length-mismatched     | warn and ignore    | The adapter must fall back to validated default or controlled sizes.                    |
| current host cannot support full pointer-drag interaction | degrade gracefully | Keyboard resizing and semantic handles remain required even if pointer drag simplifies. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                             | Notes                                    |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------- | ---------------------------------------- |
| splitter root                    | instance-derived | not applicable      | not applicable                           | root and panel or handle structure must stay stable | one registry per root                    |
| panels and handles               | composite        | no                  | yes                                      | server order must match hydration order             | order follows `panels` plus handle index |
| persisted sizes                  | data-derived     | not applicable      | not applicable                           | storage data must map to the same logical panel set | storage key scopes the record            |

## 14. SSR and Client Boundary Rules

- SSR renders root, panels, and handles with the initial size attrs derived from controlled, persisted, or default sizes.
- Live refs, pointer listeners, and persisted storage IO are runtime-only.
- Hydration or equivalent client adoption must preserve panel and handle order exactly so drag indices stay aligned.

## 15. Performance Constraints

- Reuse live root and handle refs rather than rebuilding them on every render.
- Keep at most one active drag session and associated listener set per splitter.
- Avoid unnecessary size recomputation outside real prop, drag, or keyboard events.

## 16. Implementation Dependencies

| Dependency              | Required?   | Dependency type | Why it must exist first                                           | Notes                                 |
| ----------------------- | ----------- | --------------- | ----------------------------------------------------------------- | ------------------------------------- |
| ordered registry helper | required    | shared helper   | Keeps panel and handle indices aligned with rendered order.       | Shared with toolbar and carousel.     |
| measurement helper      | required    | shared helper   | Computes drag deltas against the root size along the active axis. | Shared with scroll area and carousel. |
| capability helper       | recommended | shared helper   | Selects full pointer-drag behavior versus host fallback behavior. | Drives platform notes.                |

## 17. Recommended Implementation Sequence

1. Initialize `Root`, resolve the initial size vector, and publish context.
2. Render panels and handles in stable order.
3. Capture live root and handle refs.
4. Implement pointer drag and keyboard resize paths.
5. Add validated persistence reads and writes plus host fallback behavior.

## 18. Anti-Patterns

- Do not keep an unsynchronized local copy of panel sizes.
- Do not derive handle indices from transient host queries instead of the rendered order.
- Do not remove required separator semantics from handles.

## 19. Consumer Expectations and Guarantees

- Consumers may assume each handle reports the size of the panel it governs through ARIA value attrs.
- Consumers may assume horizontal RTL splitters reverse arrow-key deltas.
- Consumers must not assume malformed persisted sizes will be applied.

## 20. Platform Support Matrix

| Capability / behavior               | Web          | Desktop       | Mobile        | SSR           | Notes                                                                                                  |
| ----------------------------------- | ------------ | ------------- | ------------- | ------------- | ------------------------------------------------------------------------------------------------------ |
| pointer and keyboard panel resizing | full support | fallback path | fallback path | fallback path | Non-web targets may simplify pointer drag, but semantic handles and keyboard resizing remain required. |

## 21. Debug Diagnostics and Production Policy

| Condition                         | Debug build behavior | Production behavior | Notes                                                    |
| --------------------------------- | -------------------- | ------------------- | -------------------------------------------------------- |
| required splitter context missing | fail fast            | fail fast           | Compound parts must be nested under `Root`.              |
| malformed persisted sizes         | debug warning        | warn and ignore     | Fall back to validated defaults or controlled sizes.     |
| missing live root or handle refs  | fail fast            | fail fast           | Drag math and keyboard focus cannot be recovered safely. |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required?   | Responsibility                                               | Reused by                 | Notes                                            |
| ----------------------- | ----------- | ------------------------------------------------------------ | ------------------------- | ------------------------------------------------ |
| ordered registry helper | required    | Keeps panels and handles aligned with render order.          | `toolbar`, `carousel`     | Supports deterministic focus and drag targeting. |
| measurement helper      | required    | Computes size deltas and clamps them against constraints.    | measurement-heavy widgets | Avoids duplicated axis math.                     |
| capability helper       | recommended | Selects full pointer drag versus fallback interaction paths. | `scroll-area`, `portal`   | Keeps host variance explicit.                    |

## 23. Framework-Specific Behavior

Dioxus web can keep the splitter registry and live handles in context while pointer listeners and storage IO run at runtime. Non-web targets may need simplified pointer behavior, but keyboard and semantic handle behavior remain normative.

## 24. Canonical Implementation Sketch

```rust
pub mod splitter {
    #[derive(Props, Clone, PartialEq)]
    pub struct SplitterProps {
        pub panels: Vec<splitter::Panel>,
        pub children: Element,
    }

    #[component]
    pub fn Splitter(props: SplitterProps) -> Element {
        let machine = use_machine::<splitter::Machine>(splitter::Props { panels: props.panels, ..Default::default() });
        use_context_provider(|| Context::from_machine(machine));
        rsx! { div { {props.children} } }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub mod splitter {
    #[derive(Props, Clone, PartialEq)]
    pub struct SplitterProps {
        pub panels: Vec<splitter::Panel>,
        pub children: Element,
    }

    #[component]
    pub fn Splitter(props: SplitterProps) -> Element {
        let machine = use_machine::<splitter::Machine>(splitter::Props { panels: props.panels, ..Default::default() });
        use_context_provider(|| Context::from_machine(machine));
        rsx! { div { ..machine.derive(|api| api.root_attrs()), {props.children} } }
    }
}
```

## 26. Adapter Invariants

- `Splitter` owns exactly one ordered panel and handle contract.
- Live root and handle refs always exist before drag or keyboard resizing begins.
- Persisted size data is validated before it can influence rendered sizes.

## 27. Accessibility and SSR Notes

- Each handle must remain a `role="separator"` with orientation and value attrs intact.
- Keyboard resize behavior must remain available even when pointer drag is reduced on a host platform.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Splitter` contract on web, with documented host fallbacks elsewhere.
- Intentional deviations: non-web targets may simplify pointer drag behavior while retaining semantic handles and keyboard resize.
- Traceability note: adapter-owned live refs, drag cleanup, persistence validation, and host fallback selection are promoted into explicit Dioxus rules.

## 29. Test Scenarios

- Drag a handle and verify size updates plus handle value attrs.
- Use keyboard arrows, Home, End, Enter, Space, and Escape and verify expected size or collapse behavior.
- Verify RTL horizontal arrow reversal.
- Load malformed persisted sizes and verify fallback behavior.
- Render a part outside `splitter::Splitter` and verify failure behavior.
- Verify documented host fallback behavior where full pointer drag is unavailable.

## 30. Test Oracle Notes

- Panel and handle attrs: prefer `DOM attrs`.
- Size transitions: prefer `machine state`.
- Persistence behavior: prefer `cleanup side effects` or stored-value assertions.
- Ordered registration: prefer `context registration`.

## 31. Implementation Checklist

- [ ] Publish one required splitter context from `Root`.
- [ ] Keep panel and handle order aligned with the `panels` prop.
- [ ] Capture live root and handle refs before enabling drag or keyboard resizing.
- [ ] Validate persisted sizes before applying them.
- [ ] Document and implement host fallback behavior where full pointer drag is unavailable.
