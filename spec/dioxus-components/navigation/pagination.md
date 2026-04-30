---
adapter: dioxus
component: pagination
category: navigation
source: components/navigation/pagination.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Pagination — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Pagination`](../../components/navigation/pagination.md) contract onto a Dioxus 0.7.x component. The adapter preserves page-range derivation, localized trigger labels, anchor-vs-button rendering from `get_page_url`, and adapter-owned live announcements for page changes.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct PaginationProps {
    #[props(optional)]
    pub page: Option<u32>,
    pub default_page: u32,
    pub page_size: u32,
    pub total_items: u32,
    pub sibling_count: u32,
    pub boundary_count: u32,
    #[props(optional)]
    pub get_page_url: Option<pagination::GetPageUrl>,
}

#[component]
pub fn Pagination(props: PaginationProps) -> Element
```

The adapter owns visible range derivation, repeated page-trigger rendering, and the hidden live-region announcement surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with page, page size, total item count, labels, and optional URL generation.
- State parity: full parity with the core single-page selection model.
- Part parity: full parity with `Root`, `PrevTrigger`, `NextTrigger`, repeated `PageTrigger`, and `Ellipsis`.
- Adapter additions: explicit anchor-vs-button rendering policy and live-region ownership.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target | Ownership     | Attr source                    | Notes                                             |
| --------------------- | ----------- | ------------------------ | ------------- | ------------------------------ | ------------------------------------------------- |
| `Root`                | required    | `<nav>`                  | adapter-owned | `api.root_attrs()`             | owns localized landmark label                     |
| `PrevTrigger`         | required    | `<button>` or `<a>`      | adapter-owned | `api.prev_trigger_attrs()`     | host depends on `get_page_url`                    |
| `NextTrigger`         | required    | `<button>` or `<a>`      | adapter-owned | `api.next_trigger_attrs()`     | host depends on `get_page_url`                    |
| `PageTrigger`         | repeated    | `<button>` or `<a>`      | adapter-owned | `api.page_trigger_attrs(page)` | current page gets `aria-current`                  |
| `Ellipsis`            | conditional | `<span>`                 | adapter-owned | `api.ellipsis_attrs()`         | decorative visual separator                       |
| live region           | required    | hidden `<div>`           | adapter-owned | adapter-owned attrs            | announces page changes only after user navigation |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                                       | Adapter-owned attrs                                         | Consumer attrs                           | Merge order                            | Ownership notes                                           |
| ------------- | ------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------------- | -------------------------------------- | --------------------------------------------------------- |
| `Root`        | localized `aria-label`, scope and part attrs     | live-region placement                                       | wrapper decoration only if later exposed | landmark semantics win                 | `Root` stays adapter-owned                                |
| trigger hosts | labels, disabled/current attrs, page index attrs | anchor-vs-button host selection                             | decoration attrs and trailing handlers   | required ARIA and navigation attrs win | disabled anchor hosts use `aria-disabled`, not `disabled` |
| `Ellipsis`    | `aria-hidden="true"`, separator role             | hidden summary text if exposed to assistive tech separately | none                                     | decorative attrs win                   | never interactive by default                              |

## 6. Composition / Context Contract

`Pagination` is standalone. It does not publish required child context. It may compose the utility `live-region` helper internally, but consumers do not interact with that context directly.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                  | Mode       | Sync trigger            | Machine event / update path | Visible effect                                      | Notes                                            |
| ------------------------------------------------------------- | ---------- | ----------------------- | --------------------------- | --------------------------------------------------- | ------------------------------------------------ |
| `page`                                                        | controlled | prop change after mount | `SetPage`                   | updates current page and visible range              | no controlled/uncontrolled switching after mount |
| `page_size`, `total_items`, `sibling_count`, `boundary_count` | controlled | rerender with new props | core prop rebuild           | recomputes visible range and disabled edge triggers | no shadow state                                  |
| `get_page_url`                                                | controlled | rerender with new props | adapter host selection      | switches trigger host between `<button>` and `<a>`  | page semantics stay identical                    |

