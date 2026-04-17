//! [`default_provider`] factory: returns the preferred
//! [`IcuProvider`](crate::IcuProvider) for the current feature-flag
//! configuration.

use alloc::boxed::Box;

use crate::IcuProvider;

/// Returns the default [`IcuProvider`] for the current feature flags.
///
/// Precedence matches spec §9.5.3:
///
/// 1. The `icu4x` feature returns an [`Icu4xProvider`](super::Icu4xProvider)
///    with full CLDR data.
/// 2. On `wasm32` targets with the `web-intl` feature (and without `icu4x`),
///    returns a [`WebIntlProvider`](super::WebIntlProvider) that delegates
///    to the browser.
/// 3. Otherwise returns the [`StubIcuProvider`](super::StubIcuProvider).
#[must_use]
pub fn default_provider() -> Box<dyn IcuProvider> {
    #[cfg(feature = "icu4x")]
    {
        Box::new(super::Icu4xProvider)
    }

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    {
        Box::new(super::WebIntlProvider)
    }

    #[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
    {
        Box::new(super::StubIcuProvider)
    }
}
