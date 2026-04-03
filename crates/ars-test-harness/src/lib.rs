//! Framework-agnostic test harness for ars-ui component testing.
//!
//! Provides the shared testing infrastructure used by both framework-specific harness
//! crates. [`TestHarness`] configures the test environment (locale, etc.) while
//! [`HarnessBackend`] is implemented by each adapter to handle framework-specific
//! rendering and flushing.

use ars_i18n::Locale;

/// A framework-specific backend that drives rendering during tests.
///
/// Each adapter crate (e.g. `ars-test-harness-leptos`) implements this trait
/// to flush pending reactive updates and synchronize DOM state for assertions.
pub trait HarnessBackend {
    /// Flushes any pending reactive updates so DOM state is consistent for assertions.
    fn flush(&mut self);
}

/// A handle to a DOM element located by CSS selector, used for test assertions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElementHandle {
    selector: String,
}

impl ElementHandle {
    /// Creates a new element handle targeting the given CSS selector.
    #[must_use]
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    /// Returns the CSS selector this handle targets.
    #[must_use]
    pub fn selector(&self) -> &str {
        &self.selector
    }
}

/// Configuration for a component test environment.
///
/// Sets up locale and other context needed by components under test.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestHarness {
    locale: Option<Locale>,
}

impl TestHarness {
    /// Creates a test harness configured with the given locale.
    #[must_use]
    pub fn with_locale(locale: Locale) -> Self {
        Self {
            locale: Some(locale),
        }
    }

    /// Returns the configured locale, if any.
    #[must_use]
    pub fn locale(&self) -> Option<&Locale> {
        self.locale.as_ref()
    }
}
