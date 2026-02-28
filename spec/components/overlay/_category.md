# Overlay Components

Overlay components share a common pattern: a trigger element, a positioner (handles positioning logic), and content rendered above other content. They all use the `ars-dom` positioning engine.

---

## Table of Contents

- [Dialog (Modal)](dialog.md)
- [AlertDialog](alert-dialog.md)
- [Drawer](drawer.md)
- [Popover](popover.md)
- [Tooltip](tooltip.md)
- [HoverCard](hover-card.md)
- [Toast](toast.md)
- [Presence (Mount/Unmount Animation)](presence.md)
- [Tour](tour.md)
- [FloatingPanel (Draggable/Resizable Window)](floating-panel.md)

---

## Z-Index Stacking Context Management

All overlay components render into `ars-portal-root` and MUST use a coordinated z-index allocation strategy to prevent stacking context collisions. The `ars-dom` crate provides `next_z_index()` for this purpose (see [`shared/z-index-stacking.md`](../../shared/z-index-stacking.md)):

```rust
/// Canonical z-index allocator: returns the next sequential z-index.
/// Each call returns a value one higher than the previous (1000, 1001, 1002, ...).
/// This ensures overlays opened later always stack above earlier ones.
/// Implementation uses `thread_local! { static NEXT_Z_INDEX: Cell<u32> = Cell::new(1000); }`
/// Free function — no `&self`.
pub fn next_z_index() -> u32 { /* monotonic counter starting at 1000 */ }
```

**Stacking context warning**: If an overlay's content element has CSS properties that create a new stacking context (`opacity < 1`, `transform`, `filter`, `will-change`), nested overlays may be trapped in the parent's stacking context regardless of z-index. The adapter SHOULD emit a console warning at development time when these properties are detected on overlay content elements.

**Backdrop sibling pattern**: The backdrop element MUST be a sibling of the content element inside the portal root, NOT a parent wrapper. This ensures backdrop and content participate in the same stacking context and z-index values are applied correctly:

```text
ars-portal-root
├── dialog::Backdrop   (z-index: next_z_index() → 1000)   ← sibling, not parent
├── dialog::Content    (z-index: next_z_index() → 1001)
├── NestedBackdrop     (z-index: next_z_index() → 1002)
└── NestedContent      (z-index: next_z_index() → 1003)
```

### ResizeObserver Throttling for Positioning Updates

Overlay components that use floating positioning (`Popover`, `Tooltip`, `HoverCard`) rely on `ResizeObserver` for auto-update. Unthrottled `ResizeObserver` callbacks can cause layout thrashing when positioning changes trigger further resize events. The `ars-dom` positioning engine MUST apply the following mitigations:

1. **Debounce via `requestAnimationFrame`**: All `ResizeObserver` callbacks are batched into a single `requestAnimationFrame` callback. Multiple observers firing in the same frame coalesce into one positioning update.
2. **Minimum update interval**: Enforce a 16ms minimum between positioning updates. If a `ResizeObserver` fires while an update is already in-flight, the next update is scheduled asynchronously after the current one completes.
3. **Cache `getBoundingClientRect()`**: Within a single positioning update cycle, cache all `getBoundingClientRect()` results keyed by element reference. The cache is invalidated at the start of each new rAF cycle.

```rust
// Conceptual adapter-level implementation:
struct PositioningScheduler {
    /// Whether an update is pending.
    pending: bool,
    /// The last update time (performance.now() timestamp)
    last_update_time: f64,
    /// The cache of `getBoundingClientRect()` results keyed by element reference
    rect_cache: HashMap<ElementRef, DomRect>,
}

impl PositioningScheduler {
    /// Schedule an update.
    fn schedule_update(&mut self) {
        if self.pending { return; } // already scheduled
        self.pending = true;
        request_animation_frame(move || {
            let now = performance_now();
            if now - self.last_update_time < 16.0 { return; }
            self.rect_cache.clear();
            self.run_all_pending_updates();
            self.last_update_time = now;
            self.pending = false;
        });
    }
}
```

### Anchor Element `content-visibility` Warning

Anchor elements MUST NOT be inside `content-visibility: auto` containers, as `getBoundingClientRect()` returns zero-size rects for off-screen elements whose rendering is skipped by the browser. This causes overlays (Popover, Tooltip, HoverCard) to position at `(0, 0)` or collapse to zero size.

**Mitigation**: Alternatively, set `content-visibility: visible` on the container when an overlay is open. Adapters SHOULD detect zero-size anchor rects at development time and emit a console warning suggesting the `content-visibility` property as the likely cause.

---

## CSS Custom Properties for Positioning

The `ars-dom` positioning engine sets CSS custom properties on overlay content elements (Positioner parts) after each positioning update. These enable consumer CSS to adapt styling based on computed layout without JavaScript:

| Variable                 | Type                 | Description                                                            |
| ------------------------ | -------------------- | ---------------------------------------------------------------------- |
| `--ars-reference-width`  | `<length>`           | Width of the trigger/anchor element                                    |
| `--ars-reference-height` | `<length>`           | Height of the trigger/anchor element                                   |
| `--ars-available-width`  | `<length>`           | Available viewport space in the main axis direction                    |
| `--ars-available-height` | `<length>`           | Available viewport space in the cross axis direction                   |
| `--ars-x`                | `<length>`           | Computed X position of the overlay                                     |
| `--ars-y`                | `<length>`           | Computed Y position of the overlay                                     |
| `--ars-z-index`          | `<integer>`          | Allocated z-index from `next_z_index()`                                |
| `--ars-transform-origin` | `<transform-origin>` | Computed transform origin for animations (based on resolved placement) |

**Arrow-specific variables** (set on the Arrow part when present):

| Variable                 | Type       | Description                                              |
| ------------------------ | ---------- | -------------------------------------------------------- |
| `--ars-arrow-size`       | `<length>` | Arrow dimensions (width and height)                      |
| `--ars-arrow-background` | `<color>`  | Arrow fill color (mirrors content background by default) |
| `--ars-arrow-x`          | `<length>` | Arrow X offset along the content edge                    |
| `--ars-arrow-y`          | `<length>` | Arrow Y offset along the content edge                    |

**Applies to:** Popover, Tooltip, HoverCard, Tour (step content), FloatingPanel (position/size only: `--ars-x`, `--ars-y`, `--ars-z-index`).

**Adapter obligation:** After each positioning update, the adapter MUST set these properties as inline `style` values on the positioner element. Consumers can reference them in CSS for animations, sizing constraints, and responsive layouts:

```css
[data-ars-scope="popover"][data-ars-part="positioner"] {
  /* Match trigger width */
  min-width: var(--ars-reference-width);
  /* Animate from transform origin based on placement */
  transform-origin: var(--ars-transform-origin);
}
```

---

## Dependencies

All overlay components depend on:

- `ars-core`: Machine, TransitionPlan, PendingEffect, AttrMap
- `ars-dom`: positioning engine, focus utilities, portal, scroll management, inert attribute management
- `ars-a11y`: FocusScope, focus trap for Dialog/AlertDialog
- `ars-interactions`: click-outside detection for Popover, `use_move` for FloatingPanel drag
