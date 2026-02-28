---
adapter: leptos
component: tag-group
category: data-display
source: components/data-display/tag-group.md
source_foundation: foundation/08-adapter-leptos.md
---

# TagGroup — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TagGroup`](../../components/data-display/tag-group.md) contract onto a Leptos 0.8.x component. The adapter must preserve removable-tag behavior, optional selection, keyboard navigation, and the adapter-owned description, field-error, and removal-announcement concerns.

## 2. Public Adapter API

```rust
#[component]
pub fn TagGroup(
    #[prop(optional)] id: Option<String>,
    items: StaticCollection<tag_group::Tag>,
    #[prop(optional, into)] selected_keys: Option<Signal<BTreeSet<Key>>>,
    #[prop(optional)] selection_mode: Option<selection::Mode>,
    #[prop(optional)] disallow_empty_selection: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] messages: Option<tag_group::Messages>,
    #[prop(optional)] on_selection_change: Option<Callback<BTreeSet<Key>>>,
    #[prop(optional)] on_remove: Option<Callback<Key>>,
    #[prop(optional)] description: Option<Children>,
    #[prop(optional)] field_error: Option<Children>,
    #[prop(optional)] empty_state: Option<Children>,
) -> impl IntoView
```

The adapter owns repeated tag rendering, remove buttons, description and field-error slots, and empty-state behavior.

## 3. Mapping to Core Component Contract

- Props parity: full parity with explicit adapter slots for description, field error, and empty state.
- Part parity: full parity for `Root`, repeated tag cells, and remove controls; `Description`, `FieldError`, and `EmptyState` remain adapter-owned structural nodes.
- Traceability note: this spec promotes removal announcements, adapter-level description/error rendering, RTL keyboard behavior, and per-tag disabled semantics from the agnostic spec.

## 4. Part Mapping

| Core part / structure | Required?         | Adapter rendering target | Ownership     | Attr source                 | Notes                           |
| --------------------- | ----------------- | ------------------------ | ------------- | --------------------------- | ------------------------------- |
| `Root`                | required          | `<div>`                  | adapter-owned | `api.root_attrs()`          | Uses grid-style semantics.      |
| `Tag`                 | repeated          | `<div>`                  | adapter-owned | `api.tag_attrs(key)`        | One per item.                   |
| `TagRemove`           | optional per item | `<button>`               | adapter-owned | `api.tag_remove_attrs(key)` | Emits removal when allowed.     |
| `Description`         | optional          | adapter-owned wrapper    | adapter-owned | adapter-local attrs         | Adapter concern only.           |
| `FieldError`          | optional          | adapter-owned wrapper    | adapter-owned | adapter-local attrs         | Adapter concern only.           |
| `EmptyState`          | optional          | adapter-owned wrapper    | adapter-owned | adapter-local attrs         | Rendered when `items` is empty. |

## 5. Attr Merge and Ownership Rules

- Core attrs include root grid semantics, tag disabled and selected state, and remove-button labels.
- The adapter owns described-by wiring that links root, description, and field-error content.
- Consumer decoration must not replace tag or remove-button hosts or drop removal labels.

## 6. Composition / Context Contract

`TagGroup` is self-contained. It may internally publish group metadata to repeated tag helpers, but it does not expose a public context contract.

## 7. Prop Sync and Event Mapping

| Adapter prop / event              | Mode          | Sync trigger              | Machine event / update path | Notes                                     |
| --------------------------------- | ------------- | ------------------------- | --------------------------- | ----------------------------------------- |
| `selected_keys`                   | controlled    | signal change after mount | selection-related events    | Controlled selection stays machine-owned. |
| focus and blur                    | adapter event | root or tag focus         | `Focus` / `Blur`            | Preserves AT focus even when disabled.    |
| arrow/home/end keys               | adapter event | keyboard interaction      | focus navigation events     | RTL reverses horizontal direction.        |
| delete/backspace or remove button | adapter event | keyboard or click         | `RemoveTag`                 | Fires `on_remove` after machine update.   |
| selection toggling                | adapter event | click or key press        | selection events            | Suppressed for disabled tags.             |

## 8. Registration and Cleanup Contract

- The adapter owns registration of repeated tags for keyboard navigation.
- If a shared live announcer is used for removal messages, cleanup must release the announcer handle on unmount.

## 9. Ref and Node Contract

The root may need a live ref for described-by wiring and focus entry. Individual tag refs are optional and only needed for richer focus restoration after removal.

## 10. State Machine Boundary Rules

- Machine-owned state: focused key, selected keys, disabled state, and current item list.
- Adapter-owned derived values: described-by composition, description/error slots, and empty-state rendering.
- Forbidden mirror: do not maintain a second mutable tag list outside the machine during removals.

## 11. Callback Payload Contract

| Callback              | Payload source         | Payload shape   | Timing                    | Cancelable? | Notes                                                         |
| --------------------- | ---------------------- | --------------- | ------------------------- | ----------- | ------------------------------------------------------------- |
| `on_selection_change` | adapter observation    | `BTreeSet<Key>` | after selection updates   | no          | Reflects post-machine selection.                              |
| `on_remove`           | `RemoveTag` transition | `Key`           | after tag removal applies | no          | Fire after focus relocation and live announcement scheduling. |

## 12. Failure and Degradation Rules

| Condition                                       | Policy             | Notes                                            |
| ----------------------------------------------- | ------------------ | ------------------------------------------------ |
| removal requested for a disabled or missing tag | no-op              | Leave focus and selection unchanged.             |
| description or field-error slot omitted         | degrade gracefully | Described-by wiring only includes present nodes. |
| empty item set with no empty-state slot         | degrade gracefully | Render the root with no tags.                    |

