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

ZIndexAllocator provides provider-scoped z-index allocation services to descendant overlay components via context. Its agnostic `Context` wraps `ars_core::ZIndexAllocator`; the legacy `next_z_index()` free function remains available from `ars-core` and `ars-dom` for compatibility, but provider-backed overlays should allocate through context. See `spec/foundation/11-dom-utilities.md` §6.2 and `spec/shared/z-index-stacking.md`.

## 1. API

`ZIndexAllocator` is a context-only provider — it has no `Part` enum, no `ConnectApi`, and no `AttrMap` output. Adapters create a `z_index_allocator::Context` in a top-level provider and expose it via framework context. Snapshot tests for `connect()` / `Api` `AttrMap` output are not applicable because this utility has no agnostic connect API.

### 1.1 Props

```rust
/// Props for the `ZIndexAllocator` provider boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Props;

impl Props {
    /// Create default `ZIndexAllocator` provider props.
    pub const fn new() -> Self {
        Self
    }
}
```

The provider has no configurable agnostic options today. Framework adapters use this zero-sized type to preserve the utility component shape while publishing [`Context`] through framework context.

### 1.2 Context

```rust
pub use ars_core::{
    SharedState, Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index,
    reset_z_index,
};

/// Framework-agnostic z-index provider context.
#[derive(Clone, Debug, Default)]
pub struct Context {
    allocator: SharedState<ZIndexAllocator>,
}

impl Context {
    /// Create an empty z-index provider context.
    pub fn new() -> Self {
        Self {
            allocator: SharedState::new(ZIndexAllocator::new()),
        }
    }

    /// Allocate and track the next z-index value.
    pub fn allocate(&self) -> u32 {
        self.allocator.with(ZIndexAllocator::allocate)
    }

    /// Allocate and track the next z-index as an identity-safe claim handle.
    pub fn allocate_claim(&self) -> ZIndexClaim {
        self.allocator.with(ZIndexAllocator::allocate_claim)
    }

    /// Release a previously allocated z-index value.
    pub fn release(&self, z: u32) {
        self.allocator.with(|allocator| allocator.release(z));
    }

    /// Release a previously allocated z-index claim.
    pub fn release_claim(&self, claim: ZIndexClaim) {
        self.allocator.with(|allocator| allocator.release_claim(claim));
    }

    /// Clear tracked allocations and reset the provider-scoped z-index counter.
    pub fn reset(&self) {
        self.allocator.with(ZIndexAllocator::reset);
    }
}
```

`Context` is the type adapters publish through framework context. It wraps the canonical `ars_core::ZIndexAllocator` in `SharedState` so cloned context values keep one allocator scope even when framework context APIs clone provider payloads.

### 1.3 ZIndexAllocator Core Contract

```rust
use core::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZIndexClaim {
    value: u32,
    id: u64,
}

impl ZIndexClaim {
    /// Return the CSS z-index value for this claim.
    pub const fn value(self) -> u32 {
        self.value
    }
}

#[derive(Debug)]
pub struct ZIndexAllocator {
    next_z_index: Cell<u32>,
    allocated: RefCell<Vec<ZIndexClaim>>,
    next_claim_id: Cell<u64>,
}

impl ZIndexAllocator {
    /// Create a new allocator with no tracked allocations.
    pub const fn new() -> Self {
        Self {
            next_z_index: Cell::new(Z_INDEX_BASE),
            allocated: RefCell::new(Vec::new()),
            next_claim_id: Cell::new(0),
        }
    }

    /// Allocate the next z-index value.
    pub fn allocate(&self) -> u32 {
        self.allocate_claim().value()
    }

    /// Allocate the next z-index as an identity-safe claim handle.
    pub fn allocate_claim(&self) -> ZIndexClaim {
        let claim = ZIndexClaim {
            value: self.next_z_index(),
            id: self.next_claim_id.get(),
        };

        self.next_claim_id.set(claim.id.wrapping_add(1));
        self.allocated.borrow_mut().push(claim);

        claim
    }

    /// Release a previously allocated z-index.
    pub fn release(&self, z: u32) {
        let mut allocated = self.allocated.borrow_mut();

        if let Some(index) = allocated.iter().position(|claim| claim.value == z) {
            allocated.remove(index);
        }
    }

    /// Release a previously allocated z-index claim.
    pub fn release_claim(&self, claim: ZIndexClaim) {
        let mut allocated = self.allocated.borrow_mut();

        if let Some(index) = allocated.iter().position(|tracked| *tracked == claim) {
            allocated.remove(index);
        }
    }

    /// Reset all tracked allocations.
    pub fn reset(&self) {
        self.allocated.borrow_mut().clear();
        self.next_z_index.set(Z_INDEX_BASE);
    }
}
```

`allocate_claim()` / `release_claim()` are the preferred lifecycle API because they release by allocator-local identity. `allocate()` / `release(u32)` remain available for compatibility and simple integrations; value-based release removes the first tracked claim with that value and ignores unknown values. Released values are never reused. `reset()` clears tracked claims and resets the provider-scoped counter but does not reset allocator-local claim identity, preventing stale handles from matching new claims after reset.

`ars-dom` re-exports `ZIndexAllocator`, `ZIndexClaim`, `next_z_index()`, and `reset_z_index()` for adapter compatibility, but it does not own allocation behavior.

### 1.4 Constants

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

```rust,no_check
// Adapter creates the context in a top-level provider
let context = z_index_allocator::Context::new();

// Overlay components request z-index values
let claim = context.allocate_claim();
let z = claim.value();
// Use z for the overlay's container style

// On overlay unmount
context.release_claim(claim);
```

See `spec/shared/z-index-stacking.md` for the full stacking context model and `spec/foundation/11-dom-utilities.md` §6 for the z-index management architecture.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
