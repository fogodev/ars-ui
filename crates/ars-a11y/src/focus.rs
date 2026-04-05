//! Shared focus-management contracts.

/// Options controlling [`FocusScopeBehavior`] implementations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusScopeOptions {
    /// If true, Tab and Shift+Tab are prevented from leaving the scope.
    pub contain: bool,
    /// If true, focus is restored to the previously focused element when the scope deactivates.
    pub restore_focus: bool,
    /// If true, focus moves into the scope when it activates.
    pub auto_focus: bool,
}

impl Default for FocusScopeOptions {
    fn default() -> Self {
        Self {
            contain: false,
            restore_focus: true,
            auto_focus: true,
        }
    }
}

impl FocusScopeOptions {
    /// Returns the preset used by modal overlays.
    #[must_use]
    pub fn modal() -> Self {
        Self {
            contain: true,
            restore_focus: true,
            auto_focus: true,
        }
    }

    /// Returns the preset used by non-modal overlays.
    #[must_use]
    pub fn overlay() -> Self {
        Self {
            contain: false,
            restore_focus: true,
            auto_focus: true,
        }
    }

    /// Returns the preset used by inline regions that do not manage focus lifecycle.
    #[must_use]
    pub fn inline() -> Self {
        Self {
            contain: false,
            restore_focus: false,
            auto_focus: false,
        }
    }
}

/// Trait defining the public interface for focus-scope behavior.
pub trait FocusScopeBehavior {
    /// Activates the scope and optionally moves focus according to `focus_target`.
    fn activate(&mut self, focus_target: FocusTarget);
    /// Deactivates the scope and releases any focus management behavior.
    fn deactivate(&mut self);
    /// Returns whether the scope is currently active.
    fn is_active(&self) -> bool;
}

/// Selects which element receives focus when a scope activates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// Focus the first tabbable element.
    First,
    /// Focus the last tabbable element.
    Last,
    /// Focus the element explicitly marked for autofocus.
    AutofocusMarked,
    /// Focus the element that was previously active inside the scope.
    PreviouslyActive,
}

/// Describes how a composite widget manages focus among its items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusStrategy {
    /// Move DOM focus between items by toggling `tabindex` values.
    #[default]
    RovingTabindex,
    /// Keep DOM focus on the container and expose the active item through `aria-activedescendant`.
    ActiveDescendant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestFocusScope {
        active: bool,
        last_target: Option<FocusTarget>,
    }

    impl FocusScopeBehavior for TestFocusScope {
        fn activate(&mut self, focus_target: FocusTarget) {
            self.active = true;
            self.last_target = Some(focus_target);
        }

        fn deactivate(&mut self) {
            self.active = false;
        }

        fn is_active(&self) -> bool {
            self.active
        }
    }

    #[test]
    fn focus_scope_options_default_matches_spec() {
        assert_eq!(
            FocusScopeOptions::default(),
            FocusScopeOptions {
                contain: false,
                restore_focus: true,
                auto_focus: true,
            }
        );
    }

    #[test]
    fn focus_scope_option_presets_match_spec() {
        assert_eq!(
            FocusScopeOptions::modal(),
            FocusScopeOptions {
                contain: true,
                restore_focus: true,
                auto_focus: true,
            }
        );
        assert_eq!(
            FocusScopeOptions::overlay(),
            FocusScopeOptions {
                contain: false,
                restore_focus: true,
                auto_focus: true,
            }
        );
        assert_eq!(
            FocusScopeOptions::inline(),
            FocusScopeOptions {
                contain: false,
                restore_focus: false,
                auto_focus: false,
            }
        );
    }

    #[test]
    fn focus_enums_support_equality_checks() {
        assert_eq!(FocusTarget::AutofocusMarked, FocusTarget::AutofocusMarked);
        assert_ne!(FocusTarget::First, FocusTarget::Last);
        assert_eq!(FocusStrategy::default(), FocusStrategy::RovingTabindex);
        assert_ne!(
            FocusStrategy::RovingTabindex,
            FocusStrategy::ActiveDescendant
        );
    }

    #[test]
    fn focus_scope_behavior_supports_trait_objects() {
        let mut scope = TestFocusScope::default();
        let behavior: &mut dyn FocusScopeBehavior = &mut scope;

        behavior.activate(FocusTarget::PreviouslyActive);
        assert!(behavior.is_active());
        behavior.deactivate();
        assert!(!behavior.is_active());

        assert_eq!(scope.last_target, Some(FocusTarget::PreviouslyActive));
    }
}
