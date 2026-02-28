# Z-Index Stacking Context Management

Shared z-index allocation strategy for overlay components.

All overlay components render into `ars-portal-root` and MUST use a coordinated z-index allocation strategy to prevent stacking context collisions. The canonical allocator is the `next_z_index()` free function from the `ars-dom` crate (see `11-dom-utilities.md` §6 Z-Index Management for the full specification), which returns monotonically increasing values starting at 1000. Separately, the `ZIndexAllocator` component (see `components/utility/z-index-allocator.md`) is a framework-level wrapper that injects the allocator into the adapter context, making `next_z_index()` available to adapter hooks via context propagation:

```rust
/// Canonical z-index allocator: returns the next sequential z-index.
/// Each call returns a value one higher than the previous (1000, 1001, 1002, ...).
/// This ensures overlays opened later always stack above earlier ones.
/// See `11-dom-utilities.md` §6 Z-Index Management for the full specification.
/// Implementation uses `thread_local! { static NEXT_Z_INDEX: Cell<u32> = Cell::new(1000); }`
/// Free function — no `&self`.
/// Overflow protection: values are clamped at `Z_INDEX_CEILING` (see `11-dom-utilities.md` §6.2).
pub fn next_z_index() -> u32 { /* monotonic counter starting at 1000 */ }
```

**Stacking context warning**: If an overlay's content element has CSS properties that create a new stacking context (`opacity < 1`, `transform`, `filter`, `will-change`), nested overlays may be trapped in the parent's stacking context regardless of z-index. The adapter SHOULD emit a console warning at development time when these properties are detected on overlay content elements.

**Backdrop sibling pattern**: The backdrop element MUST be a sibling of the content element inside the portal root, NOT a parent wrapper. This ensures backdrop and content participate in the same stacking context and z-index values are applied correctly:

```text
ars-portal-root
├── DialogBackdrop   (z-index: next_z_index() → 1000)   ← sibling, not parent
├── DialogContent    (z-index: next_z_index() → 1001)
├── NestedBackdrop   (z-index: next_z_index() → 1002)
└── NestedContent    (z-index: next_z_index() → 1003)
```

## 1. Overlay Positioning Considerations

> **Note:** These sections relate to overlay positioning and complement the positioning engine defined in `11-dom-utilities.md` §2. They are co-located here because they interact with z-index layer management during position updates.

### 1.1 ResizeObserver Throttling for Positioning Updates

Overlay components that use floating positioning (Popover, Tooltip, HoverCard) rely on `ResizeObserver` for auto-update. Unthrottled `ResizeObserver` callbacks can cause layout thrashing when positioning changes trigger further resize events. The `ars-dom` positioning engine MUST apply the following mitigations:

1. **Debounce via `requestAnimationFrame`**: All `ResizeObserver` callbacks are batched into a single `requestAnimationFrame` callback. Multiple observers firing in the same frame coalesce into one positioning update.
2. **Minimum update interval**: Enforce a 16ms minimum between positioning updates. If a `ResizeObserver` fires while an update is already in-flight, the next update is scheduled asynchronously after the current one completes.
3. **Cache `getBoundingClientRect()`**: Within a single positioning update cycle, cache all `getBoundingClientRect()` results keyed by element reference. The cache is invalidated at the start of each new rAF cycle.

```rust
// Conceptual adapter-level implementation (pseudocode, not compilable Rust).
// Uses Rc<RefCell<>> to avoid &mut self double-borrow in rAF closures.
// Note: rect_cache uses pointer-based hashing (Rc::as_ptr()) since
// ElementRef (Rc<dyn Any>) does not implement Hash/Eq directly.
struct PositioningScheduler {
    inner: Rc<RefCell<SchedulerState>>,
}

struct SchedulerState {
    pending: bool,
    last_update_time: f64,        // web_sys::window().performance().now() timestamp
    rect_cache: HashMap<usize, Rect>, // Rect from 11-dom-utilities.md §2.2; keyed by Rc::as_ptr() as usize
}

impl PositioningScheduler {
    fn schedule_update(&self) {
        let mut state = self.inner.borrow_mut();
        if state.pending { return; } // already scheduled
        state.pending = true;
        let inner = Rc::clone(&self.inner);
        request_animation_frame(move || {
            let mut state = inner.borrow_mut();
            let now = performance_now();
            if now - state.last_update_time < 16.0 {
                // Throttled — schedule a deferred update instead of dropping it.
                state.pending = false;
                drop(state);
                let inner2 = Rc::clone(&inner);
                // Termination guarantee: after 16ms, `now - last_update_time >= 16.0`
                // will be true, so the deferred update executes rather than deferring again.
                set_timeout(move || {
                    PositioningScheduler { inner: inner2 }.schedule_update();
                }, 16);
                return;
            }
            state.rect_cache.clear();
            state.run_all_pending_updates(); // see below
            state.last_update_time = now;
            state.pending = false;
        });
    }
}

impl SchedulerState {
    /// Execute all pending positioning updates for registered overlay elements.
    /// Each adapter must implement this to iterate over its tracked overlay anchors
    /// and recompute their positions using the cached rects.
    fn run_all_pending_updates(&mut self) {
        // Adapter-provided: iterate tracked overlays, recompute positions
        // using self.rect_cache for getBoundingClientRect() results.
    }
}
```

### 1.2 Anchor Element `content-visibility` Warning

Anchor elements MUST NOT be inside `content-visibility: auto` containers, as `getBoundingClientRect()` returns zero-size rects for off-screen elements whose rendering is skipped by the browser. This causes overlays (Popover, Tooltip, HoverCard) to position at `(0, 0)` or collapse to zero size.

**Mitigation**: Alternatively, set `content-visibility: visible` on the container when an overlay is open. Adapters SHOULD detect zero-size anchor rects at development time and emit a console warning suggesting the `content-visibility` property as the likely cause.
