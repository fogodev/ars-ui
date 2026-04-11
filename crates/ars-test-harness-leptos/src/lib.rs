//! Leptos-specific test harness backend for ars-ui component testing.
//!
//! Implements [`HarnessBackend`](ars_test_harness::HarnessBackend) for flushing
//! Leptos reactive updates during tests.

use ars_test_harness::HarnessBackend;

/// Test harness backend that drives Leptos rendering during component tests.
#[derive(Debug, Default)]
pub struct LeptosHarnessBackend;

impl HarnessBackend for LeptosHarnessBackend {
    fn flush(&mut self) {}
}

#[cfg(test)]
mod tests {
    use ars_test_harness::HarnessBackend;

    use super::LeptosHarnessBackend;

    #[test]
    fn flush_is_a_no_op() {
        let mut backend = LeptosHarnessBackend;
        backend.flush();
    }
}
