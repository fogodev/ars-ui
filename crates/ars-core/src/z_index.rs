//! Pure z-index allocation for layered component surfaces.
//!
//! The allocator provides DOM-free scoped stacking counters used by overlay
//! adapters. `ars-dom` re-exports this module's public allocation API so
//! existing DOM-facing call sites keep the same paths.

use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
#[cfg(not(feature = "std"))]
use core::sync::atomic::{AtomicU32, Ordering};

/// Starting value for z-index allocation.
///
/// Application content should stay below this value; generated overlay layers
/// start here and move upward monotonically.
pub const Z_INDEX_BASE: u32 = 1000;

/// Maximum z-index value before the counter wraps back to [`Z_INDEX_BASE`].
///
/// This prevents overflow on very long-running applications that allocate many
/// transient overlay layers.
pub const Z_INDEX_CEILING: u32 = u32::MAX - 1000;

#[cfg(feature = "std")]
thread_local! {
    /// Per-thread monotonic z-index counter.
    static NEXT_Z_INDEX: Cell<u32> = const { Cell::new(Z_INDEX_BASE) };
}

#[cfg(not(feature = "std"))]
static NEXT_Z_INDEX: AtomicU32 = AtomicU32::new(Z_INDEX_BASE);

/// Allocate the next z-index from the compatibility global counter.
///
/// Each call returns a monotonically increasing value starting at
/// [`Z_INDEX_BASE`]. Values are never reused, even after a value is released
/// through [`ZIndexAllocator::release`].
///
/// # Overflow protection
///
/// When the counter reaches [`Z_INDEX_CEILING`], it returns [`Z_INDEX_BASE`]
/// for that allocation and stores [`Z_INDEX_BASE`] + 1 for the next allocation.
#[must_use]
pub fn next_z_index() -> u32 {
    next_z_index_impl()
}

#[cfg(feature = "std")]
fn next_z_index_impl() -> u32 {
    NEXT_Z_INDEX.with(|z| {
        let value = z.get();

        if value >= Z_INDEX_CEILING {
            warn_z_index_wrap();

            z.set(Z_INDEX_BASE + 1);

            Z_INDEX_BASE
        } else {
            z.set(value + 1);

            value
        }
    })
}

#[cfg(not(feature = "std"))]
fn next_z_index_impl() -> u32 {
    loop {
        let value = NEXT_Z_INDEX.load(Ordering::Relaxed);

        let (returned, next) = if value >= Z_INDEX_CEILING {
            warn_z_index_wrap();

            (Z_INDEX_BASE, Z_INDEX_BASE + 1)
        } else {
            (value, value + 1)
        };

        if NEXT_Z_INDEX
            .compare_exchange(value, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return returned;
        }
    }
}

#[cfg(feature = "debug")]
fn warn_z_index_wrap() {
    log::warn!(
        "[ars-core] z-index counter reached ceiling ({Z_INDEX_CEILING}), \
         resetting to base ({Z_INDEX_BASE})"
    );
}

#[cfg(not(feature = "debug"))]
const fn warn_z_index_wrap() {}

/// Reset the compatibility global z-index counter to the supplied base value.
///
/// This is intended for deterministic tests and application-level teardown.
/// Provider-scoped overlay flows should allocate and release through
/// [`ZIndexAllocator`] rather than resetting the compatibility global counter.
pub fn reset_z_index(base: u32) {
    reset_z_index_impl(base);
}

#[cfg(feature = "std")]
fn reset_z_index_impl(base: u32) {
    NEXT_Z_INDEX.with(|z| z.set(base));
}

#[cfg(not(feature = "std"))]
fn reset_z_index_impl(base: u32) {
    NEXT_Z_INDEX.store(base, Ordering::Relaxed);
}

/// Handle for an allocated z-index claim.
///
/// The [`value`](Self::value) is the CSS z-index to apply. The private identity
/// is used by [`ZIndexAllocator::release_claim`] so cleanup can target one
/// allocation even if a value is duplicated after a counter reset or wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZIndexClaim {
    /// CSS z-index value assigned to the claim.
    value: u32,

    /// Allocator-local identity for release bookkeeping.
    id: u64,
}

impl ZIndexClaim {
    /// Return the CSS z-index value for this claim.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }
}

/// Structured z-index allocator with explicit lifecycle tracking.
///
/// The allocator owns a scoped z-index counter and records claims that have
/// been allocated. Releasing a claim removes it from the tracked set but does
/// not make it available for reuse; the scoped counter only moves forward.
#[derive(Debug)]
pub struct ZIndexAllocator {
    /// Scoped monotonic z-index counter for this allocator instance.
    next_z_index: Cell<u32>,

    /// Claims currently tracked by this allocator instance.
    allocated: RefCell<Vec<ZIndexClaim>>,