| UI event                      | Preconditions                           | Machine event / callback path              | Ordering notes                                      | Notes                                     |
| ----------------------------- | --------------------------------------- | ------------------------------------------ | --------------------------------------------------- | ----------------------------------------- |
| prev or next activation       | not at edge                             | `GoToPrevPage` / `GoToNextPage`            | announcement fires only after committed page change | host may be button or anchor              |
| page activation               | page is visible and not already current | `SetPage(page)`                            | announcement uses committed page number             | buttons and anchors share callback timing |
| route-style anchor navigation | `get_page_url` present                  | optional browser navigation after callback | adapter does not block navigation by default        | current page still uses `aria-current`    |

## 8. Registration and Cleanup Contract

- No descendant registry exists.
- The hidden live region owns any clear-then-insert timer used for repeated announcements.
- Cleanup must cancel pending live-region work before unmount.

## 9. Ref and Node Contract

| Target part / node      | Ref required? | Ref owner     | Node availability    | Composition rule | Notes                                |
| ----------------------- | ------------- | ------------- | -------------------- | ---------------- | ------------------------------------ |
| trigger hosts           | no            | adapter-owned | always structural    | no composition   | native semantics carry most behavior |
| hidden live-region root | yes           | adapter-owned | required after mount | no composition   | needed for announcement timing       |

## 10. State Machine Boundary Rules

- Current page and visible-range derivation remain core-owned.
- Trigger host selection, disabled anchor semantics, and page-change announcement timing are adapter-owned.
- The adapter must not mirror current page in an unsynchronized local signal.

## 11. Callback Payload Contract

| Callback             | Payload source           | Payload shape                    | Timing                               | Cancelable? | Notes                  |
| -------------------- | ------------------------ | -------------------------------- | ------------------------------------ | ----------- | ---------------------- |
| page-change callback | machine-derived snapshot | `{ page: u32, page_count: u32 }` | after committed `SetPage` transition | no          | wrappers may expose it |

## 12. Failure and Degradation Rules

| Condition                           | Policy             | Notes                                                      |
| ----------------------------------- | ------------------ | ---------------------------------------------------------- |
| `page` exceeds computed page count  | warn and ignore    | clamp or preserve nearest valid page per core behavior     |
| `get_page_url` returns unusable URL | degrade gracefully | fall back to button host for that trigger                  |
| live-region timing unavailable      | degrade gracefully | announce immediately without delayed clear-insert sequence |

## 13. Identity and Key Policy

Page triggers use stable identity from page number. Ellipsis identity follows its visible range position. Server and client must agree on the initial visible range and current page.

## 14. SSR and Client Boundary Rules

- SSR renders the current range, current-page attrs, and edge-trigger disabled state from initial props.
- The live-region root may SSR as an empty hidden node; announcements remain client-only.
- Anchor-vs-button host choice must be identical across server and client for each trigger.

## 15. Performance Constraints

- Derive the visible range once per reactive change, not per rendered trigger.
- Reuse the hidden live-region helper instead of building duplicate announcers.
- Avoid rebuilding URL strings more than once per trigger render.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type   | Why it must exist first                                | Notes                                |
| -------------------------------- | ----------- | ----------------- | ------------------------------------------------------ | ------------------------------------ |
| `live-region`                    | required    | behavioral helper | page-change announcements are adapter-owned            | reuse the clear-then-insert contract |
| button-or-anchor semantic helper | recommended | semantic helper   | normalizes disabled button vs disabled anchor behavior | shared with `link`                   |

## 17. Recommended Implementation Sequence

1. Initialize the core pagination machine.
2. Render `Root` and repeated visible triggers.
3. Add anchor-vs-button host selection from `get_page_url`.
4. Add the hidden live-region surface and page-change announcements.
5. Add final diagnostics for invalid ranges or URLs.

## 18. Anti-Patterns

