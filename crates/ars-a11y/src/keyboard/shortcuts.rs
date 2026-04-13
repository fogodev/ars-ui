//! Platform-agnostic keyboard shortcut descriptors.

/// Minimal keyboard event trait for platform-agnostic modifier matching.
///
/// Adapter layers implement this for their framework's event types
/// (for example, `web_sys::KeyboardEvent` or framework-specific keyboard
/// event wrappers).
///
/// `ctrl_key()` and `meta_key()` expose raw modifier state. Callers that need
/// cross-platform "action key" semantics must use [`KeyModifiers::matches_event`]
/// instead of reading `ctrl_key()` or `meta_key()` directly.
pub trait DomEvent {
    /// Returns the event key value, if the platform exposes one.
    fn key(&self) -> Option<&str>;

    /// Returns whether the Shift modifier is pressed.
    fn shift_key(&self) -> bool;

    /// Returns whether the physical Ctrl modifier is pressed.
    fn ctrl_key(&self) -> bool;

    /// Returns whether the Meta/Cmd modifier is pressed.
    fn meta_key(&self) -> bool;

    /// Returns whether the Alt/Option modifier is pressed.
    fn alt_key(&self) -> bool;

    /// Returns the DOM-style event type, such as `keydown`.
    fn event_type(&self) -> &str;

    /// Prevents the event's default action.
    fn prevent_default(&self);

    /// Stops further propagation of the event.
    fn stop_propagation(&self);
}

/// A keyboard shortcut descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardShortcut {
    /// The key identifier for the shortcut.
    pub key: &'static str,
    /// The normalized modifier combination required by the shortcut.
    pub modifiers: KeyModifiers,
    /// Scope where the shortcut is active. `None` means global.
    pub scope: Option<&'static str>,
}

/// Modifier key combination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    /// Whether Shift must be pressed.
    pub shift: bool,
    /// Whether the platform-specific action key must be pressed.
    ///
    /// This abstracts Ctrl on Windows/Linux and Meta/Cmd on macOS and iOS.
    pub action: bool,
    /// Whether Alt/Option must be pressed.
    pub alt: bool,
}

impl KeyModifiers {
    /// No modifiers required.
    pub const NONE: Self = Self {
        shift: false,
        action: false,
        alt: false,
    };

    /// Shift only.
    pub const SHIFT: Self = Self {
        shift: true,
        action: false,
        alt: false,
    };

    /// Action key only.
    pub const ACTION: Self = Self {
        shift: false,
        action: true,
        alt: false,
    };

    /// Action key plus Shift.
    pub const ACTION_SHIFT: Self = Self {
        shift: true,
        action: true,
        alt: false,
    };

    /// Alt/Option only.
    pub const ALT: Self = Self {
        shift: false,
        action: false,
        alt: true,
    };

    /// Returns true if the event's modifier state matches this descriptor.
    pub fn matches_event(&self, event: &dyn DomEvent, platform: Platform) -> bool {
        let (action_pressed, extra_modifier_pressed) = match platform {
            Platform::MacOs | Platform::IOS => (event.meta_key(), event.ctrl_key()),
            Platform::Windows | Platform::Linux | Platform::Unknown => {
                (event.ctrl_key(), event.meta_key())
            }
        };

        self.shift == event.shift_key()
            && self.action == action_pressed
            && self.alt == event.alt_key()
            && !extra_modifier_pressed
    }
}

/// Platform identifier for modifier key normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// macOS desktop and laptop hardware.
    MacOs,
    /// iOS and iPadOS devices.
    IOS,
    /// Microsoft Windows platforms.
    Windows,
    /// Linux and Linux-like desktop platforms.
    Linux,
    /// A platform that does not match the supported detection strings.
    Unknown,
}

