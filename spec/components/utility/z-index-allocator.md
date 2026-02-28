---
component: ZIndexAllocator
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: [z-index-stacking]
related: []
references: {}
---

# ZIndexAllocator

ZIndexAllocator provides z-index allocation services to descendant overlay components via context. It delegates to the canonical `next_z_index()` thread-local counter defined in `spec/foundation/11-dom-utilities.md` §6.2 and `spec/shared/z-index-stacking.md`.

## 1. API

`ZIndexAllocator` is a context-only provider — it has no `Part` enum, no `ConnectApi`, and no `AttrMap` output. Adapters create a `ZIndexAllocator` in a top-level provider and expose it via framework context.

### 1.1 ZIndexAllocator

```rust
pub struct ZIndexAllocator {
    allocated: RefCell<Vec<u32>>,
}

impl ZIndexAllocator {
    /// Create a new allocator. Uses the global thread-local base (1000).
    pub fn new() -> Self {
        Self {
            allocated: RefCell::new(Vec::new()),
        }
    }

    /// Allocate the next z-index value.
    ///
    /// Returns a monotonically increasing `u32` starting at `Z_INDEX_BASE` (1000).
    /// Values are never reused — gaps from released values are expected.
    ///
    /// # Overflow Protection
    /// When the counter reaches `Z_INDEX_CEILING` (`u32::MAX - 1000`), the
    /// allocator resets to `Z_INDEX_BASE` and emits a `cfg(debug_assertions)`
    /// warning. In practice, overflow requires billions of allocations.
    pub fn allocate(&self) -> u32 {
        let z = next_z_index(); // ars_dom thread-local counter
        self.allocated.borrow_mut().push(z);
        z
    }

    /// Release a previously allocated z-index.
    ///
    /// Removes the value from the tracked set. Does NOT make the value
    /// available for reuse — the counter only moves forward.
    pub fn release(&self, z: u32) {
        self.allocated.borrow_mut().retain(|&v| v != z);
    }

    /// Reset all tracked allocations.
    ///
    /// Used for testing. Also resets the global counter via `reset_z_index(Z_INDEX_BASE)`.
    pub fn reset(&self) {
        self.allocated.borrow_mut().clear();
        reset_z_index(Z_INDEX_BASE);
    }
}
```

### 1.2 Constants

| Constant          | Value             | Description                           |
| ----------------- | ----------------- | ------------------------------------- |
| `Z_INDEX_BASE`    | `1000`            | Starting value for z-index allocation |
| `Z_INDEX_CEILING` | `u32::MAX - 1000` | Maximum value before reset            |

## 2. Anatomy

```text
ZIndexAllocator
└── {children}    (no DOM element rendered)
```

ZIndexAllocator is a context-only provider with no rendered DOM elements. It provides z-index allocation services to descendant overlay components (Dialog, Popover, Tooltip, etc.) via framework context.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

ZIndexAllocator has no ARIA semantics. It is a context-only provider invisible to assistive technology.

## 4. Usage

```rust
// Adapter creates the allocator in a top-level provider
let allocator = ZIndexAllocator::new();

// Overlay components request z-index values
let z = allocator.allocate();
// Use z for the overlay's container style

// On overlay unmount
allocator.release(z);
```

See `spec/shared/z-index-stacking.md` for the full stacking context model and `spec/foundation/11-dom-utilities.md` §6 for the z-index management architecture.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
