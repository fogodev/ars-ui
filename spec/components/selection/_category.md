# Selection Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and `Service`.
`03-accessibility.md` for focus management, ARIA patterns, and keyboard navigation.
`06-collections.md` for `CollectionState<T>` and item management.

---

## Table of Contents

- [Select](select.md)
- [Combobox](combobox.md)
- [Listbox](listbox.md)
- [Menu](menu.md)
- [ContextMenu](context-menu.md)
- [MenuBar](menu-bar.md)
- [TagsInput](tags-input.md)
- [Autocomplete](autocomplete.md)
- [SegmentGroup](segment-group.md)

---

## Overview

Selection components allow users to choose from a set of options. They range from simple
dropdowns (`Select`) to searchable inputs (`Combobox`), visible lists (`Listbox`), action menus
(`Menu`, `ContextMenu`, `MenuBar`), chip-style multi-value entry (`TagsInput`), search-driven
filtering (`Autocomplete`), and compact toggle-style selectors (`SegmentGroup`).

All selection components share:

- **Keyboard navigation** with roving focus or active-descendant pattern.
- **Controlled/uncontrolled value** via `Bindable<T>`.
- **ARIA composite widget patterns** (`listbox`, `menu`, `combobox`).
- **Collection management** via `ars-collections` for item indexing and typeahead.
- **Positioning** via `ars-a11y` for floating content placement.

### `aria-activedescendant` Validity Rule

Per the ARIA spec, `aria-activedescendant` MUST reference a valid, existing DOM element ID.
When no item is highlighted (`highlighted_key` is `None`), the attribute MUST be **omitted
entirely** — never set to an empty string (`""`) or a non-existent ID. All selection
components enforce this via the `if let Some(ref k)` guard in their `*_attrs()` methods.

### Focus Strategy: `aria-activedescendant` vs Roving Tabindex

Selection components use one of two focus management strategies:

| Strategy                | Primary Use                                                                | How It Works                                                                                                                                                      |
| ----------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aria-activedescendant` | `Select`, `Combobox`, `Listbox` (default), `Autocomplete`                  | Focus stays on the container/input; `aria-activedescendant` points to the highlighted item ID. Best for virtualized lists where not all items are in the DOM.     |
| Roving tabindex         | `Menu`, `MenuBar`, `ContextMenu`, `SegmentGroup`, `Listbox` (iOS fallback) | Focus moves directly to the highlighted item (`tabindex="0"` + `element.focus()`). All other items get `tabindex="-1"`. Required for VoiceOver iOS compatibility. |

**VoiceOver iOS Fallback**: VoiceOver on iOS does not reliably support `aria-activedescendant`.
Components that use `aria-activedescendant` as their primary strategy MUST detect iOS at
initialization time (via `navigator.userAgent` or `navigator.platform`) and fall back to
roving tabindex when running on iOS. The detection SHOULD be centralized in a shared
`use_focus_strategy()` hook or equivalent runtime check:

```rust,no_check
/// Determines the focus strategy based on the runtime environment.
pub fn resolve_focus_strategy(preferred: FocusStrategy) -> FocusStrategy {
    if preferred == FocusStrategy::ActiveDescendant && is_ios_voiceover() {
        FocusStrategy::RovingTabindex
    } else {
        preferred
    }
}

