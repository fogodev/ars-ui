//! Shared focus-management contracts.

use core::sync::atomic::{AtomicBool, Ordering};

use ars_core::{AttrMap, AttrValue, HtmlAttr, KeyModifiers, KeyboardKey};

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

/// Tracks whether the current focus should render a visible keyboard ring.
///
/// `FocusRing` is intentionally separate from the shared modality context: it
/// owns accessibility-specific focus-visible heuristics while consuming the same
/// normalized event stream adapters feed into modality tracking.
#[derive(Debug, Default)]
pub struct FocusRing {
    keyboard_modality: AtomicBool,
}

impl Clone for FocusRing {
    fn clone(&self) -> Self {
        Self {
            keyboard_modality: AtomicBool::new(self.keyboard_modality.load(Ordering::Relaxed)),
        }
    }
}

impl FocusRing {
    /// Creates a new focus-ring tracker with no active keyboard modality.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keyboard_modality: AtomicBool::new(false),
        }
    }

    /// Records a pointer interaction and suppresses keyboard-only focus styling.
    pub fn on_pointer_down(&self) {
        self.keyboard_modality.store(false, Ordering::Relaxed);
    }

    /// Records a keyboard interaction that implies intentional focus navigation.
    ///
    /// Modified key chords using Ctrl, Meta, or Alt are ignored because they
    /// typically represent browser or operating-system shortcuts rather than
    /// in-document focus navigation.
    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        if modifiers.ctrl || modifiers.meta || modifiers.alt {
            return;
        }

        if matches!(
            key,
            KeyboardKey::Tab
                | KeyboardKey::ArrowUp
                | KeyboardKey::ArrowDown
                | KeyboardKey::ArrowLeft
                | KeyboardKey::ArrowRight
                | KeyboardKey::Home
                | KeyboardKey::End
                | KeyboardKey::PageUp
                | KeyboardKey::PageDown
                | KeyboardKey::Enter
                | KeyboardKey::Space
                | KeyboardKey::Escape
                | KeyboardKey::F1
                | KeyboardKey::F2
                | KeyboardKey::F3
                | KeyboardKey::F4
                | KeyboardKey::F5
                | KeyboardKey::F6
                | KeyboardKey::F7
                | KeyboardKey::F8
                | KeyboardKey::F9
                | KeyboardKey::F10
                | KeyboardKey::F11
                | KeyboardKey::F12
        ) {
            self.keyboard_modality.store(true, Ordering::Relaxed);
        }
    }

    /// Records a virtual or assistive-technology interaction as focus-visible.
    pub fn on_virtual_input(&self) {
        self.keyboard_modality.store(true, Ordering::Relaxed);
    }

    /// Returns whether the next focused element should render the focus ring.
    #[must_use]
    pub fn should_show_focus_ring(&self) -> bool {
        self.keyboard_modality.load(Ordering::Relaxed)
    }

    /// Writes the canonical focus-visible data attribute into an [`AttrMap`].
    pub fn apply_focus_attrs(&self, attrs: &mut AttrMap, is_focused: bool) {
        if is_focused && self.should_show_focus_ring() {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        } else {
            attrs.set(HtmlAttr::Data("ars-focus-visible"), AttrValue::None);
        }
    }
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

    #[test]
    fn focus_ring_starts_hidden() {
        let ring = FocusRing::new();
        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_tracks_keyboard_navigation_keys() {
        let ring = FocusRing::new();
        ring.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        assert!(ring.should_show_focus_ring());

        ring.on_pointer_down();
        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(
            KeyboardKey::ArrowRight,
            KeyModifiers {
                shift: false,
                ctrl: true,
                alt: false,
                meta: false,
            },
        );
        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_virtual_input_is_visible() {
        let ring = FocusRing::new();
        ring.on_virtual_input();
        assert!(ring.should_show_focus_ring());
    }

    #[test]
    fn apply_focus_attrs_writes_and_clears_attribute() {
        let ring = FocusRing::new();
        let mut attrs = AttrMap::new();

        ring.on_pointer_down();
        ring.apply_focus_attrs(&mut attrs, true);
        assert_eq!(attrs.get_value(&HtmlAttr::Data("ars-focus-visible")), None);

        ring.on_key_down(KeyboardKey::Enter, KeyModifiers::default());
        ring.apply_focus_attrs(&mut attrs, true);
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-visible")),
            Some(&AttrValue::Bool(true))
        );

        ring.apply_focus_attrs(&mut attrs, false);
        assert_eq!(attrs.get_value(&HtmlAttr::Data("ars-focus-visible")), None);
    }
}
