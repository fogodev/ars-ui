//! Keyboard-modality tracking for focus-visible styling.

use core::sync::atomic::{AtomicBool, Ordering};

use ars_core::{AttrMap, AttrValue, HtmlAttr, KeyModifiers, KeyboardKey};

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

    #[test]
    fn focus_ring_starts_hidden() {
        let ring = FocusRing::new();

        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_pointer_down_clears_focus_visible_state() {
        let ring = FocusRing::new();

        ring.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        assert!(ring.should_show_focus_ring());

        ring.on_pointer_down();
        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_only_activates_for_navigation_and_function_keys() {
        let ring = FocusRing::new();

        ring.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        assert!(ring.should_show_focus_ring());

        ring.on_pointer_down();
        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(KeyboardKey::F12, KeyModifiers::default());
        assert!(ring.should_show_focus_ring());

        ring.on_pointer_down();
        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(KeyboardKey::Shift, KeyModifiers::default());
        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(KeyboardKey::Copy, KeyModifiers::default());
        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_ignores_ctrl_meta_and_alt_key_chords() {
        let ring = FocusRing::new();

        ring.on_key_down(
            KeyboardKey::ArrowRight,
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
        );

        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(
            KeyboardKey::Tab,
            KeyModifiers {
                meta: true,
                ..KeyModifiers::default()
            },
        );

        assert!(!ring.should_show_focus_ring());

        ring.on_key_down(
            KeyboardKey::Enter,
            KeyModifiers {
                alt: true,
                ..KeyModifiers::default()
            },
        );

        assert!(!ring.should_show_focus_ring());
    }

    #[test]
    fn focus_ring_virtual_input_marks_focus_visible() {
        let ring = FocusRing::new();

        ring.on_virtual_input();
        assert!(ring.should_show_focus_ring());
    }

    #[test]
    fn cloned_focus_ring_preserves_keyboard_modality_state() {
        let ring = FocusRing::new();

        ring.on_key_down(KeyboardKey::Tab, KeyModifiers::default());

        let cloned = ring.clone();

        assert!(cloned.should_show_focus_ring());
    }

    #[test]
    fn apply_focus_attrs_writes_and_clears_attribute_for_visible_focus() {
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

    #[test]
    fn apply_focus_attrs_uses_virtual_input_state() {
        let ring = FocusRing::new();

        let mut attrs = AttrMap::new();

        ring.on_virtual_input();
        ring.apply_focus_attrs(&mut attrs, true);
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-visible")),
            Some(&AttrValue::Bool(true))
        );

        ring.on_pointer_down();
        ring.apply_focus_attrs(&mut attrs, true);
        assert_eq!(attrs.get_value(&HtmlAttr::Data("ars-focus-visible")), None);
    }
}
