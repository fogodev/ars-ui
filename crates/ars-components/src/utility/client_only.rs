//! Logical props for client-only rendering boundaries.
//!
//! `ClientOnly` renders no wrapper and has no agnostic connect API. Framework
//! adapters own the actual SSR and hydration gating behavior; this module only
//! defines the shared props shape.

/// Props for the `ClientOnly` logical boundary.
///
/// `Fallback` is the framework-specific view or element type rendered during
/// SSR and the initial hydration pass. When [`fallback`](Self::fallback) is
/// `None`, adapters render no fallback content.
#[derive(Clone, Debug, Default)]
pub struct Props<Fallback = ()> {
    /// Optional fallback content rendered before the client-only children mount.
    pub fallback: Option<Fallback>,
}

impl<Fallback> Props<Fallback> {
    /// Create `ClientOnly` props with no fallback content.
    #[must_use]
    pub const fn new() -> Self {
        Self { fallback: None }
    }

    /// Set the fallback content rendered during SSR and initial hydration.
    #[must_use]
    pub fn fallback(mut self, fallback: Fallback) -> Self {
        self.fallback = Some(fallback);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_default_has_no_fallback() {
        let props = Props::<&str>::default();

        assert_eq!(props.fallback, None);
    }

    #[test]
    fn props_new_has_no_fallback() {
        let props = Props::<&str>::new();

        assert_eq!(props.fallback, None);
    }

    #[test]
    fn fallback_builder_sets_fallback_content() {
        let props = Props::new().fallback("Loading");

        assert_eq!(props.fallback, Some("Loading"));
    }

    #[test]
    fn props_clone_debug_cover_generic_fallback() {
        let props = Props::new().fallback(String::from("Loading"));

        let cloned = props.clone();

        assert_eq!(props.fallback, cloned.fallback);
        assert!(format!("{props:?}").contains("Loading"));
    }
}
