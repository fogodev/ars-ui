//! Framework-specific backend contract for the shared test harness.

use std::{any::Any, pin::Pin, time::Duration};

use ars_i18n::Locale;

use crate::AnyService;

/// Abstracts framework-specific rendering and reactivity for adapter tests.
pub trait HarnessBackend: 'static {
    /// Mounts a component into the given isolated container.
    fn mount(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>>;

    /// Mounts a component into the given isolated container with an explicit locale wrapper.
    fn mount_with_locale(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
        locale: Locale,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>>;

    /// Flushes pending reactive work so DOM state is safe to inspect.
    fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>>;

    /// Advances any backend-owned fake timer infrastructure.
    fn advance_time(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()>>>;
}
