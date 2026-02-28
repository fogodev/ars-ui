---
adapter: leptos
component: download-trigger
category: utility
source: components/utility/download-trigger.md
source_foundation: foundation/08-adapter-leptos.md
---

# DownloadTrigger — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DownloadTrigger`](../../components/utility/download-trigger.md) contract to Leptos 0.8.x.

## 2. Public Adapter API

```rust
#[component]
pub fn DownloadTrigger(
    href: String,
    #[prop(optional)] filename: Option<String>,
    #[prop(optional)] mime_type: Option<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core props.
- Part parity: single root part only.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target        | Ownership      | Attr source        | Notes                                                |
| --------------------- | --------- | ------------------------------- | -------------- | ------------------ | ---------------------------------------------------- |
| `Root`                | required  | native `<a>`                    | adapter-owned  | `api.root_attrs()` | Carries `href`, `download`, and disabled ARIA attrs. |
| children content      | required  | consumer children inside `Root` | consumer-owned | none               | Visual label/icon content.                           |

## 5. Attr Merge and Ownership Rules

| Target node                           | Core attrs                     | Adapter-owned attrs                                                                                                                               | Consumer attrs      | Merge order                                                                                                          | Ownership notes      |
| ------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `Root` anchor                         | `api.root_attrs()`             | temporary download-fallback bookkeeping attrs if needed; `role="link"` repair when a hosting wrapper reassigns the root onto a non-anchor element | consumer root attrs | core download, href, disabled semantics, and any required `role="link"` repair win; `class`/`style` merge additively | adapter-owned root   |
| temporary anchor/object URL resources | core fallback data when needed | adapter-created DOM/resource handles                                                                                                              | none                | adapter cleanup owns these resources fully                                                                           | never consumer-owned |

## 6. Composition / Context Contract

No context contract.

## 7. Prop Sync and Event Mapping

Download-trigger props are usually non-reactive after mount unless a wrapper recreates the anchor state.

| Adapter prop | Mode                      | Sync trigger            | Machine event / update path | Visible effect                                     | Notes                                                                    |
| ------------ | ------------------------- | ----------------------- | --------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------ |
| `disabled`   | controlled                | prop change after mount | disabled sync path          | blocks activation while preserving discoverability | immediate sync                                                           |
| `href`       | non-reactive adapter prop | render time only        | included in root attrs      | changes download target                            | post-mount changes should be treated as unsupported unless reinitialized |
| `filename`   | non-reactive adapter prop | render time only        | included in root attrs      | affects download attribute / fallback naming       | same policy as `href`                                                    |

| UI event                                      | Preconditions                                                             | Machine event / callback path                                                | Ordering notes                                                 | Notes                                                            |
| --------------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------- |
| click / activation                            | not disabled                                                              | native download path or fallback path                                        | normalization decides same-origin vs fallback before callbacks | fallback may create temporary resources                          |
| click / activation on non-anchor host wrapper | root reassigned onto a non-anchor element by a hosting composition helper | normalized download path after explicit `role="link"` semantics are repaired | role repair must be in place before interaction                | keyboard and screen-reader discoverability must remain link-like |

## 8. Registration and Cleanup Contract

- No persistent registration lifecycle exists.
- Temporary anchors, object URLs, or fallback resources must be cleaned up after activation and on component cleanup.
- Blob URLs passed directly as `href` are adapter-owned cleanup concerns even when no cross-origin fallback anchor is created.

| Registered entity             | Registration trigger                     | Identity key                                    | Cleanup trigger                  | Cleanup action                                                             | Notes                                                  |
| ----------------------------- | ---------------------------------------- | ----------------------------------------------- | -------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------ |
| temporary anchor / object URL | fallback download path                   | download-trigger instance                       | activation completion or cleanup | remove temporary node / revoke URL                                         | platform-specific details belong in framework behavior |
| direct blob URL `href`        | render with `href` starting with `blob:` | current blob URL string plus component instance | `href` replacement or cleanup    | call `URL.revokeObjectURL` exactly once for the URL owned by this instance | must not revoke consumer-owned non-blob URLs           |

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

| Callback                                                               | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                                                                                 |
| ---------------------------------------------------------------------- | -------------- | ------------- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------------------- |
| no public adapter-specific callback beyond normalized component events | none           | none          | not applicable | no          | When wrappers expose callbacks, they must preserve the normalized timing documented in `Prop Sync and Event Mapping`. |

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
- SSR must render only the stable root node; temporary fallback anchors and object URLs are client-only.

## 15. Performance Constraints

- Attr maps derived from the machine should be memoized or read through adapter derivation helpers instead of rebuilt eagerly on every render.
- Listener, timer, and observer registration must be stable across rerenders and must not churn unless the governing configuration actually changes.
- Cleanup must release only the resources owned by the current component instance and must avoid repeated quadratic teardown work.

## 16. Implementation Dependencies

| Dependency                           | Required?   | Dependency type         | Why it must exist first                                                    | Notes                                                  |
| ------------------------------------ | ----------- | ----------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------ |
| browser download fallback capability | recommended | behavioral prerequisite | Temporary anchors or object URLs depend on the selected download strategy. | Platform-specific behavior remains in framework notes. |

## 17. Recommended Implementation Sequence

1. Render the root anchor with the documented attrs.
2. Wire disabled gating and activation normalization.
3. Add same-origin/cross-origin fallback handling.
4. Clean up temporary anchors or object URLs after use.

## 18. Anti-Patterns

- Do not leak temporary anchors or object URLs after activation.
- Do not ignore disabled-state discoverability when blocking activation.

## 19. Consumer Expectations and Guarantees

- Consumers may assume adapter-owned fallback resources are cleaned up after activation and on component cleanup.
- Consumers may assume non-anchor host composition still requires explicit link-semantic repair.
- Consumers must not assume consumer-owned blob URLs are revoked by the adapter unless the adapter created or adopted ownership explicitly.

## 20. Platform Support Matrix

| Capability / behavior      | Browser client | SSR            | Notes                                                         |
| -------------------------- | -------------- | -------------- | ------------------------------------------------------------- |
| direct anchor download     | full support   | SSR-safe empty | SSR renders the root only; downloads start on the client.     |
| cross-origin blob fallback | full support   | client-only    | Blob, fetch, and temporary object-URL logic are client-owned. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                    | Debug build behavior | Production behavior | Notes                                                                                                        |
| ------------------------------------------------------------ | -------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------ |
| non-anchor host reuses trigger attrs without semantic repair | debug warning        | warn and ignore     | The adapter should repair link semantics before interaction; the warning catches misuse in host composition. |
| adapter-owned blob or fallback URL cleanup path is skipped   | fail fast            | fail fast           | Cleanup must revoke or release adapter-owned temporary resources exactly once.                               |

## 22. Shared Adapter Helper Notes

| Helper concept                         | Required?      | Responsibility                                                             | Reused by                                                      | Notes                                                             |
| -------------------------------------- | -------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------- |
| hidden-input form participation helper | not applicable | No hidden-input helper is needed for this utility.                         | not applicable                                                 | Use direct root attrs plus fallback resource bookkeeping instead. |
| platform capability helper             | required       | Select web blob fallback versus Desktop or Mobile native download routing. | `download-trigger`, `drop-zone`, `dismissable`, `action-group` | Capability lookup should stay separate from event normalization.  |
| debug-warning helper                   | recommended    | Emit semantic-repair diagnostics for non-anchor host misuse.               | `download-trigger`, `as-child`, `action-group`                 | Diagnostic only; cleanup remains mandatory in all builds.         |

## 23. Framework-Specific Behavior

Blob fallback is client-only in Leptos. Cross-origin downloads may use `fetch` plus `Blob` plus `URL.createObjectURL`, while direct `blob:` href values and temporary fallback URLs must both be revoked via `web_sys::Url::revoke_object_url` during cleanup. If a hosting wrapper reassigns the root onto a non-anchor element, it must repair semantics with `role="link"` before reusing the download-trigger attrs.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn DownloadTrigger(children: Children) -> impl IntoView {
    let api = download_trigger::Api::new(&download_trigger::Props::default());
    view! { <a {..api.root_attrs()}>{children()}</a> }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = build_adapter_surface_from_props(props);
let root_attrs = derive_root_anchor_attrs(machine);
let capability = resolve_download_capability();
let cleanup = create_download_resource_cleanup_helper();

render_stable_root(root_attrs);
repair_non_anchor_link_semantics_if_needed(root_attrs);
wire_activation(|| {
    match capability {
        WebDirect => start_direct_download(),
        WebBlobFallback => start_blob_fallback_and_register_cleanup(cleanup),
        NativeFallback => route_to_platform_download_api(),
    }
});

on_cleanup(|| cleanup.release_owned_urls_and_nodes());
```

