//! Dioxus-specific test harness backend for ars-ui component testing.
//!
//! Implements [`HarnessBackend`](ars_test_harness::HarnessBackend) for flushing
//! Dioxus reactive updates during tests.

use ars_test_harness::HarnessBackend;

/// Test harness backend that drives Dioxus rendering during component tests.
#[derive(Debug, Default)]
pub struct DioxusHarnessBackend;

impl HarnessBackend for DioxusHarnessBackend {
    fn flush(&mut self) {}
}

#[cfg(test)]
mod tests {
    use ars_test_harness::HarnessBackend;

    use super::DioxusHarnessBackend;

    #[test]
    fn flush_is_a_no_op() {
        let mut backend = DioxusHarnessBackend;
        backend.flush();
    }
}
