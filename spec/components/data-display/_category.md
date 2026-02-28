# Data Display Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and crate structure,
`03-accessibility.md` for ARIA patterns, focus management, and keyboard navigation,
`04-internationalization.md` for `NumberFormatter`, `PluralCategory`, and RTL layout,
`05-interactions.md` for pointer and keyboard event handling.

---

## 1. Overview

Data display components present information to the user in structured, meaningful ways. They
span tabular data (Table), identity representation (Avatar), task/measurement progress
(Progress, Meter), subjective ratings (RatingGroup), categorical labels (Badge), metric
summaries (Stat), removable tag groups (TagGroup), keyboard-navigable grid lists
(GridList), scrolling content (Marquee), and loading placeholders (Skeleton).

| Component     | Tier      | Purpose                                                                     |
| ------------- | --------- | --------------------------------------------------------------------------- |
| `Table`       | complex   | Sortable, selectable data grid with expandable rows                         |
| `Avatar`      | stateful  | User identity with image and initials fallback                              |
| `Progress`    | stateful  | Task completion bar/spinner, determinate or indeterminate                   |
| `Meter`       | stateless | Measurement gauge with semantic low/high/optimum zones                      |
| `RatingGroup` | stateful  | Interactive or read-only star rating                                        |
| `Badge`       | stateless | Small status/count/category label chip                                      |
| `Stat`        | stateless | Key metric with label, value, and trend indicator                           |
| `TagGroup`    | stateful  | Display-only group of removable tags with keyboard navigation               |
| `GridList`    | stateful  | Keyboard-navigable grid of items with 2D arrow key navigation and selection |
| `Marquee`     | stateful  | Scrolling content with play/pause and loop control                          |
| `Skeleton`    | stateless | Animated loading placeholder with pulse/wave/shimmer variants               |

All stateful components follow the same structural rules as the rest of ars-ui:

- **State machine**: `State`, `Event`, `Context`, `Props`, `Xxx` (machine struct), `Api`
- **`Bindable<T>`**: Controls the controlled/uncontrolled duality for every value.
- **`ConnectApi`**: `Api` implements `ConnectApi` with a `Part` enum and `part_attrs()`
  dispatch that returns `AttrMap` for every anatomy part.
- **Anatomy parts**: Every element carries `data-ars-scope="xxx"` and `data-ars-part="yyy"`.
- **Data attributes**: `data-ars-state`, `data-ars-disabled`, `data-ars-selected`,
  `data-ars-sorted`, `data-ars-sort-direction`, `data-ars-highlighted`, `data-ars-expanded`.

Stateless components (`Badge`, `Skeleton`, `Stat`, `Meter`) omit the state machine and expose
only `Props` and a `ConnectApi` implementation. `Table` is the sole complex-tier component,
with variant sections for SelectAll and Column Resizing.

---

---

## 2. Table of Contents

- [Table](table.md)
- [Avatar](avatar.md)
- [Progress](progress.md)
- [Meter](meter.md)
- [RatingGroup](rating-group.md)
- [Badge](badge.md)
- [Stat](stat.md)
- [TagGroup](tag-group.md)
- [GridList](grid-list.md)
- [Marquee](marquee.md)
- [Skeleton](skeleton.md)

---

## 3. Empty State Handling

Components that display collections (Table, GridList, TagGroup) support an optional **EmptyState** anatomy part rendered when the collection has zero items.

- **EmptyState** part: `data-ars-scope="{component}" data-ars-part="empty-state"`
- **Announcement**: The empty state container uses `aria-live="polite"` so screen readers announce when a previously populated list becomes empty (e.g., after filtering).
- **Popup components** (Select, Combobox, Menu): When items are empty, either show the EmptyState inside the popup content, or do not open the popup at all. The machine can guard the `Open` transition on `!ctx.items.is_empty()`.

---

## 4. Cross-Component Notes

### 4.1 Shared Patterns

All stateful components in this file follow these invariants:

1. **Bindable duality**: Every externally observable value uses `Bindable<T>`. Controlled
   mode: `Bindable::controlled(value)`. Uncontrolled mode: `Bindable::uncontrolled(default)`.

2. **AttrMap composition**: `connect()` returns `Api` whose `*_attrs()` methods emit
   `AttrMap`. Adapters (ars-leptos, ars-dioxus) spread these onto the rendered elements.
   Consumers can merge additional props using `AttrMap::merge()`.

3. **Effect cleanup**: Every `Effect` returns a cleanup `Box<dyn FnOnce()>`. The adapter
   calls the cleanup when the component unmounts or the effect re-runs.

4. **Data attribute conventions**:
   - `data-ars-state="<state-name>"` on the root or primary element.
   - `data-ars-disabled` (presence-based) when disabled — set to `""`, omit when not disabled.
   - `data-ars-selected` (presence-based) when selected.
   - `data-ars-expanded` (presence-based) on expandable parts.
   - `data-ars-sorted` (presence-based) on the active sort column header.
   - `data-ars-sort="ascending|descending|none"` encodes sort direction.
   - `data-ars-highlighted` (presence-based) on hovered/previewed items (RatingGroup).
   - `data-ars-loading` (presence-based) when loading (Stat, Progress).