fn is_ios_voiceover() -> bool {
    // Feature-detect iOS: check navigator.userAgent for "iPad" or "iPhone"
    // and whether VoiceOver is active (prefers-reduced-motion media query
    // or UIAccessibility). Cache the result for the session lifetime.
    cfg!(target_arch = "wasm32") && /* browser detection logic */
}
```

Per-component strategy:

- **Select**: Primary = `aria-activedescendant`. iOS fallback = roving tabindex on listbox items.
- **Combobox**: Primary = `aria-activedescendant` (focus MUST stay on input for typing). iOS fallback = add `aria-selected="true"` on active option for improved VoiceOver announcements. Roving tabindex is NOT used because focus cannot leave the input. See §2.5 Connect API for implementation details.

    **Combobox iOS VoiceOver Implementation Detail:**
    When `is_ios_voiceover()` returns `true`, the `Combobox` connect code MUST:
    1. **Omit `aria-activedescendant`** from the input element entirely (do not set it even when
       an item is highlighted). iOS VoiceOver ignores this attribute and may produce confusing
       announcements when it is present.
    2. **Set `aria-selected="true"`** on the currently highlighted option (in addition to the
       existing `aria-selected` for selection state). This is the primary mechanism VoiceOver
       uses to announce the active option on iOS.
    3. **Do NOT use roving tabindex** — focus must remain on the `<input>` for typing. This
       distinguishes `Combobox` from `Select`/`Listbox`, which can use roving tabindex as their iOS
       fallback.
    4. **Virtualized lists**: When combined with `Virtualizer`, ensure the highlighted item is
       scrolled into view before setting `aria-selected="true"`. The adapter must wait for the
       DOM node to mount (via `requestAnimationFrame` or `MutationObserver`) before applying
       the attribute.

    The detection is performed once at initialization via `resolve_focus_strategy()` and stored
    in `Context` as `is_ios: bool` for use in the connect code.

- **Listbox**: Primary = `aria-activedescendant`. iOS fallback = roving tabindex.
- **Menu / ContextMenu / MenuBar**: Primary = roving tabindex (no fallback needed; already VoiceOver-compatible).
- **Autocomplete**: Primary = `aria-activedescendant`. iOS fallback = `aria-selected="true"` on active option (same as `Combobox`, focus must stay on input).
- **SegmentGroup**: Primary = roving tabindex (no fallback needed). Uses `role="radiogroup"` + `role="radio"` semantics — not collection-based.

### Keyboard Navigation with Virtualization

When a selection component uses a `Virtualizer` (see `06-collections.md` §Virtualizer),
the following rules apply:

1. **Use `aria-activedescendant`** (not roving tabindex). Virtualized lists only render
   visible items in the DOM; roving tabindex requires the target element to exist.
2. **Scroll-into-view before highlight**: When keyboard navigation (ArrowDown, ArrowUp,
   Home, End, typeahead) would highlight an item that is not currently rendered in the DOM,
   the machine MUST emit a `ScrollIntoView { key }` effect **before** setting
   `highlighted_key`. The adapter scrolls the virtualizer to make the target item visible,
   waits for the DOM node to mount, then sets `aria-activedescendant`.
3. **`aria-setsize` / `aria-posinset`**: Each rendered option MUST include these attributes
   so screen readers can report the total list size even when most items are off-screen.
4. **Roving tabindex fallback (iOS)**: When the iOS VoiceOver fallback is active and the
   list is virtualized, the adapter MUST scroll the target item into view and wait for its
   DOM node before calling `element.focus()`. This may require a `requestAnimationFrame`
   or `MutationObserver` callback to ensure the element exists.

- **Error and description linkage**: For form-participating selection controls (`Select`,
  `Combobox`, `TagsInput`), `aria-describedby` is wired on the primary interactive element
  using the same pattern as input components:

    ```rust,no_check
    // IMPORTANT: Only reference IDs for parts that are actually rendered,
    // otherwise the aria-describedby will point to a non-existent element
    // (a "dangling reference"), which confuses assistive technology.
    let mut describedby_parts = Vec::new();
    if self.ctx.has_description {
        describedby_parts.push(self.ctx.ids.part("description"));
    }
    if self.ctx.invalid {
        describedby_parts.push(self.ctx.ids.part("error-message"));
    }
    if !describedby_parts.is_empty() {
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
    }
    ```

| Component      | Value Type        | Popup    | Key ARIA Pattern                 |
| -------------- | ----------------- | -------- | -------------------------------- |
| `Select`       | `selection::Set`  | Yes      | `combobox` + `listbox`           |
| `Combobox`     | `selection::Set`  | Yes      | `combobox` + `listbox`           |
| `Listbox`      | `selection::Set`  | No       | `listbox`                        |
| `Menu`         | — (actions)       | Yes      | `menu` + `menuitem`              |
| `ContextMenu`  | — (actions)       | Yes      | `menu` (pointer-anchored)        |
| `MenuBar`      | — (actions)       | Yes      | `menubar`                        |
| `TagsInput`    | `Vec<String>`     | No       | `group` + editable tags          |
| `Autocomplete` | — (search+action) | Optional | `combobox` + `menu` or `listbox` |
| `SegmentGroup` | `String`          | No       | `radiogroup` + `radio`           |

### Selection Behavior (React Aria Parity)

All multi-select components (`Select`, `Combobox`, `Listbox`, `TagGroup`, `Table`, `GridList`) support a
`selection_behavior` prop that controls how pointer interaction affects the selection set.

> **TagGroup Clarification.** `TagGroup` is a **display-only** group of removable tags (not an
> editable input like `TagsInput`). It uses `CollectionState` to manage a keyed list of tags.
> Keyboard navigation uses **roving tabindex** across tags. Each `Tag` contains a `RemoveButton`
> sub-part (`<button aria-label="Remove {label}">`) for removal.
>
> **Anatomy:**
>
> ```text
> TagGroup          role="grid", aria-label
> └── Tag           role="row", tabindex="0|-1" (roving), data-ars-part="tag"
>     ├── TagLabel  role="gridcell", data-ars-part="tag-label"
>     └── RemoveButton  role="gridcell" > <button>, data-ars-part="tag-remove"
> ```
>
> Focus indication follows `data-ars-focus-visible` conventions. The active tag receives
> `tabindex="0"`; all others receive `tabindex="-1"` (roving tabindex pattern).
>
> `Behavior` — defined in `shared/selection-patterns.md`

#### Selection Behavior Screen Reader Announcements

- **`aria-multiselectable`**: The container element (listbox, grid, etc.) MUST set
  `aria-multiselectable="true"` when `selection_mode == Multiple` and
  `aria-multiselectable="false"` (or omit the attribute) when `selection_mode == Single`.
- **Live region on mode change**: If `selection_mode` changes dynamically at runtime
  (e.g., toggling between single and multi-select), announce the change via an
  `aria-live="polite"` region: `"Multi-select enabled"` / `"Single-select enabled"`.
- **Roving tabindex guidance**: In `Replace` behavior with `Multiple` mode, the focused
  item uses `tabindex="0"` while all other items use `tabindex="-1"`. Selected items
  are indicated by `aria-selected="true"`, not by focus position. This ensures keyboard
  users can navigate freely without accidentally changing selection.

When `selection_behavior` is `Replace`:

- Click on an item → replaces selection with that single item
- Ctrl/Cmd + Click → toggles the clicked item in/out of the selection
- Shift + Click → selects the range from the last selected item to the clicked item
- The machine adds `RangeSelect { anchor: Key, target: Key }` and
  `SelectItemCtrl(Key)` event variants to support this behavior

#### Extended Selection: Ctrl/Cmd+Click (SelectItemCtrl)

When `selection_behavior` is `Replace` and `selection_mode` is `Multiple`, Ctrl+Click
(or Cmd+Click on macOS) toggles a single item in or out of the current selection without
affecting other selected items. This matches desktop OS conventions (Finder, Explorer).

**Event variant:**

```rust,no_check
/// Emitted when Ctrl/Cmd+Click toggles a single item in a Replace-mode multi-select.
/// The adapter translates `pointerup` with `ctrlKey` (or `metaKey` on macOS) into this event.
Event::SelectItemCtrl(Key)
```

**Adapter translation:**

The adapter layer intercepts `pointerup` events on item elements and checks modifier keys:

- `pointerup` with no modifier → `Event::SelectItem(key)` (replace selection)
- `pointerup` with `ctrlKey` (Windows/Linux) or `metaKey` (macOS) → `Event::SelectItemCtrl(key)`
- `pointerup` with `shiftKey` → `Event::RangeSelect { anchor: last_selected_key, target: key }`

```rust,no_check
// Adapter-level pointer event handler (Leptos example):
fn on_item_pointerup(key: Key, event: web_sys::PointerEvent, send: &dyn Fn(Event)) {
    let is_ctrl = if cfg!(target_os = "macos") {
        event.meta_key()
    } else {
        event.ctrl_key()
    };

    if event.shift_key() {
        send(Event::RangeSelect { anchor: /* last_selected_key */, target: key });
    } else if is_ctrl {
        send(Event::SelectItemCtrl(key));
    } else {
        send(Event::SelectItem(key));
    }
}
```

**Keyboard equivalent:** `Space` with Ctrl/Cmd held toggles the focused item (same as
`SelectItemCtrl`). The adapter translates `keydown("Space")` with the modifier into
`Event::SelectItemCtrl(highlighted_key)`.

**Transition handler:**

```rust,no_check
/// Toggle a single item without affecting the rest of the selection.
/// Only effective when selection_behavior == Replace && selection_mode == Multiple.
(_, Event::SelectItemCtrl(key)) => {
    if ctx.selection_state.behavior != selection::Behavior::Replace { return None; }
    if ctx.selection_state.mode != selection::Mode::Multiple { return None; }
    if ctx.selection_state.is_disabled(&key) { return None; }
    let key = key.clone();
    Some(TransitionPlan::context_only(move |ctx| {
        if ctx.selection.get().contains(&key) {
            let new_sel = ctx.selection_state.deselect(&key);
            ctx.selection.set(new_sel);
        } else {
            let new_sel = ctx.selection_state.toggle(key.clone());
            ctx.selection.set(new_sel);
        }
        ctx.last_selected_key = Some(key);
    }))
}
```

**Virtualized list support:** `SelectItemCtrl` works identically with virtualized collections.
The adapter only needs the item's `Key`, not a DOM reference, so off-screen items that have
been scrolled into view and clicked work without special handling.

#### Range Selection (Shift+Click / Shift+Arrow)

Range selection allows users to select a contiguous span of items by holding Shift and
clicking or arrow-navigating. The selection context tracks the anchor point, and the
machine computes the inclusive range between anchor and target.

**Event variant:**

```rust,no_check
/// Emitted when Shift+Click or Shift+Arrow extends a selection range.
Event::RangeSelect { anchor: Key, target: Key }
```

**Context extension:**

```rust,no_check
/// Added to the shared selection context alongside `selection::State`.
/// Tracks the last explicitly selected key, serving as the anchor for
/// subsequent range operations.
pub last_selected_key: Option<Key>,
```

**Shift+Click behavior:**
When the user Shift-clicks an item, the machine emits `RangeSelect` where `anchor` is
`last_selected_key` (falling back to the first item in the collection if `None`) and
`target` is the clicked item's key. All items in the inclusive range are added to the
current selection. `last_selected_key` is NOT updated — the anchor remains stable so that
subsequent Shift-clicks extend from the same origin.

**Shift+Arrow behavior:**
When the user presses Shift+ArrowDown or Shift+ArrowUp, selection extends one item at a
time. Each press emits `RangeSelect` with the same `anchor` and the new focus target.
This matches native OS list selection (e.g., Finder, Windows Explorer).

**Range computation:**
The machine iterates the collection from `anchor` to `target` (or `target` to `anchor` if
target precedes anchor) using `Collection::iter_keys()`. All keys in the traversed span,
inclusive of both endpoints, are added to the selection set.

**Disabled key skipping:**
Items with `disabled: true` within the computed range are excluded from selection. They are
still traversed for range boundary purposes — they do not break the range.

```rust
/// Computes the set of keys in the inclusive range [anchor, target],
/// skipping disabled items.
fn compute_range(
    collection: &impl Collection,
    anchor: &Key,
    target: &Key,
) -> Vec<Key> {
    let keys = collection.iter_keys().collect::<Vec<Key>>();

    let anchor_idx = keys.iter().position(|k| k == anchor);
    let target_idx = keys.iter().position(|k| k == target);

    let (start, end) = match (anchor_idx, target_idx) {
        (Some(a), Some(t)) if a <= t => (a, t),
        (Some(a), Some(t))            => (t, a),
        _ => return vec![],
    };

    keys[start..=end]
        .iter()
        .filter(|k| !collection.is_disabled(k))
        .cloned()
        .collect()
}

