---
adapter: dioxus
component: steps
category: navigation
source: components/navigation/steps.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Steps — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Steps`](../../components/navigation/steps.md) contract onto a Dioxus 0.7.x component. The adapter preserves ordered step rendering, current-step semantics, status-driven item output, optional interactive step navigation, and the documented prev or next trigger behavior.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct StepsProps {
    #[props(optional)]
    pub step: Option<u32>,
    pub default_step: u32,
    pub count: u32,
    pub statuses: Vec<steps::Status>,
    #[props(default = false)]
    pub linear: bool,
    pub orientation: Orientation,
    #[props(default = false)]
    pub interactive: bool,
    pub children: Element,
}

#[component]
pub fn Steps(props: StepsProps) -> Element
```

The adapter owns repeated item rendering, step-status projection, current-content visibility, and prev or next trigger wiring.

## 3. Mapping to Core Component Contract

- Props parity: full parity with current step, statuses, linear guard rules, count, and orientation.
- State parity: full parity with the core single-current-step model.
- Part parity: full parity with `Root`, `List`, repeated `Item`, `Indicator`, `Title`, `Description`, `Separator`, repeated `Content`, `PrevTrigger`, and `NextTrigger`.
- Adapter additions: explicit list-pattern vs tab-pattern rendering guidance when `interactive=true`.

## 4. Part Mapping

| Core part / structure         | Required? | Adapter rendering target                | Ownership     | Attr source                                             | Notes                                                |
| ----------------------------- | --------- | --------------------------------------- | ------------- | ------------------------------------------------------- | ---------------------------------------------------- |
| `Root`                        | required  | `<div>`                                 | adapter-owned | `api.root_attrs()`                                      | owns orientation and labeling                        |
| `List`                        | required  | `<ol>` by default                       | adapter-owned | `api.list_attrs()`                                      | interactive variant may repair semantics             |
| `Item`                        | repeated  | `<li>` or `<button>`-containing wrapper | adapter-owned | `api.item_attrs(index)`                                 | current item gets `aria-current="step"` in list mode |
| `Indicator`                   | repeated  | `<span>`                                | adapter-owned | `api.indicator_attrs(index)`                            | decorative status indicator                          |
| `Title` / `Description`       | repeated  | `<span>`                                | adapter-owned | adapter-owned attrs plus core part ids                  | textual subparts                                     |
| `Separator`                   | repeated  | `<div>`                                 | adapter-owned | `api.separator_attrs(index)`                            | decorative progress separator                        |
| `Content`                     | repeated  | `<div>`                                 | adapter-owned | `api.content_attrs(index)`                              | hidden when not current                              |
| `PrevTrigger` / `NextTrigger` | required  | `<button>`                              | adapter-owned | `api.prev_trigger_attrs()` / `api.next_trigger_attrs()` | navigation controls                                  |

## 5. Attr Merge and Ownership Rules

| Target node             | Core attrs                                | Adapter-owned attrs                                  | Consumer attrs                           | Merge order                           | Ownership notes                              |
| ----------------------- | ----------------------------------------- | ---------------------------------------------------- | ---------------------------------------- | ------------------------------------- | -------------------------------------------- |
| `Root` and `List`       | scope, part, orientation, and label attrs | none beyond structural rendering                     | wrapper decoration only if later exposed | structure and accessibility attrs win | adapter-owned root structure                 |
| repeated `Item` surface | current, status, and index attrs          | optional interactive click or key handlers           | item decoration attrs                    | required current and status attrs win | interactive mode still keeps adapter control |
| `Content`               | hidden and index attrs                    | presence policy if a wrapper adds lazy content later | decoration attrs                         | hidden and linkage attrs win          | inactive content remains non-current         |
| prev or next triggers   | disabled and label attrs                  | normalized click handlers                            | decoration attrs and trailing handlers   | disabled semantics win                | native button behavior preferred             |

## 6. Composition / Context Contract

`Steps` is standalone. It does not require child context for baseline rendering. If wrappers expose compound subcomponents later, they must preserve the same current-step and status contract documented here.

## 7. Prop Sync and Event Mapping

