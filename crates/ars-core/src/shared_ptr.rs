//! Platform-conditional shared pointer.
//!
//! [`ArsRc`] wraps `Rc` on wasm and `Arc` on native targets, providing a
//! unified type for shared trait-object references throughout the ars-ui
//! crate family.

extern crate alloc;

use core::fmt::{self, Debug};

/// Platform-conditional shared pointer.
///
/// Uses `Rc` on wasm (single-threaded) and `Arc` on native (multi-threaded)
/// targets, mirroring the [`Callback`](crate::Callback) platform split. This
/// is the standard wrapper for shared trait-object references
/// (`ArsRc<dyn ModalityContext>`, `ArsRc<dyn PlatformEffects>`, etc.)
/// throughout the ars-ui crate family.
///
/// Cloning increments the reference count — it does **not** clone the inner
/// value.
#[cfg(target_arch = "wasm32")]
pub struct ArsRc<T: ?Sized>(pub(crate) alloc::rc::Rc<T>);

/// Platform-conditional shared pointer.
///
/// Uses `Rc` on wasm (single-threaded) and `Arc` on native (multi-threaded)
/// targets, mirroring the [`Callback`](crate::Callback) platform split. This
/// is the standard wrapper for shared trait-object references
/// (`ArsRc<dyn ModalityContext>`, `ArsRc<dyn PlatformEffects>`, etc.)
/// throughout the ars-ui crate family.
///
/// Cloning increments the reference count — it does **not** clone the inner
/// value.
#[cfg(not(target_arch = "wasm32"))]
pub struct ArsRc<T: ?Sized>(pub(crate) alloc::sync::Arc<T>);

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> Clone for ArsRc<T> {
    fn clone(&self) -> Self {
        ArsRc(alloc::rc::Rc::clone(&self.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> Clone for ArsRc<T> {
    fn clone(&self) -> Self {
        ArsRc(alloc::sync::Arc::clone(&self.0))
    }
}

impl<T: ?Sized> Debug for ArsRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ArsRc(..)")
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> PartialEq for ArsRc<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> PartialEq for ArsRc<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> core::ops::Deref for ArsRc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> AsRef<T> for ArsRc<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Constructor for concrete types on wasm targets.
#[cfg(target_arch = "wasm32")]
impl<T: 'static> ArsRc<T> {
    /// Creates a new shared pointer wrapping the given value.
    pub fn new(value: T) -> Self {
        Self(alloc::rc::Rc::new(value))
    }
}

/// Constructor for concrete types on native targets.
///
/// Requires `Send + Sync` so the resulting `ArsRc` can be safely shared
/// across threads.
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync + 'static> ArsRc<T> {
    /// Creates a new shared pointer wrapping the given value.
    pub fn new(value: T) -> Self {
        Self(alloc::sync::Arc::new(value))
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[test]
    fn ars_rc_new_and_deref() {
        let rc = ArsRc::new(42u32);
        assert_eq!(*rc, 42);
    }

    #[test]
    fn ars_rc_as_ref() {
        let rc = ArsRc::new(String::from("hello"));
        let s: &String = rc.as_ref();
        assert_eq!(s, "hello");
    }

    #[test]
    fn ars_rc_clone_shares_pointer() {
        let rc1 = ArsRc::new(99u32);
        let rc2 = rc1.clone();
        assert_eq!(rc1, rc2);
        assert_eq!(*rc2, 99);
    }

    #[test]
    fn ars_rc_partial_eq_by_pointer_identity() {
        let rc1 = ArsRc::new(42u32);
        let rc2 = rc1.clone();
        let rc3 = ArsRc::new(42u32);

        // Same allocation
        assert_eq!(rc1, rc2);
        // Different allocation (same value but different pointer)
        assert_ne!(rc1, rc3);
    }

    #[test]
    fn ars_rc_debug_output() {
        let rc = ArsRc::new(42u32);
        assert_eq!(alloc::format!("{rc:?}"), "ArsRc(..)");
    }

    #[test]
    fn ars_rc_from_modality_creates_trait_object() {
        use crate::{DefaultModalityContext, ModalitySnapshot};

        let rc = ArsRc::from_modality(DefaultModalityContext::new());
        assert_eq!(rc.snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn ars_rc_from_platform_creates_trait_object() {
        use crate::NullPlatformEffects;

        let rc = ArsRc::from_platform(NullPlatformEffects);
        rc.focus_element_by_id("test");
    }

    #[test]
    fn ars_rc_trait_object_clone_preserves_identity() {
        use crate::DefaultModalityContext;

        let rc1 = ArsRc::from_modality(DefaultModalityContext::new());
        let rc2 = ArsRc::clone(&rc1);
        assert_eq!(rc1, rc2);
    }
}
