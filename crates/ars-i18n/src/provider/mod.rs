//! Concrete [`IntlBackend`](crate::IntlBackend) backends.
//!
//! This module owns the three production-ready provider implementations the
//! spec calls out in §9.5:
//!
//! - [`StubIntlBackend`] — English-only provider for tests and builds without
//!   any backend feature enabled.
//! - [`Icu4xBackend`] — ICU4X-backed provider with CLDR data, used on native
//!   builds with the `icu4x` feature.
//! - [`WebIntlBackend`] — browser-backed provider that delegates to the
//!   `Intl.*` APIs, used on WASM client builds with the `web-intl` feature.
//!
//! The [`default_backend`] factory returns the preferred backend for the
//! current feature-flag configuration.

mod factory;
#[cfg(feature = "icu4x")]
mod icu4x;
mod stub;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
mod web_intl;

#[cfg(all(test, feature = "icu4x"))]
#[path = "../../tests/unit/provider_icu4x.rs"]
mod provider_icu4x_tests;
#[cfg(test)]
#[path = "../../tests/unit/provider_stub.rs"]
mod provider_stub_tests;
#[cfg(all(test, feature = "web-intl", target_arch = "wasm32"))]
#[path = "../../tests/unit/provider_web_intl.rs"]
mod provider_web_intl_tests;

pub use factory::default_backend;
#[cfg(feature = "icu4x")]
pub use icu4x::Icu4xBackend;
pub use stub::StubIntlBackend;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub use web_intl::WebIntlBackend;