impl Platform {
    /// Detects the current platform from `navigator.platform` and touch support.
    ///
    /// iPadOS 13+ reports `"MacIntel"` from `navigator.platform`, so
    /// `max_touch_points > 1` is used to distinguish it from actual macOS
    /// hardware.
    pub fn detect(navigator_platform: &str, max_touch_points: u32) -> Self {
        if navigator_platform.contains("iPhone") || navigator_platform.contains("iPad") {
            Self::IOS
        } else if navigator_platform.contains("Mac") {
            if max_touch_points > 1 {
                Self::IOS
            } else {
                Self::MacOs
            }
        } else if navigator_platform.contains("Win") {
            Self::Windows
        } else if navigator_platform.contains("Linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }

    /// Returns the human-readable action-key label used in shortcut UI.
    pub const fn action_key_label(self) -> &'static str {
        match self {
            Self::MacOs | Self::IOS => "⌘",
            Self::Windows | Self::Linux | Self::Unknown => "Ctrl",
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;
    extern crate std;

    use core::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    #[derive(Debug, Default)]
    struct TestEvent {
        key: Option<&'static str>,
        shift: bool,
        ctrl: bool,
        meta: bool,
        alt: bool,
        event_type: &'static str,
        default_prevented: Cell<bool>,
        propagation_stopped: Cell<bool>,
    }

    impl DomEvent for TestEvent {
        fn key(&self) -> Option<&str> {
            self.key
        }

        fn shift_key(&self) -> bool {
            self.shift
        }

        fn ctrl_key(&self) -> bool {
            self.ctrl
        }

        fn meta_key(&self) -> bool {
            self.meta
        }

        fn alt_key(&self) -> bool {
            self.alt
        }

        fn event_type(&self) -> &str {
            self.event_type
        }

        fn prevent_default(&self) {
            self.default_prevented.set(true);
        }

        fn stop_propagation(&self) {
            self.propagation_stopped.set(true);
        }
    }

    fn hash_shortcut(shortcut: &KeyboardShortcut) -> u64 {
        let mut hasher = DefaultHasher::new();
        shortcut.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn platform_detect_returns_macos_for_mac_without_touch_points() {
        assert_eq!(Platform::detect("MacIntel", 0), Platform::MacOs);
    }

    #[test]
    fn platform_detect_returns_ios_for_mac_with_touch_points() {
        assert_eq!(Platform::detect("MacIntel", 2), Platform::IOS);
    }

    #[test]
    fn platform_detect_returns_ios_for_iphone() {
        assert_eq!(Platform::detect("iPhone", 0), Platform::IOS);
    }

    #[test]
    fn platform_detect_returns_ios_for_ipad() {
        assert_eq!(Platform::detect("iPad", 0), Platform::IOS);
    }

    #[test]
    fn platform_detect_returns_windows() {
        assert_eq!(Platform::detect("Win32", 0), Platform::Windows);
    }

    #[test]
    fn platform_detect_returns_linux() {
        assert_eq!(Platform::detect("Linux x86_64", 0), Platform::Linux);
    }

    #[test]
    fn platform_detect_returns_unknown_for_other_values() {
        assert_eq!(Platform::detect("other", 0), Platform::Unknown);
    }

    #[test]
    fn action_key_label_matches_platform() {
        assert_eq!(Platform::MacOs.action_key_label(), "⌘");
        assert_eq!(Platform::IOS.action_key_label(), "⌘");
        assert_eq!(Platform::Windows.action_key_label(), "Ctrl");
        assert_eq!(Platform::Linux.action_key_label(), "Ctrl");
        assert_eq!(Platform::Unknown.action_key_label(), "Ctrl");
    }

    #[test]
    fn none_matches_event_without_modifiers() {
        let event = TestEvent::default();

        assert!(KeyModifiers::NONE.matches_event(&event, Platform::Windows));
    }

    #[test]
    fn shift_matches_only_shift_without_extra_modifiers() {
        let matching = TestEvent {
            shift: true,
            ..TestEvent::default()
        };
        let extra_alt = TestEvent {
            shift: true,
            alt: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::SHIFT.matches_event(&matching, Platform::Windows));
        assert!(!KeyModifiers::SHIFT.matches_event(&extra_alt, Platform::Windows));
    }

    #[test]
    fn alt_matches_only_alt_without_extra_modifiers() {
        let matching = TestEvent {
            alt: true,
            ..TestEvent::default()
        };
        let extra_shift = TestEvent {
            shift: true,
            alt: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::ALT.matches_event(&matching, Platform::Windows));
        assert!(!KeyModifiers::ALT.matches_event(&extra_shift, Platform::Windows));
    }

    #[test]
    fn action_matches_ctrl_on_windows() {
        let event = TestEvent {
            ctrl: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::ACTION.matches_event(&event, Platform::Windows));
    }

    #[test]
    fn action_matches_meta_on_macos() {
        let event = TestEvent {
            meta: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::ACTION.matches_event(&event, Platform::MacOs));
    }

    #[test]
    fn action_matches_meta_on_ios() {
        let event = TestEvent {
            meta: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::ACTION.matches_event(&event, Platform::IOS));
    }

    #[test]
    fn action_rejects_ctrl_without_meta_on_macos() {
        let event = TestEvent {
            ctrl: true,
            ..TestEvent::default()
        };

        assert!(!KeyModifiers::ACTION.matches_event(&event, Platform::MacOs));
    }

    #[test]
    fn action_rejects_ctrl_without_meta_on_ios() {
        let event = TestEvent {
            ctrl: true,
            ..TestEvent::default()
        };

        assert!(!KeyModifiers::ACTION.matches_event(&event, Platform::IOS));
    }

    #[test]
    fn action_rejects_ctrl_with_meta_on_macos() {
        let event = TestEvent {
            ctrl: true,
            meta: true,
            ..TestEvent::default()
        };

        assert!(!KeyModifiers::ACTION.matches_event(&event, Platform::MacOs));
    }

    #[test]
    fn action_rejects_meta_with_ctrl_on_windows() {
        let event = TestEvent {
            ctrl: true,
            meta: true,
            ..TestEvent::default()
        };

        assert!(!KeyModifiers::ACTION.matches_event(&event, Platform::Windows));
    }

    #[test]
    fn action_shift_requires_both_modifiers() {
        let matching = TestEvent {
            shift: true,
            ctrl: true,
            ..TestEvent::default()
        };
        let missing_shift = TestEvent {
            ctrl: true,
            ..TestEvent::default()
        };
        let missing_action = TestEvent {
            shift: true,
            ..TestEvent::default()
        };

        assert!(KeyModifiers::ACTION_SHIFT.matches_event(&matching, Platform::Windows));
        assert!(!KeyModifiers::ACTION_SHIFT.matches_event(&missing_shift, Platform::Windows));
        assert!(!KeyModifiers::ACTION_SHIFT.matches_event(&missing_action, Platform::Windows));
    }

    #[test]
    fn matches_event_rejects_extra_modifiers() {
        let event = TestEvent {
            ctrl: true,
            alt: true,
            ..TestEvent::default()
        };

        assert!(!KeyModifiers::ACTION.matches_event(&event, Platform::Windows));
    }

    #[test]
    fn keyboard_shortcut_equality_and_hashing_work_for_registries() {
        let first = KeyboardShortcut {
            key: "k",
            modifiers: KeyModifiers::ACTION,
            scope: Some("palette"),
        };
        let second = KeyboardShortcut {
            key: "k",
            modifiers: KeyModifiers::ACTION,
            scope: Some("palette"),
        };
        let different = KeyboardShortcut {
            key: "k",
            modifiers: KeyModifiers::ACTION_SHIFT,
            scope: Some("palette"),
        };

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert_eq!(hash_shortcut(&first), hash_shortcut(&second));
        assert_ne!(hash_shortcut(&first), hash_shortcut(&different));
    }

    #[test]
    fn dom_event_trait_surface_is_exercised_by_test_event() {
        let event = TestEvent {
            key: Some("k"),
            event_type: "keyup",
            ..TestEvent::default()
        };

        assert_eq!(event.key(), Some("k"));
        assert_eq!(event.event_type(), "keyup");
        assert!(!event.default_prevented.get());
        assert!(!event.propagation_stopped.get());

        event.prevent_default();
        event.stop_propagation();

        assert!(event.default_prevented.get());
        assert!(event.propagation_stopped.get());
    }
}