/// Transition handler for RangeSelect inside the selection state machine.
fn handle_range_select(
    ctx: &mut selection::Context,
    collection: &impl Collection,
    anchor: &Key,
    target: &Key,
) {
    let range_keys = compute_range(collection, anchor, target);
    for key in &range_keys {
        ctx.selection.insert(key.clone());
    }
    // Anchor is NOT updated — it stays at last_selected_key so that
    // subsequent Shift-clicks extend from the same origin.
    ctx.focused_key = Some(target.clone());
}
```

### Touch Interaction Patterns

Selection components must handle touch input distinctly from mouse/keyboard. Adapters
use `ctx.pointer_type: PointerType` to distinguish input modality and adjust behavior:

> `PointerType` — defined in `foundation/05-interactions.md`

Touch-specific behaviors:

- **Tap** = action (select item). Equivalent to click for mouse input.
- **Long-press** = enter selection mode (multi-select). When `selection_mode` is `Multiple`,
  a long-press on an item enters selection mode where subsequent taps toggle items.
- **Touch + drag** = range selection. When in selection mode, dragging across items selects
  the contiguous range from the drag start to the current touch position.

Adapters SHOULD detect pointer type via `pointerdown` event's `pointerType` property and
store it in `ctx.pointer_type` so that state machine transitions and rendering logic can
branch on input modality. For example, touch mode may show selection checkboxes that are
hidden in mouse/keyboard mode.

---

## Appendix: Shared Selection Patterns

### Navigation

All selection components (`Select`, `Combobox`, `Listbox`, `Menu`, `MenuBar`) use `Collection::key_after`
/ `key_before` for navigation, with `next_enabled_key` / `prev_enabled_key` from
`ars-collections` (see `06-collections.md` §3.3.2) for disabled-item awareness. Structural
nodes (`Section`, `Header`, `Separator`) are skipped automatically by the `Collection` trait.

### Typeahead

`Typeahead` is handled by `typeahead::State::process_char()` from `ars-collections`
(see `06-collections.md` §3.4), which accepts any `&C: Collection<T>` and returns
`(typeahead::State, Option<Key>)`. Each component stores `typeahead: typeahead::State` in
its `Context`.

The adapter is responsible for scheduling the 500ms timeout cleanup via `PendingEffect`:

```rust,no_check
(_, Event::TypeaheadSearch(ch, now_ms)) => {
    let (new_ta, found) = ctx.typeahead.process_char(*ch, *now_ms,
        ctx.highlighted_key.as_ref(), &ctx.items);
    Some(TransitionPlan::context_only(move |ctx| {
        ctx.typeahead = new_ta;
        if let Some(k) = found { ctx.highlighted_key = Some(k); }
    }).cancel_effect("typeahead_timeout")
      .with_effect(PendingEffect::new("typeahead_timeout", |ctx, _props, send| {
        // Adapter sets a 500ms timeout; previous timeout is cancelled
        // via cancel_effect above, then a fresh one is scheduled.
        let platform = use_platform_effects();
        let send = send.clone();
        let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
            send(Event::TypeaheadClear);
        }));
        let pc = platform.clone();
        Box::new(move || { pc.clear_timeout(handle); })
    })))
}
```

Matching uses `ars_i18n::Collator` for locale-aware, case-insensitive, accent-insensitive
comparison.

**TypeaheadClear Semantics**: `Event::ClearTypeahead` is a context-only update (no state
change). It clears the typeahead buffer but does NOT reset `highlighted_key` — the previously
matched item remains highlighted. This is intentional: clearing the buffer is a timeout
cleanup, not a navigation reset. Guard: `ClearTypeahead` is only valid when `typeahead_buffer`
is non-empty. Adapters must ensure the `ClearTypeahead` timeout is cancelled and re-created on
each new keystroke to avoid race conditions.

**IME Composition Interleaving with Selection Events**: During IME composition
(`is_composing=true`):

1. `TypeaheadSearch` events are suppressed (not queued).
2. Arrow key events ARE processed normally — composition doesn't block navigation.
3. On `compositionend`, if the composed text matches a typeahead pattern, it contributes to
   a new typeahead search.
4. The machine transition guard checks `context.is_composing` for `TypeaheadSearch` and
   returns `Ignored` when `true`.

### Positioning Pattern

Used by `Select`, `Combobox`, `Menu`, `ContextMenu`.

All positioning types (`Placement`, `PositioningOptions`, `Strategy`, `Offset`, `Boundary`, `PositioningResult`) are defined in `11-dom-utilities.md` §2.2. Key field mappings for selection components:

- `placement: Placement` — typically `BottomStart` for dropdown-style components
- `strategy: Strategy` — `Absolute` (default) or `Fixed` for portal-based rendering
- `shift: bool` — keeps the listbox within the viewport (canonical name; equivalent to `slide` in earlier drafts)
- `boundary: Boundary` — `Viewport` (default) or `Element(ref)` for custom containers

In RTL, `Start` and `End` placements are mirrored automatically via `Placement::resolve_logical(dir)`.
