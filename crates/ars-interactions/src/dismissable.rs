//! Dismissable behavior configuration and dismiss-button attributes.
//!
//! This module defines the adapter-facing data model for dismissable overlay
//! behavior — the props, localizable messages, component parts, and
//! [`dismiss_button_attrs`] function used by overlay components (`Dialog`,
//! `Popover`, `Tooltip`, `Select`, etc.) to produce consistent click-outside
//! and Escape-to-close behavior.
//!
//! The module intentionally stays free of DOM or framework types so that
//! attribute generation can be tested with pure unit tests.
//!
//! **Locale and messages are not part of [`Props`].** They are environment
//! context resolved by the adapter from `ArsProvider` and passed as explicit
//! parameters to [`dismiss_button_attrs`]. This keeps the core crate
//! framework-agnostic and fully testable without reactive context.

use std::{fmt, string::String, vec::Vec};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentMessages, ComponentPart, HtmlAttr, MessageFn,
};
use ars_i18n::Locale;

use super::InteractOutsideEvent;

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings for the Dismissable component.
///
/// Contains a single message field for the accessible label applied to the
/// visually-hidden dismiss button. The adapter resolves a `Messages` value
/// from the three-level chain (prop override → `I18nRegistries` →
/// `Messages::default()`) and passes it to [`dismiss_button_attrs`].
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the visually-hidden `DismissButton`.
    ///
    /// Receives the active [`Locale`] so translators can return per-locale text.
    /// The default returns `"Dismiss"` regardless of locale.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            close_label: MessageFn::static_str("Dismiss"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// DOM parts of the Dismissable component.
#[derive(ComponentPart)]
#[scope = "dismissable"]
pub enum Part {
    /// The root dismissable container.
    Root,
    /// The visually-hidden dismiss button placed at the start and end of
    /// a dismissable region, giving screen reader users a click target to
    /// dismiss overlays without relying on Escape.
    DismissButton,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the Dismissable component.
///
/// Contains only behavioral configuration — callbacks, pointer-event
/// blocking, and excluded IDs. Locale and messages are environment
/// context resolved by the adapter and passed separately to
/// [`dismiss_button_attrs`].
#[derive(Clone, Default, PartialEq)]
pub struct Props {
    /// Called when the user interacts outside the dismissable element.
    ///
    /// The adapter invokes this on `pointerdown` outside, or `focusin` on
    /// an element outside.
    pub on_interact_outside: Option<Callback<dyn Fn(InteractOutsideEvent)>>,

    /// Called when the user presses the Escape key while focus is inside.
    pub on_escape_key_down: Option<Callback<dyn Fn()>>,

    /// Called when a dismiss trigger fires (combines outside interaction
    /// and Escape).
    pub on_dismiss: Option<Callback<dyn Fn()>>,

    /// When true, outside pointer events are intercepted and prevented from
    /// reaching underlying elements (transparent overlay with
    /// `pointer-events: auto`). Default: `false`.
    pub disable_outside_pointer_events: bool,

    /// DOM IDs of elements that should NOT trigger an outside interaction
    /// when clicked. Typically includes the trigger button that opened the
    /// overlay.
    pub exclude_ids: Vec<String>,
}

impl fmt::Debug for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Props")
            .field(
                "disable_outside_pointer_events",
                &self.disable_outside_pointer_events,
            )
            .field(
                "on_interact_outside",
                &self.on_interact_outside.as_ref().map(|_| "<closure>"),
            )
            .field(
                "on_escape_key_down",
                &self.on_escape_key_down.as_ref().map(|_| "<closure>"),
            )
            .field("on_dismiss", &self.on_dismiss.as_ref().map(|_| "<closure>"))
            .finish()
    }
}

// ────────────────────────────────────────────────────────────────────
// dismiss_button_attrs
// ────────────────────────────────────────────────────────────────────

/// Returns attributes for the visually-hidden `DismissButton` element.
///
/// Produces scope/part data attributes, native button semantics
/// (`role="button"`, `tabindex="0"`), a visually-hidden marker, and
/// the localized `aria-label`.
///
/// `locale` and `messages` are resolved by the adapter from `ArsProvider`
/// context and passed explicitly — this function has no framework dependency.
#[must_use]
pub fn dismiss_button_attrs(locale: &Locale, messages: &Messages) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "button");
    attrs.set(HtmlAttr::TabIndex, "0");
    attrs.set(
        HtmlAttr::Aria(AriaAttr::Label),
        (messages.close_label)(locale),
    );
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden"), true);
    attrs
}

#[cfg(test)]
mod tests {
    use ars_core::AttrValue;
    use ars_i18n::locales;

    use super::*;

    // ── Messages tests ─────────────────────────────────────────────

    #[test]
    fn messages_default_close_label_returns_dismiss() {
        let messages = Messages::default();
        let locale = locales::en_us();
        assert_eq!((messages.close_label)(&locale), "Dismiss");
    }

    #[test]
    fn messages_default_close_label_ignores_locale() {
        let messages = Messages::default();
        assert_eq!((messages.close_label)(&locales::ja_jp()), "Dismiss");
    }

    #[test]
    fn messages_clone_shares_pointer_identity() {
        let messages = Messages::default();
        let cloned = messages.clone();
        assert_eq!(messages.close_label, cloned.close_label);
    }

    #[test]
    fn messages_partial_eq_different_allocations_are_not_equal() {
        let m1 = Messages::default();
        let m2 = Messages::default();
        assert_ne!(m1, m2);
    }

    #[test]
    fn messages_debug_output_shows_closure_marker() {
        let messages = Messages::default();
        let debug = format!("{messages:?}");
        assert!(debug.contains("close_label: <closure>"));
    }