## 26. Adapter Invariants

- Disabled state must suppress activation without making the trigger undiscoverable to assistive technology.
- Same-origin and cross-origin download behavior must be documented explicitly when the adapter falls back to browser navigation.
- Rendering onto a non-anchor host must repair link semantics with `role="link"` and must preserve keyboard discoverability.
- Any generated anchor or temporary DOM node used for download initiation must be cleaned up after activation.
- Any blob URL owned or created by the adapter must be revoked on replacement or cleanup.

## 27. Accessibility and SSR Notes

Disabled state uses `aria-disabled`, not removal from discoverability. When a hosting wrapper renders the root as a non-anchor element, the adapter must add `role="link"` so assistive technology still exposes link semantics.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop and part parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for non-anchor link semantic repair, same-origin vs fallback download strategy selection, temporary anchor ownership, and blob URL cleanup.

## 29. Test Scenarios

- root anchor mapping
- disabled suppression
- cross-origin fallback path
- non-anchor host rendering repairs link semantics with `role="link"`
- blob URL cleanup on unmount or href replacement

## 30. Test Oracle Notes

| Behavior                                | Preferred oracle type | Notes                                                                                |
| --------------------------------------- | --------------------- | ------------------------------------------------------------------------------------ |
| structural rendering and part presence  | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.           |
| accessibility and state attrs           | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                       |
| link semantic repair on non-anchor host | DOM attrs             | Assert `role="link"` is present when the root is rendered onto a non-anchor element. |
| blob URL cleanup                        | cleanup side effects  | Verify adapter-owned blob URLs are revoked on replacement and component cleanup.     |

Cheap verification recipe:

1. Render the trigger onto both an anchor host and a non-anchor host, then assert `role="link"` repair only appears on the non-anchor case.
2. Drive one same-origin or direct-download path and one fallback path, then assert only the documented temporary anchor or URL resources are created.
3. Replace or unmount the href source and verify adapter-owned blob URLs are revoked exactly once.

## 31. Implementation Checklist

- [ ] Root anchor attrs and disabled semantics are correct.
- [ ] Fallback download behavior follows the documented strategy.
- [ ] Non-anchor host rendering repairs semantics with `role="link"` when the root is reassigned by a hosting wrapper.
- [ ] Temporary resources clean up correctly.
- [ ] Adapter-owned blob URLs are revoked on cleanup or href replacement.