## 13. Identity and Key Policy

Tag identity is keyed by `Tag.key`. Remove buttons derive identity from their parent tag key. Focus restoration after removal uses neighboring tag keys, not indices alone.

## 14. SSR and Client Boundary Rules

- SSR renders the initial tag list, description/error slots, and empty state if applicable.
- Removal announcements and keyboard navigation run only after mount.
- The number and order of initial tags must match between server and client.

## 15. Performance Constraints

- Keep selection and disabled lookups keyed by `Key`.
- Removal announcements should reuse a shared live-region helper rather than allocate one per tag.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type      | Why it must exist first                     | Notes                                |
| ------------------------ | ----------- | -------------------- | ------------------------------------------- | ------------------------------------ |
| live announcement helper | recommended | accessibility helper | Announces removals.                         | Shared with `table` and `meter`.     |
| collection helper        | required    | registration helper  | Stable key iteration and focus restoration. | Shared with `grid-list` and `table`. |

## 17. Recommended Implementation Sequence

1. Initialize the machine from items and selection props.
2. Render root and tag list with optional description/error nodes.
3. Wire remove buttons and keyboard navigation.
4. Observe selection and removal for callbacks.
5. Add removal announcements and empty-state rendering.

## 18. Anti-Patterns

- Do not treat tag removal as a purely visual delete without focus management.
- Do not hide description or field-error content inside tag renderers.
- Do not let disabled tags participate in removal or selection.

## 19. Consumer Expectations and Guarantees

- Consumers may assume description and field-error are adapter-owned concerns.
- Consumers may assume removals restore focus predictably.
- Consumers must not assume tags are link targets; the adapter preserves non-link semantics.

## 20. Platform Support Matrix

| Capability / behavior               | Browser client | SSR            | Notes                                 |
| ----------------------------------- | -------------- | -------------- | ------------------------------------- |
| tag selection and removal semantics | full support   | full support   | Structure and attrs are server-safe.  |
| removal announcements               | client-only    | SSR-safe empty | Announcement helpers run after mount. |
| RTL key reversal                    | full support   | full support   | Adapter-owned directional mapping.    |

## 21. Debug Diagnostics and Production Policy

| Condition                              | Debug build behavior | Production behavior | Notes                                                  |
| -------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------ |
| duplicate tag keys                     | fail fast            | fail fast           | Focus restoration and selection depend on unique keys. |
| remove button missing accessible label | debug warning        | warn and ignore     | Visual rendering still succeeds.                       |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                          | Reused by            | Notes                             |
| ------------------------ | ----------- | --------------------------------------- | -------------------- | --------------------------------- |
| live announcement helper | recommended | Announce tag removals.                  | `table`, `meter`     | Reuse a single hidden region.     |
| collection helper        | required    | Stable focus restoration after removal. | `grid-list`, `table` | Prefer key-based neighbor lookup. |

## 23. Framework-Specific Behavior

Leptos 0.8.x can keep tag registration keyed by `Key` and use effect-based described-by composition when optional description or error content mounts or unmounts.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn TagGroup(items: StaticCollection<tag_group::Tag>) -> impl IntoView {
    let machine = use_machine::<tag_group::Machine>(tag_group::Props {
        items: items.clone(),
        ..Default::default()
    });

    view! {
        <div {..attr_map_to_leptos(machine.derive(|api| api.root_attrs()).get(), &use_style_strategy(), None).attrs}>
            <For
                each=move || items.clone()
                key=|item| item.key.clone()
                children=move |item| view! {
                    <div {..attr_map_to_leptos(machine.derive(move |api| api.tag_attrs(&item.key)).get(), &use_style_strategy(), None).attrs}>
                        <button {..attr_map_to_leptos(machine.derive(move |api| api.tag_remove_attrs(&item.key)).get(), &use_style_strategy(), None).attrs} />
                    </div>
                }
            />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, one key-based focus-restoration helper, optional described-by composition for description and field error, and one announcement helper path for removals.

## 26. Adapter Invariants

- Remove controls stay adapter-owned and labeled.
- Description and field-error slots are outside repeated tag markup.
- Focus restoration after removal uses stable neighbor selection.

## 27. Accessibility and SSR Notes

- Root semantics and described-by wiring must include description and field-error only when present.
- Removal announcements should use localized messages with tag labels.
- SSR must keep the initial tag order and described-by structure stable.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of description, field error, and empty state.
- Intentional deviations: tag links and escape-key selection clearing remain intentionally unsupported as described in the agnostic parity notes.

## 29. Test Scenarios

1. Tag removal updates focus to the correct neighbor and fires `on_remove`.
2. Description and field-error content wire into root described-by only when present.
3. Disabled tags remain non-removable and non-selectable.

## 30. Test Oracle Notes

- Preferred oracle for removal behavior: inspect focus target and callback order after deleting a tag.
- Preferred oracle for accessibility: inspect described-by references plus remove-button labels.
- Verification recipe: remove middle, first, and last tags and confirm focus restoration chooses the documented neighbor each time.

## 31. Implementation Checklist

- [ ] Tag identity is key-based.
- [ ] Remove controls are explicit and localized.
- [ ] Description and field-error remain adapter-owned.
- [ ] Removal announcements and focus restoration are cleanup-safe.
- [ ] Tests cover removal, described-by wiring, and disabled-tag behavior.