| Adapter prop                           | Mode       | Sync trigger            | Machine event / update path | Visible effect                                                 | Notes                                            |
| -------------------------------------- | ---------- | ----------------------- | --------------------------- | -------------------------------------------------------------- | ------------------------------------------------ |
| `step`                                 | controlled | prop change after mount | `GoToStep`                  | updates current item, content, and edge-trigger disabled state | no controlled/uncontrolled switching after mount |
| `statuses`                             | controlled | rerender with new props | core prop rebuild           | updates item and separator state                               | index alignment must stay stable                 |
| `linear`, `orientation`, `interactive` | controlled | rerender with new props | core prop rebuild           | updates navigation guards and semantics                        | no shadow state                                  |

| UI event                | Preconditions                              | Machine event / callback path | Ordering notes                                                    | Notes                                                |
| ----------------------- | ------------------------------------------ | ----------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------- |
| prev trigger activation | step > 0                                   | `PrevStep`                    | committed step changes before wrapper callbacks                   | native button path                                   |
| next trigger activation | step < count - 1 and linear guard passes   | `NextStep`                    | validation guard runs before callbacks                            | completion callback may fire when final step commits |
| item activation         | `interactive=true` and target step allowed | `GoToStep(index)`             | list mode may repair into tab-like semantics only when documented | current item may be clicked again as no-op           |

## 8. Registration and Cleanup Contract

- No descendant registry is required for baseline repeated rendering.
- Optional interactive item node refs remain local to the instance only.
- No timers or global listeners are required.

## 9. Ref and Node Contract

| Target part / node    | Ref required?                                                 | Ref owner     | Node availability        | Composition rule                     | Notes                                          |
| --------------------- | ------------------------------------------------------------- | ------------- | ------------------------ | ------------------------------------ | ---------------------------------------------- |
| prev or next triggers | no                                                            | adapter-owned | always structural        | no composition                       | native button semantics                        |
| interactive item node | recommended when roving or repaired keyboard support is added | adapter-owned | repeated and conditional | compose only if wrappers expose refs | list-mode-only rendering does not require refs |

## 10. State Machine Boundary Rules

- Current step, step count, statuses, and linear completion guards remain core-owned.
- Item clickability, optional tab-style semantics, and host choice remain adapter-owned.
- The adapter must not infer completed vs incomplete state from DOM order alone.

## 11. Callback Payload Contract

| Callback             | Payload source           | Payload shape                          | Timing                          | Cancelable? | Notes                            |
| -------------------- | ------------------------ | -------------------------------------- | ------------------------------- | ----------- | -------------------------------- |
| step-change callback | machine-derived snapshot | `{ step: u32, status: steps::Status }` | after committed step transition | no          | wrapper-owned surface only       |
| completion callback  | machine-derived snapshot | `{ completed_step: u32 }`              | when final transition commits   | no          | only when the wrapper exposes it |

## 12. Failure and Degradation Rules

| Condition                                                  | Policy             | Notes                                                      |
| ---------------------------------------------------------- | ------------------ | ---------------------------------------------------------- |
| `statuses.len()` does not match `count`                    | warn and ignore    | fall back to derived incomplete statuses for missing items |
| controlled step outside `0..count`                         | warn and ignore    | clamp or preserve nearest valid step                       |
| interactive tab-style semantics unsupported by host markup | degrade gracefully | fall back to list semantics with button activation only    |

## 13. Identity and Key Policy

Each step item is keyed by stable step index. Server and client must preserve count, index order, and current-step visibility for hydration safety.

## 14. SSR and Client Boundary Rules

- SSR renders all item shells and the current content branch from initial props.
- Non-current content may remain hidden but structurally present unless a later adapter extension explicitly adopts lazy presence behavior.
- Interactive item activation remains client-side.

## 15. Performance Constraints

- Derive item state from the committed core snapshot rather than mirroring per-item signals.
- Avoid rebuilding status text or hidden summaries more than once per render pass.
- Keep optional interactive-node bookkeeping instance-local.

## 16. Implementation Dependencies

| Dependency             | Required?   | Dependency type   | Why it must exist first                                                              | Notes                               |
| ---------------------- | ----------- | ----------------- | ------------------------------------------------------------------------------------ | ----------------------------------- |
| button semantic helper | recommended | behavioral helper | prev or next triggers and optional interactive items reuse native-button-first rules | shared with `pagination` and `tabs` |

