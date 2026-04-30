---
adapter: leptos
component: link
category: navigation
source: components/navigation/link.md
source_foundation: foundation/08-adapter-leptos.md
---

# Link — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Link`](../../components/navigation/link.md) contract onto a Leptos 0.8.x component. The adapter preserves anchor-first semantics, router interception, current-item semantics, external-link security repair, and non-anchor fallback semantics when consumers intentionally opt out of `<a>`.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Link(
    href: link::Target,
    #[prop(optional)] target: Option<String>,
    #[prop(optional)] rel: Option<String>,
    #[prop(optional)] is_current: Option<link::AriaCurrent>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] children: Children,
) -> impl IntoView
```

The adapter renders a native anchor by default. `Target::Route` stays progressively enhanced by still emitting an `href` and intercepting activation through the router only on the client.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core target, current-item, disabled, locale, and message contract.
- State parity: full parity with focused, pressed, and disabled link behavior.
- Part parity: full parity with the single `Root` part.
- Adapter additions: explicit router interception, automatic external-link `rel` repair, and non-anchor semantic repair rules.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                                                      | Ownership                | Attr source        | Notes                                         |
| --------------------- | --------- | ----------------------------------------------------------------------------- | ------------------------ | ------------------ | --------------------------------------------- |
| `Root`                | required  | `<a>` by default; consumer-selected non-anchor only when explicitly supported | adapter-owned by default | `api.root_attrs()` | Anchor-first surface for navigation semantics |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                      | Adapter-owned attrs                                                                               | Consumer attrs                         | Merge order                                                                                                             | Ownership notes                      |
| ----------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `Root`      | `href`, `target`, `rel`, `aria-current`, `aria-disabled`, scope and state attrs | router click interception, external-link security repair, non-anchor `role` and `tabindex` repair | decoration attrs and trailing handlers | required navigation and security attrs win; handlers compose adapter before consumer when preventing invalid navigation | root remains the semantic link owner |

- When `target="_blank"` and `rel` is absent, the adapter must append `noopener noreferrer`.
- When rendering a non-anchor host, the adapter must add `role="link"` and keyboard activation repair.
- Consumers must not remove `aria-current` or `aria-disabled` once emitted by the adapter.

## 6. Composition / Context Contract

`Link` is standalone. It may optionally consume routing context or environment context for current-route comparison, but missing router context must not break standard anchor navigation.

## 7. Prop Sync and Event Mapping

| Adapter prop                          | Mode       | Sync trigger              | Machine event / update path | Visible effect                                 | Notes                         |
| ------------------------------------- | ---------- | ------------------------- | --------------------------- | ---------------------------------------------- | ----------------------------- |
| `disabled`                            | controlled | signal change after mount | `SetDisabled`               | updates disabled attrs and blocks activation   | only reactive prop by default |
| `href`, `target`, `rel`, `is_current` | controlled | rerender with new props   | machine or render rebuild   | updates destination and current-item semantics | no shadow state               |

| UI event                  | Preconditions             | Machine event / callback path                | Ordering notes                                                      | Notes                                   |
| ------------------------- | ------------------------- | -------------------------------------------- | ------------------------------------------------------------------- | --------------------------------------- |
| focus                     | root receives focus       | `Focus { is_keyboard }`                      | focus-visible modality must settle before attrs are read            | native anchor focus stays authoritative |
| blur                      | root loses focus          | `Blur`                                       | clears focus-visible state after focus leaves                       | no late pointer cleanup races           |
| pointer or key press      | not disabled              | `Press` / `PressEnd`                         | no synthetic double-fire on native anchors                          | pressed state remains machine-owned     |
| click or Enter activation | not disabled              | `Navigate` then optional router interception | adapter prevents default only for `Target::Route` or disabled state | browser navigation remains the fallback |
| Space on non-anchor host  | semantic repair path only | `Navigate`                                   | only when adapter added `role="link"`                               | do not add on native anchors            |

## 8. Registration and Cleanup Contract

- No descendant registry exists.
- Router subscriptions or current-route reads are instance-local and must be dropped on unmount.
- No timers or global listeners are required for baseline link behavior.

## 9. Ref and Node Contract

| Target part / node | Ref required?                                                         | Ref owner     | Node availability                                      | Composition rule                     | Notes                                           |
| ------------------ | --------------------------------------------------------------------- | ------------- | ------------------------------------------------------ | ------------------------------------ | ----------------------------------------------- |
| `Root`             | yes when router interception or non-anchor repair needs the live node | adapter-owned | required after mount for repaired non-anchor semantics | compose with consumer ref if exposed | native anchor mode may rely mostly on DOM attrs |

## 10. State Machine Boundary Rules

- Focus-visible, pressed, disabled, and current-item state remain machine-owned.
- Router interception, automatic `rel` repair, and non-anchor semantic repair are adapter-owned.
- The adapter must not infer selection or active route state from CSS alone.

## 11. Callback Payload Contract

| Callback            | Payload source             | Payload shape                                                 | Timing                                                  | Cancelable?                                  | Notes                      |
| ------------------- | -------------------------- | ------------------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------- | -------------------------- |
| navigation callback | normalized adapter payload | `{ target: link::Target, is_route: bool, is_keyboard: bool }` | after disabled guard, before browser navigation escapes | yes, through normal event default prevention | wrapper-owned surface only |

## 12. Failure and Degradation Rules

| Condition                                      | Policy             | Notes                                                               |
| ---------------------------------------------- | ------------------ | ------------------------------------------------------------------- |
| router context unavailable for `Target::Route` | degrade gracefully | fall back to anchor navigation via rendered `href`                  |
| invalid explicit non-anchor root composition   | fail fast          | link semantics cannot be guaranteed                                 |
| external-link hint text unavailable            | degrade gracefully | keep security attrs even if announcement text falls back to default |

## 13. Identity and Key Policy

Each `Link` instance owns one semantic root. Server and client must preserve the same root host choice for hydration safety.

## 14. SSR and Client Boundary Rules

- SSR must emit an `href` for both standard URLs and client routes.
- Router interception is client-only and must never remove the progressive-enhancement path.
- Disabled links must SSR without `href` only when the core contract requires omission.

## 15. Performance Constraints

- Avoid recomputing external-link security repair in multiple code paths; derive it once per render.
- Do not attach non-anchor keyboard repair listeners to native anchors.
- Keep router comparison instance-local and avoid global polling.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type | Why it must exist first                                   | Notes                                      |
| -------------------------------- | ----------- | --------------- | --------------------------------------------------------- | ------------------------------------------ |
| button-or-anchor semantic helper | required    | semantic helper | centralizes native-anchor vs repaired non-anchor behavior | shared with `pagination` and `breadcrumbs` |
| router integration helper        | recommended | platform helper | standardizes `Target::Route` interception                 | keep fallback anchor path intact           |

## 17. Recommended Implementation Sequence

1. Build root attrs from the core API.
2. Derive external-link target and `rel` repair.
3. Add disabled and current-item semantics.
4. Add route interception for `Target::Route`.
5. Add non-anchor semantic repair only for explicit non-anchor rendering.

## 18. Anti-Patterns

- Do not prevent default on standard external or same-document anchors.
- Do not drop `href` for route targets just because client routing is available.
- Do not attach Space-key activation to native anchors.

## 19. Consumer Expectations and Guarantees

- Consumers may assume native anchors remain the default rendering target.
- Consumers may assume `target="_blank"` receives safe `rel` defaults when `rel` is absent.
- Consumers must not assume route interception is available during SSR.

## 20. Platform Support Matrix

| Capability / behavior                                 | Browser client | SSR          | Notes                                               |
| ----------------------------------------------------- | -------------- | ------------ | --------------------------------------------------- |
| standard anchor navigation and current-item semantics | full support   | full support | baseline behavior                                   |
| route interception                                    | full support   | client-only  | rendered `href` remains the fallback                |
| non-anchor semantic repair                            | full support   | full support | same host choice must be preserved across hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                     | Debug build behavior | Production behavior | Notes                              |
| --------------------------------------------- | -------------------- | ------------------- | ---------------------------------- |
| non-anchor host rendered without repair hooks | fail fast            | fail fast           | semantic mismatch                  |
| `_blank` without secure `rel`                 | debug warning        | warn and ignore     | adapter still appends safe default |

## 22. Shared Adapter Helper Notes

| Helper concept                   | Required?   | Responsibility                                             | Reused by                        | Notes                                      |
| -------------------------------- | ----------- | ---------------------------------------------------------- | -------------------------------- | ------------------------------------------ |
| button-or-anchor semantic helper | required    | distinguishes native anchor vs repaired host semantics     | `pagination`, `breadcrumbs`      | avoid duplicate keyboard-repair logic      |
| router integration helper        | recommended | normalizes route interception and current-route comparison | navigation surfaces using `Link` | keep behavior progressive-enhancement-safe |

## 23. Framework-Specific Behavior

Leptos should use the active router hooks only inside the client interception path. `Signal<bool>` is appropriate for `disabled`; all other props may remain plain values unless a wrapper intentionally makes them reactive.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<link::Machine>(props);
let attrs = machine.derive(|api| api.root_attrs());

view! {
    <a
        {..attrs.get()}
        on:click=move |ev| maybe_intercept_route(ev, props.href.clone())
    >
        {children()}
    </a>
}
```