    /// Allocator-local claim id source.
    next_claim_id: Cell<u64>,
}

impl ZIndexAllocator {
    /// Create a new allocator with no tracked allocations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_z_index: Cell::new(Z_INDEX_BASE),
            allocated: RefCell::new(Vec::new()),
            next_claim_id: Cell::new(0),
        }
    }

    /// Allocate and track the next z-index value.
    ///
    /// Values start at [`Z_INDEX_BASE`] and increase monotonically until the
    /// overflow ceiling is reached.
    #[must_use]
    pub fn allocate(&self) -> u32 {
        self.allocate_claim().value()
    }

    /// Allocate and track the next z-index as an identity-safe claim handle.
    ///
    /// Prefer this API for lifecycle-managed overlays. The returned
    /// [`ZIndexClaim`] can be released with [`Self::release_claim`] without
    /// relying on value matching.
    #[must_use]
    pub fn allocate_claim(&self) -> ZIndexClaim {
        let claim = ZIndexClaim {
            value: self.next_z_index(),
            id: self.next_claim_id.get(),
        };

        self.next_claim_id.set(claim.id.wrapping_add(1));

        self.allocated.borrow_mut().push(claim);

        claim
    }

    /// Release a previously allocated z-index value.
    ///
    /// Unknown values and already released values are ignored. This
    /// compatibility API releases the first tracked claim with the supplied
    /// value. Prefer [`Self::release_claim`] when the caller owns a claim
    /// handle.
    pub fn release(&self, z: u32) {
        let mut allocated = self.allocated.borrow_mut();

        if let Some(index) = allocated.iter().position(|claim| claim.value == z) {
            allocated.remove(index);
        }
    }

    /// Release a previously allocated z-index claim.
    ///
    /// Unknown claims and already released claims are ignored. Released values
    /// are not reused by future allocations.
    pub fn release_claim(&self, claim: ZIndexClaim) {
        let mut allocated = self.allocated.borrow_mut();

        if let Some(index) = allocated.iter().position(|tracked| *tracked == claim) {
            allocated.remove(index);
        }
    }

    /// Clear tracked allocations and reset the provider-scoped counter to
    /// [`Z_INDEX_BASE`].
    ///
    /// Allocator-local claim identity is not reset. That prevents stale
    /// [`ZIndexClaim`] handles from matching new claims after a reset.
    pub fn reset(&self) {
        self.allocated.borrow_mut().clear();

        self.next_z_index.set(Z_INDEX_BASE);
    }
}

impl ZIndexAllocator {
    /// Allocate the next z-index from this allocator's scoped counter.
    fn next_z_index(&self) -> u32 {
        let value = self.next_z_index.get();

        if value >= Z_INDEX_CEILING {
            warn_z_index_wrap();

            self.next_z_index.set(Z_INDEX_BASE + 1);

            Z_INDEX_BASE
        } else {
            self.next_z_index.set(value + 1);

            value
        }
    }
}