    // ── Part tests ─────────────────────────────────────────────────

    #[test]
    fn part_scope_is_dismissable() {
        assert_eq!(Part::scope(), "dismissable");
    }

    #[test]
    fn part_root_name_is_root() {
        assert_eq!(Part::Root.name(), "root");
    }

    #[test]
    fn part_dismiss_button_name_is_dismiss_button() {
        assert_eq!(Part::DismissButton.name(), "dismiss-button");
    }

    #[test]
    fn part_data_attrs_produce_scope_and_part() {
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
        assert_eq!(scope_attr, HtmlAttr::Data("ars-scope"));
        assert_eq!(scope_val, "dismissable");
        assert_eq!(part_attr, HtmlAttr::Data("ars-part"));
        assert_eq!(part_val, "dismiss-button");
    }

    #[test]
    fn part_all_returns_both_variants() {
        let all = Part::all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], Part::Root);
        assert_eq!(all[1], Part::DismissButton);
    }

    // ── Props tests ────────────────────────────────────────────────

    #[test]
    fn props_default_values() {
        let props = Props::default();
        assert!(props.on_interact_outside.is_none());
        assert!(props.on_escape_key_down.is_none());
        assert!(props.on_dismiss.is_none());
        assert!(!props.disable_outside_pointer_events);
        assert!(props.exclude_ids.is_empty());
    }

    #[test]
    fn props_debug_redacts_callbacks_when_some() {
        let props = Props {
            on_interact_outside: Some(Callback::new(|_: InteractOutsideEvent| {})),
            on_escape_key_down: Some(Callback::new_void(|| {})),
            on_dismiss: Some(Callback::new_void(|| {})),
            ..Props::default()
        };
        let debug = format!("{props:?}");
        assert!(debug.contains("disable_outside_pointer_events: false"));
        assert!(debug.contains("on_interact_outside: Some(\"<closure>\")"));
        assert!(debug.contains("on_escape_key_down: Some(\"<closure>\")"));
        assert!(debug.contains("on_dismiss: Some(\"<closure>\")"));
    }

    #[test]
    fn props_debug_shows_none_when_no_callbacks() {
        let props = Props::default();
        let debug = format!("{props:?}");
        assert!(debug.contains("on_interact_outside: None"));
        assert!(debug.contains("on_escape_key_down: None"));
        assert!(debug.contains("on_dismiss: None"));
    }

    #[test]
    fn props_clone_preserves_callback_pointer_identity() {
        let cb = Callback::new(|_: InteractOutsideEvent| {});
        let props = Props {
            on_interact_outside: Some(cb.clone()),
            ..Props::default()
        };
        let cloned = props.clone();
        assert_eq!(props.on_interact_outside, cloned.on_interact_outside);
    }

    #[test]
    fn props_partial_eq_uses_callback_pointer_identity() {
        let cb = Callback::new_void(|| {});
        let props1 = Props {
            on_dismiss: Some(cb.clone()),
            ..Props::default()
        };
        let props2 = Props {
            on_dismiss: Some(cb),
            ..Props::default()
        };
        assert_eq!(props1, props2);

        let props3 = Props {
            on_dismiss: Some(Callback::new_void(|| {})),
            ..Props::default()
        };
        assert_ne!(props1, props3);
    }

    #[test]
    fn props_exclude_ids_preserved_through_clone() {
        let props = Props {
            exclude_ids: vec!["trigger-1".into(), "trigger-2".into()],
            ..Props::default()
        };
        let cloned = props.clone();
        assert_eq!(cloned.exclude_ids, vec!["trigger-1", "trigger-2"]);
    }

    #[test]
    fn props_disable_outside_pointer_events_preserved() {
        let props = Props {
            disable_outside_pointer_events: true,
            ..Props::default()
        };
        assert!(props.disable_outside_pointer_events);
        let cloned = props.clone();
        assert!(cloned.disable_outside_pointer_events);
    }

    // ── dismiss_button_attrs tests ─────────────────────────────────

    fn default_locale() -> Locale {
        locales::en_us()
    }

    #[test]
    fn dismiss_button_attrs_sets_scope_and_part() {
        let attrs = dismiss_button_attrs(&default_locale(), &Messages::default());
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("dismissable"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")),
            Some("dismiss-button")
        );
    }

    #[test]
    fn dismiss_button_attrs_sets_role_button() {
        let attrs = dismiss_button_attrs(&default_locale(), &Messages::default());
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
    }

    #[test]
    fn dismiss_button_attrs_sets_tabindex_zero() {
        let attrs = dismiss_button_attrs(&default_locale(), &Messages::default());
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn dismiss_button_attrs_sets_visually_hidden() {
        let attrs = dismiss_button_attrs(&default_locale(), &Messages::default());
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-visually-hidden")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn dismiss_button_attrs_default_aria_label_is_dismiss() {
        let attrs = dismiss_button_attrs(&default_locale(), &Messages::default());
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Dismiss"));
    }

    #[test]
    fn dismiss_button_attrs_uses_custom_messages() {
        let messages = Messages {
            close_label: MessageFn::static_str("Close"),
        };
        let attrs = dismiss_button_attrs(&default_locale(), &messages);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Close"));
    }

    #[test]
    fn dismiss_button_attrs_uses_locale_in_message_fn() {
        let messages = Messages {
            close_label: MessageFn::from(|locale: &Locale| {
                if locale.to_bcp47() == "de-DE" {
                    "Schlie\u{00df}en".into()
                } else {
                    "Dismiss".into()
                }
            }),
        };
        let attrs = dismiss_button_attrs(&locales::de_de(), &messages);
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Schlie\u{00df}en")
        );
    }
}
