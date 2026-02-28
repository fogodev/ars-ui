---
adapter: leptos
component: live-region
category: utility
source: components/utility/live-region.md
source_foundation: foundation/08-adapter-leptos.md
---

# LiveRegion — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`LiveRegion`](../../components/utility/live-region.md) machine to Leptos 0.8.x.

## 2. Public Adapter API

```rust
#[component] pub fn LiveRegion(...) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Event parity: announce, clear, rendered, and prop-sync events are adapter-driven.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target   | Ownership     | Attr source                               | Notes                                |
| --------------------- | --------- | -------------------------- | ------------- | ----------------------------------------- | ------------------------------------ |
| `Root`                | required  | hidden live-region `<div>` | adapter-owned | `api.part_attrs(live_region::Part::Root)` | The single announcement target node. |

## 5. Attr Merge and Ownership Rules

| Target node          | Core attrs                                    | Adapter-owned attrs                                | Consumer attrs                                                     | Merge order                                               | Ownership notes              |
| -------------------- | --------------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------------- | ---------------------------- |
| live-region root     | root attrs from the core API                  | timing or announcement bookkeeping attrs if needed | consumer root attrs only if explicitly exposed                     | core `aria-live`, `aria-atomic`, and visibility attrs win | adapter-owned root           |
| announcement content | message text/content derived from the machine | clear-then-insert placeholder state                | no direct consumer override unless a documented render prop exists | machine-owned content wins                                | adapter-owned content region |

## 6. Composition / Context Contract

Wrappers may publish an announcer handle via context.

## 7. Prop Sync and Event Mapping

Announcements are reactive. Switching between controlled sources is allowed only through the documented message prop path; timing remains machine-driven.

| Adapter prop    | Mode       | Sync trigger            | Machine event / update path | Visible effect                 | Notes                                         |
| --------------- | ---------- | ----------------------- | --------------------------- | ------------------------------ | --------------------------------------------- |
| message/content | controlled | prop change after mount | announce/update path        | updates the live-region output | repeated values may still clear then reinsert |
| politeness      | controlled | prop change after mount | live-region config update   | changes urgency semantics      | immediate sync                                |

| UI event       | Preconditions                    | Machine event / callback path   | Ordering notes                             | Notes                                |
| -------------- | -------------------------------- | ------------------------------- | ------------------------------------------ | ------------------------------------ |
| message change | hydrated live-region root exists | clear-then-insert announce path | clear phase must happen before reinsertion | avoids missed repeated announcements |

## 8. Registration and Cleanup Contract

- Announcement timers or queued work register when a message needs deferred clear/reinsert handling.
- Cleanup must cancel pending timers before the root unmounts.
- The root node itself persists for the component lifetime.

| Registered entity              | Registration trigger                               | Identity key         | Cleanup trigger                | Cleanup action                              | Notes                             |
| ------------------------------ | -------------------------------------------------- | -------------------- | ------------------------------ | ------------------------------------------- | --------------------------------- |
| queued announcement timer/work | repeated announcement or delayed clear/insert path | live-region instance | message replacement or cleanup | cancel timer/work and discard stale message | prevents late stale announcements |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                                            | Notes                                                  |
| ------------------ | ------------- | ------------- | -------------------- | ----------------------------------------------------------- | ------------------------------------------------------ |
| live-region root   | yes           | adapter-owned | required after mount | no composition unless a wrapper explicitly exposes the node | Announcements require a concrete node after hydration. |

## 10. State Machine Boundary Rules

- machine-owned state: current message, politeness mode, queue semantics, and clear-then-insert behavior.
- adapter-local derived bookkeeping: timer handles and root-node handle only.
- forbidden local mirrors: do not keep a second visible message queue outside the machine/update path.
- allowed snapshot-read contexts: message-change effects, render derivation, and cleanup for queued announcement work.

## 11. Callback Payload Contract

| Callback                                     | Payload source             | Payload shape                                             | Timing                                              | Cancelable? | Notes                     |
| -------------------------------------------- | -------------------------- | --------------------------------------------------------- | --------------------------------------------------- | ----------- | ------------------------- |
| announcer callback when exposed by a wrapper | normalized adapter payload | `{ message: String, politeness: String, repeated: bool }` | after normalization of clear-then-insert sequencing | no          | Must not fire during SSR. |

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                                          |
| ------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------ |
| announce requested before hydration or before the root exists | no-op              | Preserve structure and wait for a live root node.                              |
| timer API unavailable                                         | degrade gracefully | Prefer immediate message replacement without deferred sequencing if necessary. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                                       | Notes                                                |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| queued announcements             | composite        | yes                 | not applicable                           | root identity must remain stable across hydration                                             | Identity is message token plus live-region instance. |
| live-region root                 | instance-derived | not applicable      | not applicable                           | root structure must be present on the server when hydration-stable announcements are required | One root per live-region instance.                   |

## 14. SSR and Client Boundary Rules

- The live-region root must be present in SSR output whenever the component is expected to preserve hydration-stable announcement behavior.
- Announcement timers and message sequencing are client-only.
- The root node handle is server-safe absent and required after mount.

## 15. Performance Constraints

- Repeated identical announcements should reuse the documented clear-then-insert path instead of stacking redundant timers.
- Timer cleanup must eagerly discard stale queued work when messages change.
- Root-node replacement should be avoided; patch the existing live-region node instead.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type         | Why it must exist first                                                          | Notes                                                          |
| ---------- | ----------- | ----------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `form`     | recommended | behavioral prerequisite | Form status regions should reuse the same clear-then-insert and hydration rules. | Important when the live region is reused for status messaging. |

## 17. Recommended Implementation Sequence

1. Render the persistent live-region root.
2. Wire message and politeness sync.
3. Add clear-then-insert timing for repeated announcements.
4. Register and clean up any timers or queued work.
5. Verify SSR/hydration root stability and test oracles.

## 18. Anti-Patterns

- Do not announce before hydration or before the live-region root exists.
- Do not replace the root node instead of patching it in place.
- Do not keep stale announcement timers alive after message replacement or unmount.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior          | Browser client | SSR          | Notes                                           |
| ------------------------------ | -------------- | ------------ | ----------------------------------------------- |
| live-region root structure     | full support   | full support | The root should stay hydration-stable.          |
| announcement sequencing timers | full support   | client-only  | Timer-backed clear/insert logic is client-only. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                           | Debug build behavior | Production behavior | Notes                                                                        |
| ------------------------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------------------- |
| documented platform capability is unavailable on the active runtime | debug warning        | degrade gracefully  | Use the documented fallback path instead of inventing browser-only behavior. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos effects and cleanup manage pending announcement timers.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn LiveRegion() -> impl IntoView {
    let machine = use_machine::<live_region::Machine>(live_region::Props::default());
    let attrs = machine.derive(|api| api.part_attrs(live_region::Part::Root));
    view! { <div {..attrs.get()} /> }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = create_live_region_controller(props);
let root_ref = create_root_ref();
let timer_helper = create_announcement_timer_helper();

render_stable_live_region_root(root_ref);
sync_message_and_politeness(machine, props);
run_clear_then_insert_sequence(timer_helper, root_ref, machine.current_message());

on_cleanup(|| timer_helper.cancel_all());
```