impl Default for ZIndexAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    static TEST_SERIAL: AtomicBool = AtomicBool::new(false);

    struct SerialGuard;

    impl Drop for SerialGuard {
        fn drop(&mut self) {
            TEST_SERIAL.store(false, Ordering::Release);
        }
    }

    fn serial_reset() -> SerialGuard {
        while TEST_SERIAL
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        reset_z_index(Z_INDEX_BASE);

        SerialGuard
    }

    impl ZIndexAllocator {
        fn tracked_count(&self) -> usize {
            self.allocated.borrow().len()
        }

        fn set_next_z_index(&self, value: u32) {
            self.next_z_index.set(value);
        }
    }

    #[test]
    fn next_z_index_starts_at_base() {
        let _guard = serial_reset();

        assert_eq!(next_z_index(), 1000);
    }

    #[test]
    fn next_z_index_is_monotonically_increasing() {
        let _guard = serial_reset();

        let z1 = next_z_index();
        let z2 = next_z_index();
        let z3 = next_z_index();

        assert_eq!(z1, 1000);
        assert_eq!(z2, 1001);
        assert_eq!(z3, 1002);
        assert!(z3 > z2);
        assert!(z2 > z1);
    }

    #[test]
    fn next_z_index_wraps_at_ceiling() {
        let _guard = serial_reset();

        reset_z_index(Z_INDEX_CEILING);

        assert_eq!(
            next_z_index(),
            Z_INDEX_BASE,
            "ceiling hit should return Z_INDEX_BASE"
        );
    }

    #[test]
    fn next_z_index_resumes_after_wrap() {
        let _guard = serial_reset();

        reset_z_index(Z_INDEX_CEILING);

        let _ = next_z_index();

        assert_eq!(
            next_z_index(),
            Z_INDEX_BASE + 1,
            "post-wrap should resume from Z_INDEX_BASE + 1"
        );
    }

    #[test]
    fn next_z_index_one_below_ceiling_then_wraps() {
        let _guard = serial_reset();

        reset_z_index(Z_INDEX_CEILING - 1);

        assert_eq!(next_z_index(), Z_INDEX_CEILING - 1);
        assert_eq!(next_z_index(), Z_INDEX_BASE);
    }

    #[test]
    fn reset_z_index_changes_next_value() {
        let _guard = serial_reset();

        reset_z_index(5000);

        assert_eq!(next_z_index(), 5000);
        assert_eq!(next_z_index(), 5001);
    }

    #[test]
    fn reset_z_index_to_base_restores_default() {
        let _guard = serial_reset();

        let _ = next_z_index();
        let _ = next_z_index();

        reset_z_index(Z_INDEX_BASE);

        assert_eq!(next_z_index(), 1000);
    }

    #[test]
    fn allocator_allocate_returns_increasing_values() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        assert_eq!(allocator.allocate(), 1000);
        assert_eq!(allocator.allocate(), 1001);
        assert_eq!(allocator.allocate(), 1002);
    }

    #[test]
    fn allocator_allocate_wraps_at_ceiling() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        allocator.set_next_z_index(Z_INDEX_CEILING);

        assert_eq!(allocator.allocate(), Z_INDEX_BASE);
        assert_eq!(allocator.allocate(), Z_INDEX_BASE + 1);
        assert_eq!(allocator.tracked_count(), 2);
    }

    #[test]
    fn allocator_allocate_is_scoped_independent_from_global_counter() {
        let _guard = serial_reset();

        reset_z_index(5000);

        let allocator = ZIndexAllocator::new();

        assert_eq!(next_z_index(), 5000);
        assert_eq!(allocator.allocate(), Z_INDEX_BASE);
        assert_eq!(next_z_index(), 5001);
        assert_eq!(allocator.allocate(), Z_INDEX_BASE + 1);
    }

    #[test]
    fn allocator_release_removes_from_tracked() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let z1 = allocator.allocate();
        let _z2 = allocator.allocate();

        assert_eq!(allocator.tracked_count(), 2);

        allocator.release(z1);

        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_allocate_claim_returns_claim_value() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let claim = allocator.allocate_claim();

        assert_eq!(claim.value(), Z_INDEX_BASE);
        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_claim_removes_exact_claim() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let claim = allocator.allocate_claim();
        let _other = allocator.allocate_claim();

        allocator.release_claim(claim);

        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_claim_does_not_remove_same_value_with_different_identity() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let stale = allocator.allocate_claim();

        allocator.reset();

        let current = allocator.allocate_claim();

        assert_eq!(stale.value(), current.value());

        allocator.release_claim(stale);

        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_value_removes_one_matching_claim() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let first = allocator.allocate_claim();

        allocator.set_next_z_index(Z_INDEX_BASE);

        let second = allocator.allocate_claim();

        assert_eq!(first.value(), second.value());

        allocator.release(first.value());

        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_unknown_is_noop() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        allocator.release(9999);

        assert_eq!(allocator.tracked_count(), 0);
    }

    #[test]
    fn allocator_release_does_not_affect_counter() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let z1 = allocator.allocate();

        allocator.release(z1);

        assert_eq!(allocator.allocate(), 1001);
    }

    #[test]
    fn allocator_double_release_is_noop() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let z1 = allocator.allocate();
        let _z2 = allocator.allocate();

        assert_eq!(allocator.tracked_count(), 2);

        allocator.release(z1);

        assert_eq!(allocator.tracked_count(), 1);

        allocator.release(z1);

        assert_eq!(allocator.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_after_reset_is_noop() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let z1 = allocator.allocate();

        allocator.reset();

        assert_eq!(allocator.tracked_count(), 0);

        allocator.release(z1);

        assert_eq!(allocator.tracked_count(), 0);
    }

    #[test]
    fn allocator_reset_clears_tracked_and_resets_counter() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        let _ = allocator.allocate();
        let _ = allocator.allocate();

        assert_eq!(allocator.tracked_count(), 2);

        allocator.reset();

        assert_eq!(allocator.tracked_count(), 0);
        assert_eq!(allocator.allocate(), 1000);
    }

    #[test]
    fn allocator_default_is_empty() {
        let _guard = serial_reset();

        assert_eq!(ZIndexAllocator::default().tracked_count(), 0);
    }

    #[test]
    fn allocator_debug_format() {
        let _guard = serial_reset();

        let debug = format!("{:?}", ZIndexAllocator::new());

        assert!(
            debug.contains("ZIndexAllocator"),
            "Debug output should contain the type name"
        );
    }
}
