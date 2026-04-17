//! [`StubIcuProvider`] — English-only fallback provider.
//!
//! `StubIcuProvider` is the default implementation available on every
//! target. It inherits the rollout-era default trait method bodies on
//! [`IcuProvider`](crate::IcuProvider), which match spec §9.5.1 verbatim:
//! English weekday and month names, `AM`/`PM` day-period labels, locale-
//! insensitive zero-padded digit formatting, and CLDR-backed calendar
//! math routed through the shared helpers in `crate::calendar`.
//!
//! [Issue #546](https://github.com/fogodev/ars-ui/issues/546) will
//! remove those rollout defaults and require every provider to implement
//! the trait explicitly. When that lands, the `impl` block below grows
//! spec §9.5.1 bodies; today it can stay empty.

use crate::IcuProvider;

/// English-only stub provider for tests and builds without a backend feature.
///
/// Every method follows the spec §9.5.1 English/default behavior through
/// the rollout default method bodies defined on
/// [`IcuProvider`](crate::IcuProvider). This keeps the stub tiny while
/// downstream tasks plumb the real backends in.
#[derive(Debug, Default)]
pub struct StubIcuProvider;

impl IcuProvider for StubIcuProvider {}