5. **Anatomy scoping**: Every rendered element has both `data-ars-scope` (component name)
   and `data-ars-part` (part name). This allows global CSS selectors like:
   `[data-ars-scope="table"][data-ars-part="column-header"][data-ars-sorted]`.

### 4.2 Relationship Between Progress and Meter

| Dimension          | Progress                        | Meter                             |
| ------------------ | ------------------------------- | --------------------------------- |
| Semantic intent    | Task completion (how far done?) | Measurement (how much right now?) |
| Indeterminate mode | Yes (`value = None`)            | No (always a known value)         |
| Semantic zones     | None                            | Low / high / optimum              |
| HTML element       | `<div role="progressbar">`      | `<meter>` preferred               |
| State machine      | Active (Idle/Loading/Complete)  | Minimal / static                  |
| Primary ARIA       | `role="progressbar"`            | `role="meter"`                    |

### 4.3 no_std Compatibility

`Table`, `Avatar`, `Progress`, `Meter`, `RatingGroup`, `Stat`, `TagGroup`, `GridList` all live in `ars-core` which is
`no_std` compatible with `alloc`. Avoid:

- `std::time` (use `u32` milliseconds or platform-supplied timers via Effects).
- `std::collections::HashMap` (use `alloc::vec::Vec` for small sets; or gate on `std`
  feature).
- Direct DOM access (go through `AttrMap` and the adapter layer).

### 4.4 Testing

Each machine can be unit-tested without a DOM:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_sort_cycles_through_directions() {
        let props = Props::default();
        let (state, mut ctx) = Table::init(&props);
        assert_eq!(state, State::Idle);

        // First click: None → Ascending (context-only transition)
        let t = Table::transition(
            &state, &Event::SortColumn { column: "name".into() }, &ctx, &props
        ).expect("SortColumn transition should exist");
        if let Some(mut apply) = t.apply { apply(&mut ctx); }
        let desc = ctx.sort_descriptor.get().expect("sort_descriptor should be set");
        assert_eq!(desc.direction, SortDirection::Ascending);

        // Second click: Ascending → Descending
        let t = Table::transition(
            &state, &Event::SortColumn { column: "name".into() }, &ctx, &props
        ).expect("SortColumn transition should exist");
        if let Some(mut apply) = t.apply { apply(&mut ctx); }
        let desc = ctx.sort_descriptor.get().expect("sort_descriptor should be set");
        assert_eq!(desc.direction, SortDirection::Descending);

        // Third click: Descending → None (clears sort)
        let t = Table::transition(
            &state, &Event::SortColumn { column: "name".into() }, &ctx, &props
        ).expect("SortColumn transition should exist");
        if let Some(mut apply) = t.apply { apply(&mut ctx); }
        assert!(ctx.sort_descriptor.get().is_none());
    }

    #[test]
    fn rating_group_increment_clamps_at_max() {
        let props = Props { count: 5, ..Default::default() };
        let (mut state, mut ctx) = RatingGroup::init(&props);

        // Set value to 5 (max)
        let t = RatingGroup::transition(&state, &Event::Rate(5.0), &ctx, &props)
            .expect("Rate transition should exist");
        if let Some(mut apply) = t.apply { apply(&mut ctx); }

        // Increment should not exceed max
        let t = RatingGroup::transition(&state, &Event::IncrementRating, &ctx, &props)
            .expect("IncrementRating transition should exist");
        if let Some(mut apply) = t.apply { apply(&mut ctx); }
        assert_eq!(*ctx.value.get(), 5.0);
    }

    #[test]
    fn avatar_shows_fallback_on_error() {
        let props = Props { src: Some("https://example.com/img.png".into()), ..Default::default() };
        let (mut state, mut ctx) = Avatar::init(&props);
        assert_eq!(state, State::Loading);

        let t = Avatar::transition(&state, &Event::ImageError, &ctx, &props)
            .expect("ImageError transition should exist from Loading");
        state = t.target.expect("ImageError should produce Error state");
        assert_eq!(state, State::Error);
        assert!(ctx.fallback_visible);
    }

    #[test]
    fn progress_compute_percent_clamps() {
        assert_eq!(Context::compute_percent(Some(150.0), 0.0, 100.0), 100.0);
        assert_eq!(Context::compute_percent(Some(-10.0), 0.0, 100.0), 0.0);
        assert_eq!(Context::compute_percent(Some(50.0),  0.0, 100.0), 50.0);
        assert_eq!(Context::compute_percent(None,        0.0, 100.0), 0.0);
    }

    #[test]
    fn meter_segment_derivation() {
        // optimum in low zone
        let seg = compute_segment(5.0, 0.0, 100.0, Some(30.0), Some(70.0), Some(10.0));
        assert_eq!(seg, Segment::Optimal); // value < low, opt < low → Optimal

        let seg = compute_segment(50.0, 0.0, 100.0, Some(30.0), Some(70.0), Some(10.0));
        assert_eq!(seg, Segment::SubOptimal); // value in mid, opt in low

        let seg = compute_segment(80.0, 0.0, 100.0, Some(30.0), Some(70.0), Some(10.0));
        assert_eq!(seg, Segment::SubSubOptimal); // value > high, opt in low
    }
}
```