## 25. Reference Implementation Skeleton

- Build the root attrs from the core machine.
- Resolve safe `target` and `rel` defaults.
- Render a native anchor whenever possible.
- Intercept only `Target::Route` on the client and only after disabled guards pass.

## 26. Adapter Invariants

- Native anchor rendering remains the default.
- `Target::Route` still emits a usable `href`.
- Disabled links never navigate.
- Non-anchor rendering always receives full semantic repair.

## 27. Accessibility and SSR Notes

- `aria-current` must reflect the committed current-item semantics, not optimistic router state.
- `aria-disabled` belongs on the semantic root even when `href` is omitted.
- SSR must preserve the same root host shape the client hydrates.

## 28. Parity Summary and Intentional Deviations

- Matches the core link contract without intentional adapter divergence.
- Promotes router interception, progressive enhancement, external-link security repair, and non-anchor semantic repair into explicit Leptos-facing rules.

## 29. Test Scenarios

- standard anchor navigation with current-page semantics
- route interception with preserved `href`
- disabled link that remains discoverable but non-navigable
- `_blank` link with automatic `rel` repair
- non-anchor host with `role="link"` and Space-key repair

## 30. Test Oracle Notes

- Inspect DOM attrs for `href`, `target`, `rel`, `aria-current`, and `aria-disabled`.
- Assert click behavior separately for standard URLs and `Target::Route`.
- Use keyboard tests to confirm Space only activates repaired non-anchor hosts.

## 31. Implementation Checklist

- [ ] Keep native anchor rendering as the default.
- [ ] Preserve rendered `href` for route targets.
- [ ] Append safe `rel` defaults for new-tab links when absent.
- [ ] Block navigation cleanly when disabled.
- [ ] Add keyboard repair only for explicit non-anchor hosts.
