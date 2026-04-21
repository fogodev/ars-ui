//! [`default_backend`] factory: returns the preferred
//! [`IntlBackend`](crate::IntlBackend) for the current feature-flag
//! configuration.

use alloc::boxed::Box;

use crate::IntlBackend;

/// Returns the default [`IntlBackend`] for the current feature flags.
///
/// Precedence matches spec §9.5.3:
///
/// 1. The `icu4x` feature returns an [`Icu4xBackend`](super::Icu4xBackend)
///    with full CLDR data.
/// 2. On `wasm32` targets with the `web-intl` feature (and without `icu4x`),
///    returns a [`WebIntlBackend`](super::WebIntlBackend) that delegates
///    to the browser.
/// 3. Otherwise returns the [`StubIntlBackend`](super::StubIntlBackend).
#[must_use]
pub fn default_backend() -> Box<dyn IntlBackend> {
    #[cfg(feature = "icu4x")]
    {
        Box::new(super::Icu4xBackend)
    }

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    {
        Box::new(super::WebIntlBackend)
    }

    #[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
    {
        Box::new(super::StubIntlBackend)
    }
}
