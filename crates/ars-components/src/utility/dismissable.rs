//! Dismissable behavior configuration and dismiss-button attributes.
//!
//! This module defines the adapter-facing data model for dismissable overlay
//! behavior — the props, component parts, and
//! [`dismiss_button_attrs`] function used by overlay components (`Dialog`,
//! `Popover`, `Tooltip`, `Select`, etc.) to produce consistent click-outside
//! and Escape-to-close behavior.
//!
//! The module intentionally stays free of DOM or framework types so that
//! attribute generation can be tested with pure unit tests.
//!
//! **Dismiss-button wording is not part of [`Props`].** Callers resolve a
//! localized label in their own message bundle and pass the final string to
//! [`dismiss_button_attrs`]. This keeps the shared utility focused on behavior
//! and structure rather than owning overlay-specific wording.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{AriaAttr, AttrMap, Callback, ComponentPart, HtmlAttr};
use ars_interactions::InteractOutsideEvent;

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
/// blocking, and excluded IDs. Accessible wording for the dismiss button is
/// resolved by the caller and passed separately to [`dismiss_button_attrs`].
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

impl Debug for Props {
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
/// the caller-provided `aria-label`.
#[must_use]
pub fn dismiss_button_attrs(label: &str) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();

    attrs
        .set(scope_attr, scope_val)
        .set(part_attr, part_val)
        .set(HtmlAttr::Role, "button")
        .set(HtmlAttr::TabIndex, "0")
        .set(HtmlAttr::Aria(AriaAttr::Label), label)
        .set_bool(HtmlAttr::Data("ars-visually-hidden"), true);

    attrs
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ars_core::AttrValue;

    use super::*;

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
        let interact_calls = Arc::new(AtomicUsize::new(0));
        let escape_calls = Arc::new(AtomicUsize::new(0));
        let dismiss_calls = Arc::new(AtomicUsize::new(0));

        let props = Props {
            on_interact_outside: Some({
                let interact_calls = Arc::clone(&interact_calls);
                Callback::new(move |_: InteractOutsideEvent| {
                    interact_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_escape_key_down: Some({
                let escape_calls = Arc::clone(&escape_calls);
                Callback::new_void(move || {
                    escape_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_dismiss: Some({
                let dismiss_calls = Arc::clone(&dismiss_calls);
                Callback::new_void(move || {
                    dismiss_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..Props::default()
        };

        let debug = format!("{props:?}");

        assert!(debug.contains("disable_outside_pointer_events: false"));
        assert!(debug.contains("on_interact_outside: Some(\"<closure>\")"));
        assert!(debug.contains("on_escape_key_down: Some(\"<closure>\")"));
        assert!(debug.contains("on_dismiss: Some(\"<closure>\")"));

        props.on_interact_outside.as_ref().expect("callback")(InteractOutsideEvent::EscapeKey);
        props.on_escape_key_down.as_ref().expect("callback")();
        props.on_dismiss.as_ref().expect("callback")();

        assert_eq!(interact_calls.load(Ordering::SeqCst), 1);
        assert_eq!(escape_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dismiss_calls.load(Ordering::SeqCst), 1);
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
        let calls = Arc::new(AtomicUsize::new(0));

        let cb = {
            let calls = Arc::clone(&calls);
            Callback::new(move |_: InteractOutsideEvent| {
                calls.fetch_add(1, Ordering::SeqCst);
            })
        };

        let props = Props {
            on_interact_outside: Some(cb.clone()),
            ..Props::default()
        };

        let cloned = props.clone();

        assert_eq!(props.on_interact_outside, cloned.on_interact_outside);

        cloned.on_interact_outside.as_ref().expect("callback")(InteractOutsideEvent::EscapeKey);

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn props_partial_eq_uses_callback_pointer_identity() {
        let shared_calls = Arc::new(AtomicUsize::new(0));

        let cb = {
            let shared_calls = Arc::clone(&shared_calls);
            Callback::new_void(move || {
                shared_calls.fetch_add(1, Ordering::SeqCst);
            })
        };

        let props1 = Props {
            on_dismiss: Some(cb.clone()),
            ..Props::default()
        };

        let props2 = Props {
            on_dismiss: Some(cb),
            ..Props::default()
        };

        assert_eq!(props1, props2);

        props2.on_dismiss.as_ref().expect("callback")();

        assert_eq!(shared_calls.load(Ordering::SeqCst), 1);

        let different_calls = Arc::new(AtomicUsize::new(0));

        let props3 = Props {
            on_dismiss: Some({
                let different_calls = Arc::clone(&different_calls);
                Callback::new_void(move || {
                    different_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..Props::default()
        };

        assert_ne!(props1, props3);

        props3.on_dismiss.as_ref().expect("callback")();

        assert_eq!(different_calls.load(Ordering::SeqCst), 1);
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

    #[test]
    fn dismiss_button_attrs_sets_scope_and_part() {
        let attrs = dismiss_button_attrs("Dismiss");

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("dismissable"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")),
            Some("dismiss-button")
        );
    }

    #[test]
    fn dismiss_button_attrs_sets_role_button() {
        let attrs = dismiss_button_attrs("Dismiss");

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
    }

    #[test]
    fn dismiss_button_attrs_sets_tabindex_zero() {
        let attrs = dismiss_button_attrs("Dismiss");

        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn dismiss_button_attrs_sets_visually_hidden() {
        let attrs = dismiss_button_attrs("Dismiss");

        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-visually-hidden")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn dismiss_button_attrs_uses_provided_label() {
        let attrs = dismiss_button_attrs("Dismiss");

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Dismiss"));
    }

    #[test]
    fn dismiss_button_attrs_preserves_custom_label_text() {
        let attrs = dismiss_button_attrs("Close");

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Close"));
    }

    #[test]
    fn dismiss_button_attrs_accepts_overlay_specific_wording() {
        let attrs = dismiss_button_attrs("Dismiss popover");

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Dismiss popover")
        );
    }
}