- Do not use the `disabled` HTML attribute on anchor hosts.
- Do not announce the initial page on first render.
- Do not make `Ellipsis` interactive unless a higher-level design explicitly extends the contract.

## 19. Consumer Expectations and Guarantees

- Consumers may assume buttons and anchors expose the same page-change semantics.
- Consumers may assume the current page gets `aria-current="page"`.
- Consumers must not assume first or last trigger parts exist beyond the documented API.

## 20. Platform Support Matrix

| Capability / behavior                         | Web          | Desktop      | Mobile       | SSR          | Notes                            |
| --------------------------------------------- | ------------ | ------------ | ------------ | ------------ | -------------------------------- |
| current-page semantics and visible page range | full support | full support | full support | full support | baseline pagination behavior     |
| live-region page-change announcement          | full support | full support | full support | client-only  | initial render stays silent      |
| anchor-vs-button trigger host selection       | full support | full support | full support | full support | host must match across hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                               | Debug build behavior | Production behavior | Notes                             |
| --------------------------------------- | -------------------- | ------------------- | --------------------------------- |
| invalid controlled page outside range   | debug warning        | warn and ignore     | clamp or retain last valid page   |
| unusable URL returned by `get_page_url` | debug warning        | degrade gracefully  | render affected trigger as button |

## 22. Shared Adapter Helper Notes

| Helper concept                   | Required?   | Responsibility                                          | Reused by             | Notes                                  |
| -------------------------------- | ----------- | ------------------------------------------------------- | --------------------- | -------------------------------------- |
| live-region helper               | required    | owns page-change announcement sequencing                | `tabs`, `steps`       | adapter-owned hidden announcer         |
| button-or-anchor semantic helper | recommended | normalizes button vs anchor hosts and disabled behavior | `link`, `breadcrumbs` | especially important for edge triggers |

## 23. Framework-Specific Behavior

Dioxus can watch the controlled page prop through ordinary prop synchronization and keep the hidden live-region node instance-local. The host selection logic can stay render-driven because it depends only on current props and page numbers.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<pagination::Machine>(props);
let range = machine.derive(|api| api.visible_range());

rsx! {
    nav { ..machine.derive(|api| api.root_attrs()),
        // render prev, visible pages, ellipses, and next
        LiveRegion {}
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the pagination machine from the committed props.
- Derive the visible range and current-page attrs once per change.
- Render each trigger as `<a>` only when `get_page_url` yields a usable URL; otherwise render `<button>`.
- Announce only committed page changes after mount.

## 26. Adapter Invariants

- Trigger semantics stay identical whether the host is an anchor or button.
- Disabled anchor hosts use `aria-disabled`, not `disabled`.
- `Ellipsis` remains decorative.
- Page-change announcements never fire on initial render.

## 27. Accessibility and SSR Notes

- `Root` owns the localized pagination label.
- The current page trigger carries `aria-current="page"` regardless of host type.
- SSR must not omit the hidden live-region root if hydration tests rely on stable structure.

## 28. Parity Summary and Intentional Deviations

- Matches the core pagination contract without intentional adapter divergence.
- Promotes host selection, localized labels, disabled anchor semantics, and page-change announcements into explicit Dioxus-facing guidance.

## 29. Test Scenarios

- button-host pagination without URLs
- anchor-host pagination driven by `get_page_url`
- current-page `aria-current` semantics
- edge-trigger disabled state on first and last pages
- page-change live announcement after user navigation

## 30. Test Oracle Notes

- Inspect trigger hosts and attrs for `aria-current`, `aria-disabled`, and labels.
- Verify URL-backed triggers still update committed page state correctly.
- Assert live-region content stays empty on initial render and updates after page changes only.

## 31. Implementation Checklist

- [ ] Render `PrevTrigger`, visible `PageTrigger`s, `Ellipsis`, and `NextTrigger` from the derived range.
- [ ] Keep anchor and button trigger semantics aligned.
- [ ] Use `aria-disabled` for disabled anchor hosts.
- [ ] Own page-change announcements inside the adapter.
- [ ] Keep the initial render silent.
