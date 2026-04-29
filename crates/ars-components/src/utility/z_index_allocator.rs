//! Context contract for z-index allocation providers.
//!
//! `ZIndexAllocator` is a logical utility: adapters create a provider-scoped
//! context so descendant overlays can claim monotonically increasing stacking
//! layers without sharing a process-global allocator. The agnostic component
//! owns no DOM element and therefore exposes no `Part`, `Api`, `ConnectApi`, or
//! `AttrMap` output.

use ars_core::SharedState;
pub use ars_core::{
    Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index, reset_z_index,
};

/// Framework-agnostic z-index provider context.
///
/// Adapters publish this type through framework context. Descendant overlays
/// claim z-index values from the contained allocator and release those claims
/// during teardown.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// Allocator used by descendant overlay components.
    allocator: SharedState<ZIndexAllocator>,
}

impl Context {
    /// Create an empty z-index provider context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            allocator: SharedState::new(ZIndexAllocator::new()),
        }
    }

    /// Allocate and track the next z-index value.
    #[must_use]
    pub fn allocate(&self) -> u32 {
        self.allocator.with(ZIndexAllocator::allocate)
    }

    /// Allocate and track the next z-index as an identity-safe claim handle.
    #[must_use]
    pub fn allocate_claim(&self) -> ZIndexClaim {
        self.allocator.with(ZIndexAllocator::allocate_claim)
    }

    /// Release a previously allocated z-index value.
    pub fn release(&self, z: u32) {
        self.allocator.with(|allocator| allocator.release(z));
    }

    /// Release a previously allocated z-index claim.
    pub fn release_claim(&self, claim: ZIndexClaim) {
        self.allocator
            .with(|allocator| allocator.release_claim(claim));
    }

    /// Clear tracked allocations and reset the provider-scoped z-index counter.
    pub fn reset(&self) {
        self.allocator.with(ZIndexAllocator::reset);
    }
}

/// Props for the `ZIndexAllocator` provider boundary.
///
/// The provider has no configurable agnostic options today. Adapter components
/// use this zero-sized type to preserve the utility component shape while
/// publishing [`Context`] through framework context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Props;

impl Props {
    /// Create default `ZIndexAllocator` provider props.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_default_is_empty() {
        let props: Props = default_value();

        assert_eq!(props, Props);
    }

    #[test]
    fn props_new_matches_default() {
        assert_eq!(Props::new(), Props);
    }

    #[test]
    fn module_reexports_allocator_context_type() {
        reset_z_index(Z_INDEX_BASE);

        let allocator = ZIndexAllocator::new();

        assert_eq!(allocator.allocate(), Z_INDEX_BASE);
    }

    #[test]
    fn context_allocates_from_provider_scope() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();

        assert_eq!(context.allocate(), Z_INDEX_BASE);
        assert_eq!(next_z_index(), Z_INDEX_BASE);
    }

    #[test]
    fn context_release_claim_does_not_reuse_values() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();

        let claim = context.allocate_claim();

        context.release_claim(claim);

        assert_eq!(context.allocate(), Z_INDEX_BASE + 1);
    }

    #[test]
    fn context_release_value_delegates_to_shared_allocator() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();
        let cloned = context.clone();

        let z = context.allocate();
        let _other = context.allocate();

        cloned.release(z);

        assert_eq!(context.allocate(), Z_INDEX_BASE + 2);
    }

    #[test]
    fn context_reset_clears_tracked_and_resets_provider_scope() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();
        let cloned = context.clone();

        let stale = context.allocate_claim();
        let _other = context.allocate_claim();

        cloned.reset();
        context.release_claim(stale);

        assert_eq!(context.allocate(), Z_INDEX_BASE);
    }

    #[test]
    fn context_clone_shares_allocator_scope() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();
        let cloned = context.clone();

        assert_eq!(context.allocate(), Z_INDEX_BASE);
        assert_eq!(cloned.allocate(), Z_INDEX_BASE + 1);
    }

    #[test]
    fn context_clone_release_claim_shares_allocator_scope() {
        reset_z_index(Z_INDEX_BASE);

        let context = Context::new();
        let cloned = context.clone();

        let claim = context.allocate_claim();
        let _other = context.allocate_claim();

        cloned.release_claim(claim);

        assert_eq!(context.allocate(), Z_INDEX_BASE + 2);
    }

    #[test]
    fn allocator_integration_is_independent_from_global_counter() {
        reset_z_index(Z_INDEX_BASE);

        let allocator = ZIndexAllocator::new();

        assert_eq!(next_z_index(), Z_INDEX_BASE);
        assert_eq!(allocator.allocate(), Z_INDEX_BASE);
        assert_eq!(next_z_index(), Z_INDEX_BASE + 1);
    }

    #[test]
    fn allocator_release_does_not_reuse_values() {
        reset_z_index(Z_INDEX_BASE);

        let allocator = ZIndexAllocator::new();
        let z = allocator.allocate();

        allocator.release(z);

        assert_eq!(allocator.allocate(), Z_INDEX_BASE + 1);
    }

    fn default_value<T: Default>() -> T {
        T::default()
    }
}
