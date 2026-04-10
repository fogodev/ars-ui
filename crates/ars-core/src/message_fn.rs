//! Shared closure wrapper for translatable component message strings.
//!
//! [`MessageFn`] wraps message closures in [`ArsRc`] (platform-conditional
//! `Rc`/`Arc`), eliminating cfg-gated code in this module. It is distinct from
//! [`Callback`](crate::Callback) (used for event handler closures) and
//! [`CleanupFn`](crate::CleanupFn) (used for effect cleanup).

extern crate alloc;

use alloc::string::String;
use core::{fmt, ops::Deref};

use ars_i18n::Locale;

use crate::ArsRc;

/// Shared function pointer for component message closure fields.
///
/// Wraps closures in [`ArsRc`] — `Rc` on WASM (no atomic overhead), `Arc` on
/// native (thread-safe). All `MessageFn` trait objects include `+ Send + Sync`
/// on all targets so the public API is identical everywhere — closures must be
/// thread-safe for native desktop runtimes (Dioxus Desktop, Tauri). The `Rc`
/// wrapper satisfies WASM's single-threaded `Send`/`Sync` auto-impl.
///
/// `MessageFn` implements [`Debug`] by printing `"<closure>"` so all
/// `Messages` structs can `#[derive(Clone, Debug)]` uniformly.
pub struct MessageFn<T: ?Sized>(ArsRc<T>);

impl<T: ?Sized> Clone for MessageFn<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> fmt::Debug for MessageFn<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<closure>")
    }
}

impl<T: ?Sized> PartialEq for MessageFn<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Deref for MessageFn<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> AsRef<T> for MessageFn<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T: ?Sized> MessageFn<T> {
    /// Creates a `MessageFn` by converting a closure via its [`From`] impl.
    ///
    /// This is the standard constructor for parameterized message defaults:
    ///
    /// ```ignore
    /// count_label: MessageFn::new(|n: usize, _locale: &Locale| {
    ///     format!("{n} items selected")
    /// }),
    /// ```
    pub fn new(f: impl Into<Self>) -> Self {
        f.into()
    }
}

/// `From` impl for `MessageFn<dyn Fn(&Locale) -> String + Send + Sync>`.
#[cfg(target_arch = "wasm32")]
impl<F: Fn(&Locale) -> String + Send + Sync + 'static> From<F>
    for MessageFn<dyn Fn(&Locale) -> String + Send + Sync>
{
    fn from(f: F) -> Self {
        MessageFn(ArsRc(alloc::rc::Rc::new(f)))
    }
}

/// `From` impl for `MessageFn<dyn Fn(&Locale) -> String + Send + Sync>`.
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(&Locale) -> String + Send + Sync + 'static> From<F>
    for MessageFn<dyn Fn(&Locale) -> String + Send + Sync>
{
    fn from(f: F) -> Self {
        MessageFn(ArsRc(alloc::sync::Arc::new(f)))
    }
}

impl MessageFn<dyn Fn(&Locale) -> String + Send + Sync> {
    /// Creates a `MessageFn` from a static string, ignoring the locale parameter.
    ///
    /// Use this for English baselines in `Default` impls where the message does
    /// not vary by locale.
    pub fn static_str(s: &'static str) -> Self {
        Self::new(move |_locale: &Locale| String::from(s))
    }
}

/// Marker trait for component message structs.
///
/// Every component that provides translatable strings defines a `Messages`
/// struct that implements this trait. The trait requires [`Clone`] (for
/// sharing across reactive scopes) and [`Default`] (for English fallbacks).
pub trait ComponentMessages: Clone + Default {}

/// Blanket impl for components with no translatable strings.
///
/// Use `type Messages = ();` in [`Machine`](crate::Machine) implementations
/// that have no user-facing i18n messages.
impl ComponentMessages for () {}

#[cfg(test)]
mod tests {
    use alloc::format;

    use ars_i18n::locales;

    use super::*;

    #[test]
    fn message_fn_clone_shares_pointer_identity() {
        let mf = MessageFn::static_str("Hello");
        let cloned = mf.clone();
        assert_eq!(mf, cloned);
    }

    #[test]
    fn message_fn_different_allocations_are_not_equal() {
        let mf1 = MessageFn::static_str("Hello");
        let mf2 = MessageFn::static_str("Hello");
        assert_ne!(mf1, mf2);
    }

    #[test]
    fn message_fn_debug_output() {
        let mf = MessageFn::static_str("Dismiss");
        assert_eq!(format!("{mf:?}"), "<closure>");
    }

    #[test]
    fn message_fn_deref_invokes_closure() {
        let mf = MessageFn::static_str("Dismiss");
        let locale = locales::en_us();
        assert_eq!(mf(&locale), "Dismiss");
    }

    #[test]
    fn message_fn_as_ref_delegates_to_inner() {
        let mf = MessageFn::static_str("Test");
        let f: &(dyn Fn(&Locale) -> String + Send + Sync) = mf.as_ref();
        assert_eq!(f(&locales::en()), "Test");
    }

    #[test]
    fn message_fn_from_closure() {
        let mf = MessageFn::from(|locale: &Locale| format!("Close ({})", locale.to_bcp47()));
        let locale = locales::de_de();
        assert_eq!(mf(&locale), "Close (de-DE)");
    }

    #[test]
    fn message_fn_new_delegates_to_from() {
        let mf = MessageFn::new(|locale: &Locale| format!("Hello {}", locale.to_bcp47()));
        assert_eq!(mf(&locales::fr()), "Hello fr-FR");
    }

    #[test]
    fn message_fn_static_str_ignores_locale() {
        let mf = MessageFn::static_str("Dismiss");
        assert_eq!(mf(&locales::ja_jp()), "Dismiss");
        assert_eq!(mf(&locales::ar_eg()), "Dismiss");
    }
}