## 26. Adapter Invariants

- The live-region root must remain present in SSR output whenever the spec relies on hydration-stable announcements.
- Announcements must not run before hydration or before the live region node exists in the document.
- Repeated announcements must preserve the documented clear-then-insert sequence.
- Timers and queued announcement work must be cancelled before unmount.
- Politeness-level differences must remain explicit and must not be collapsed into one generic announce path.

## 27. Accessibility and SSR Notes

Root must remain present and hidden visually while preserving live-region semantics.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- clear-then-insert timing
- urgent vs normal announcements

## 30. Test Oracle Notes

| Behavior                                           | Preferred oracle type | Notes                                                            |
| -------------------------------------------------- | --------------------- | ---------------------------------------------------------------- |
| live-region root presence                          | rendered structure    | Assert the root remains present and structurally stable.         |
| hydration-safe announcement behavior               | hydration structure   | Verify SSR and hydrated root identity match.                     |
| timer cleanup and repeated-announcement sequencing | cleanup side effects  | Assert stale timers/work are canceled on replacement or unmount. |

## 31. Implementation Checklist

- [ ] The live-region root is present whenever SSR/hydration stability requires it.
- [ ] Message and politeness updates follow the documented clear-then-insert rules.
- [ ] Timer and queued-work cleanup is verified.
- [ ] Rendered structure and hydration test oracles are covered.