## 17. Recommended Implementation Sequence

1. Initialize the core steps machine.
2. Render `Root`, `List`, repeated items, separators, and current content.
3. Add prev or next trigger behavior.
4. Add optional interactive item activation and linear guards.
5. Add diagnostics for invalid status vectors or step ranges.

## 18. Anti-Patterns

- Do not mark every step as `aria-current`.
- Do not expose decorative separators to assistive technology.
- Do not skip linear validation guards in item-click paths.

## 19. Consumer Expectations and Guarantees

- Consumers may assume exactly one current step is exposed at a time.
- Consumers may assume prev or next triggers reflect edge disabled state.
- Consumers must not assume the interactive variant automatically becomes a full tabs implementation unless explicitly documented.

## 20. Platform Support Matrix

| Capability / behavior                        | Web          | Desktop      | Mobile       | SSR          | Notes                                 |
| -------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------- |
| ordered step list and current-step semantics | full support | full support | full support | full support | baseline stepper behavior             |
| interactive step activation                  | full support | full support | full support | client-only  | server still renders stable structure |

## 21. Debug Diagnostics and Production Policy

| Condition                     | Debug build behavior | Production behavior | Notes                                 |
| ----------------------------- | -------------------- | ------------------- | ------------------------------------- |
| status vector length mismatch | debug warning        | warn and ignore     | derive missing statuses as incomplete |
| invalid controlled step index | debug warning        | warn and ignore     | clamp or hold nearest valid step      |

## 22. Shared Adapter Helper Notes

| Helper concept         | Required?   | Responsibility                                      | Reused by            | Notes                                    |
| ---------------------- | ----------- | --------------------------------------------------- | -------------------- | ---------------------------------------- |
| button semantic helper | recommended | normalizes trigger activation and disabled handling | `tabs`, `pagination` | useful for interactive item surfaces too |

## 23. Framework-Specific Behavior

Dioxus can sync the controlled step prop through ordinary prop updates and keep repeated item rendering purely data-driven. `Element` children may render all step panels with hidden state rather than introducing implicit lazy mounting.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<steps::Machine>(props);

rsx! {
    div { ..machine.derive(|api| api.root_attrs()),
        // render list, items, separators, content, and prev/next triggers
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the core machine from current props.
- Render the ordered step list and content panes from the committed snapshot.
- Keep prev or next triggers as native buttons.
- Gate interactive item activation through the same linear and validation rules as trigger navigation.

## 26. Adapter Invariants

- Exactly one current step is exposed at a time.
- Decorative indicators and separators remain hidden from assistive technology when they do not carry semantic text.
- Prev or next triggers never bypass linear guards.
- Inactive content never claims current-step semantics.

## 27. Accessibility and SSR Notes

- In list mode, `aria-current="step"` belongs only on the current item.
- If interactive tab-like semantics are adopted, the adapter must switch the affected parts consistently rather than mixing list and tab roles.
- SSR must keep the same item count and current-content branch the client hydrates.

## 28. Parity Summary and Intentional Deviations

- Matches the core steps contract without intentional adapter divergence.
- Promotes current-step labeling, linear guards, trigger ownership, and optional interactive semantics into explicit Dioxus-facing rules.

## 29. Test Scenarios

- linear stepper with prev or next trigger guards
- status-driven item rendering and current-step semantics
- interactive item activation allowed only for valid steps
- current content visibility switching with stable item order

## 30. Test Oracle Notes

- Inspect DOM attrs for `aria-current`, disabled trigger state, and hidden inactive content.
- Verify step transitions through both prev/next triggers and interactive item activation.
- Use hydration tests to confirm the server and client agree on current step and visible content.

## 31. Implementation Checklist

- [ ] Render the documented repeated item, separator, content, and trigger structure.
- [ ] Expose exactly one current step.
- [ ] Keep decorative pieces hidden from assistive technology when appropriate.
- [ ] Route interactive item activation through the same linear guards as trigger navigation.
- [ ] Preserve stable count and item order across hydration.
