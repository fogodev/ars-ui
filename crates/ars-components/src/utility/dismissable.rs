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

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{
    fmt::{self, Debug},
    sync::atomic::{AtomicBool, Ordering},
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentMessages, ComponentPart, HtmlAttr, MessageFn,
};
use ars_i18n::Locale;
use ars_interactions::InteractOutsideEvent;

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable messages for the dismissable utility.
///
/// Adapters resolve this bundle through the standard provider stack so the
/// visually-hidden dismiss buttons get a locale-aware `aria-label` even
/// when the embedding overlay does not pass one explicitly. Overlay
/// components that own their own wording (e.g. "Dismiss popover") build
/// the label themselves and pass it directly to [`dismiss_button_attrs`]
/// without going through this bundle.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the localized aria-label for the visually-hidden dismiss
    /// buttons.
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dismiss_label: MessageFn::new(|_locale: &Locale| String::from("Dismiss")),
        }
    }
}

impl PartialEq for Messages {
    fn eq(&self, other: &Self) -> bool {
        self.dismiss_label == other.dismiss_label
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// DismissReason
// ────────────────────────────────────────────────────────────────────

/// Why a dismissable surface was dismissed.
///
/// Passed to [`Props::on_dismiss`] after the dismiss decision is finalized.
/// Per `spec/components/utility/dismissable.md` §11 "Callback Payload
/// Contract", the reason taxonomy carries the path that triggered the
/// dismissal so consumers (analytics, undo banners, …) can react differently
/// per source without re-implementing detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DismissReason {
    /// A pointer event landed outside the dismissable surface and outside
    /// every registered inside-boundary or portal-owner.
    OutsidePointer,

    /// Focus moved to an element outside the dismissable surface and
    /// outside every registered inside-boundary or portal-owner.
    OutsideFocus,

    /// The user pressed the `Escape` key while the dismissable was the
    /// topmost overlay.
    Escape,

    /// The user activated one of the visually-hidden dismiss buttons (or a
    /// wrapper invoked the programmatic adapter handle's `dismiss`,
    /// e.g. `ars_leptos::dismissable::Handle::dismiss`).
    DismissButton,
}

// ────────────────────────────────────────────────────────────────────
// DismissAttempt
// ────────────────────────────────────────────────────────────────────

/// Veto-capable wrapper passed to the dismiss-decision callbacks
/// (`on_interact_outside`, `on_escape_key_down`).
///
/// Per `spec/components/utility/dismissable.md` §11, those callbacks fire
/// **before** the final dismiss decision and may cancel it. Calling
/// [`prevent_dismiss`](Self::prevent_dismiss) sets a shared flag that the
/// adapter checks before dispatching `on_dismiss`. The flag is backed by an
/// [`Arc<AtomicBool>`] so the consumer's callback observation is visible to
/// the adapter regardless of thread of origin.
///
/// `event` is the underlying payload (e.g. [`InteractOutsideEvent`] for
/// outside-interaction callbacks, `()` for Escape).
pub struct DismissAttempt<E> {
    /// Underlying event payload.
    pub event: E,
    veto: Arc<AtomicBool>,
}

impl<E> DismissAttempt<E> {
    /// Creates a fresh dismiss attempt that is initially not vetoed.
    #[must_use]
    pub fn new(event: E) -> Self {
        Self {
            event,
            veto: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Marks the dismissal attempt as vetoed.
    ///
    /// After this call the adapter will skip `on_dismiss` for this event.
    /// Idempotent — calling repeatedly has no additional effect.
    pub fn prevent_dismiss(&self) {
        self.veto.store(true, Ordering::SeqCst);
    }

    /// Returns whether [`prevent_dismiss`](Self::prevent_dismiss) has been
    /// called for this attempt.
    #[must_use]
    pub fn is_prevented(&self) -> bool {
        self.veto.load(Ordering::SeqCst)
    }
}

impl<E: Clone> Clone for DismissAttempt<E> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            veto: Arc::clone(&self.veto),
        }
    }
}

impl<E: Debug> Debug for DismissAttempt<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DismissAttempt")
            .field("event", &self.event)
            .field("prevented", &self.is_prevented())
            .finish()
    }
}

impl<E: PartialEq> PartialEq for DismissAttempt<E> {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event && Arc::ptr_eq(&self.veto, &other.veto)
    }
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// DOM parts of the Dismissable component.
#[derive(ComponentPart)]
#[scope = "dismissable"]
pub enum Part {
    /// The root dismissable container.
    Root,

    /// The visually-hidden dismiss button. The spec mandates **two** of
    /// these — one at the start of the region, one at the end — both
    /// firing [`DismissReason::DismissButton`] identically. The
    /// duplication is deliberate and serves assistive-technology paths
    /// only:
    ///
    /// - **Forward and backward tab exits.** Focus-trapped overlays wrap
    ///   on `Tab` / `Shift+Tab`; a button at each boundary keeps dismiss
    ///   reachable in one keystroke regardless of direction.
    /// - **Reading-order proximity for screen readers.** The start button
    ///   is announced immediately when focus enters the overlay; the end
    ///   button is the next stop after the user has traversed the content
    ///   linearly so they do not have to navigate back to find a dismiss
    ///   control.
    /// - **Rotor / element-list discovery.** Buttons-list rotors
    ///   (`VoiceOver`, `NVDA`, `JAWS`) surface both instances, so users
    ///   can pick whichever is closest to current focus.
    ///
    /// Sighted users never see either button — [`dismiss_button_attrs`]
    /// sets `data-ars-visually-hidden`, so the duplication is strictly an
    /// assistive-technology concern with no visual cost. See
    /// `spec/components/utility/dismissable.md` §3 for the canonical
    /// rationale.
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
    /// Called when the user interacts outside the dismissable element,
    /// **before** the final dismiss decision is finalized.
    ///
    /// The adapter invokes this on `pointerdown` outside, or `focusin` on
    /// an element outside. The callback receives a
    /// [`DismissAttempt<InteractOutsideEvent>`] whose
    /// [`prevent_dismiss`](DismissAttempt::prevent_dismiss) method may be
    /// called to veto the upcoming `on_dismiss` invocation.
    pub on_interact_outside:
        Option<Callback<dyn Fn(DismissAttempt<InteractOutsideEvent>) + Send + Sync>>,

    /// Called when the user presses the Escape key while the dismissable is
    /// the topmost overlay, **before** the final dismiss decision is
    /// finalized.
    ///
    /// The callback receives a [`DismissAttempt<()>`] whose
    /// [`prevent_dismiss`](DismissAttempt::prevent_dismiss) method may be
    /// called to veto the upcoming `on_dismiss` invocation.
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,

    /// Called after the dismiss decision is finalized — observational only,
    /// **not** cancelable. The callback receives a [`DismissReason`]
    /// identifying which path triggered the dismissal.
    pub on_dismiss: Option<Callback<dyn Fn(DismissReason) + Send + Sync>>,

    /// When true, outside pointer events are intercepted and prevented from
    /// reaching underlying elements (transparent overlay with
    /// `pointer-events: auto`). Default: `false`.
    pub disable_outside_pointer_events: bool,

    /// DOM IDs of elements that should NOT trigger an outside interaction
    /// when clicked. Typically includes the trigger button that opened the
    /// overlay.
    ///
    /// **IDs are mandatory for participation.** Adapter containment walks the
    /// DOM ancestor chain comparing each node's `id` attribute (and
    /// `data-ars-portal-owner` for portaled subtrees). Elements without an
    /// `id` cannot be matched against `exclude_ids` or the adapter's
    /// reactive `inside_boundaries` set — wrappers that need to register a
    /// node as an inside-boundary must ensure it has an `id`.
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

impl Props {
    /// Returns a fresh [`Props`] with every field at its
    /// [`Default`] value — no callbacks registered, pointer events not
    /// blocked, and no excluded ids.
    ///
    /// This is the documented entry point for the builder chain. Use
    /// chained setters ([`on_dismiss`](Self::on_dismiss),
    /// [`exclude_ids`](Self::exclude_ids), …) to populate behavioural
    /// configuration without the `Some(Callback::new(_))` and
    /// `..Props::default()` ceremony struct-literal construction
    /// requires.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the pre-dismiss callback fired on outside pointer or
    /// focus interactions.
    ///
    /// Wraps the supplied closure in [`Some(Callback::new(_))`](Callback::new)
    /// and stores it in [`on_interact_outside`](Self::on_interact_outside).
    /// The callback receives a [`DismissAttempt<InteractOutsideEvent>`] whose
    /// [`prevent_dismiss`](DismissAttempt::prevent_dismiss) method may be
    /// invoked to veto the upcoming dismissal.
    #[must_use]
    pub fn on_interact_outside<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<InteractOutsideEvent>) + Send + Sync + 'static,
    {
        self.on_interact_outside = Some(Callback::new(f));
        self
    }

    /// Registers the pre-dismiss callback fired on Escape while topmost.
    ///
    /// Wraps the supplied closure in [`Some(Callback::new(_))`](Callback::new)
    /// and stores it in [`on_escape_key_down`](Self::on_escape_key_down).
    /// The callback receives a [`DismissAttempt<()>`] whose
    /// [`prevent_dismiss`](DismissAttempt::prevent_dismiss) method may be
    /// invoked to veto the upcoming dismissal.
    #[must_use]
    pub fn on_escape_key_down<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_escape_key_down = Some(Callback::new(f));
        self
    }

    /// Registers the post-decision dismiss callback.
    ///
    /// Wraps the supplied closure in [`Some(Callback::new(_))`](Callback::new)
    /// and stores it in [`on_dismiss`](Self::on_dismiss). The callback
    /// receives a [`DismissReason`] identifying which path triggered the
    /// dismissal and is observational only — it cannot be vetoed.
    #[must_use]
    pub fn on_dismiss<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissReason) + Send + Sync + 'static,
    {
        self.on_dismiss = Some(Callback::new(f));
        self
    }

    /// Sets [`disable_outside_pointer_events`](Self::disable_outside_pointer_events)
    /// to the supplied value.
    ///
    /// When `true`, outside pointer events are intercepted and prevented
    /// from reaching underlying elements (transparent overlay with
    /// `pointer-events: auto`).
    #[must_use]
    pub const fn disable_outside_pointer_events(mut self, value: bool) -> Self {
        self.disable_outside_pointer_events = value;
        self
    }

    /// Replaces [`exclude_ids`](Self::exclude_ids) with the supplied iterator
    /// of DOM ids that must NOT trigger an outside-interaction dismissal
    /// when clicked or focused.
    ///
    /// Each item is converted into [`String`] via [`Into`], so callers can
    /// pass an array of `&str`, `String`, `Cow<str>`, or any other
    /// `Into<String>` type.
    #[must_use]
    pub fn exclude_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.exclude_ids = ids.into_iter().map(Into::into).collect();
        self
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
                Callback::new(move |_: DismissAttempt<InteractOutsideEvent>| {
                    interact_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_escape_key_down: Some({
                let escape_calls = Arc::clone(&escape_calls);
                Callback::new(move |_: DismissAttempt<()>| {
                    escape_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_dismiss: Some({
                let dismiss_calls = Arc::clone(&dismiss_calls);
                Callback::new(move |_: DismissReason| {
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

        props.on_interact_outside.as_ref().expect("callback")(DismissAttempt::new(
            InteractOutsideEvent::EscapeKey,
        ));
        props.on_escape_key_down.as_ref().expect("callback")(DismissAttempt::new(()));
        props.on_dismiss.as_ref().expect("callback")(DismissReason::Escape);

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
            Callback::new(move |_: DismissAttempt<InteractOutsideEvent>| {
                calls.fetch_add(1, Ordering::SeqCst);
            })
        };

        let props = Props {
            on_interact_outside: Some(cb.clone()),
            ..Props::default()
        };

        let cloned = props.clone();

        assert_eq!(props.on_interact_outside, cloned.on_interact_outside);

        cloned.on_interact_outside.as_ref().expect("callback")(DismissAttempt::new(
            InteractOutsideEvent::EscapeKey,
        ));

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn props_partial_eq_uses_callback_pointer_identity() {
        let shared_calls = Arc::new(AtomicUsize::new(0));

        let cb = {
            let shared_calls = Arc::clone(&shared_calls);
            Callback::new(move |_: DismissReason| {
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

        props2.on_dismiss.as_ref().expect("callback")(DismissReason::DismissButton);

        assert_eq!(shared_calls.load(Ordering::SeqCst), 1);

        let different_calls = Arc::new(AtomicUsize::new(0));

        let props3 = Props {
            on_dismiss: Some({
                let different_calls = Arc::clone(&different_calls);
                Callback::new(move |_: DismissReason| {
                    different_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..Props::default()
        };

        assert_ne!(props1, props3);

        props3.on_dismiss.as_ref().expect("callback")(DismissReason::DismissButton);

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

    // ── DismissReason / DismissAttempt tests ───────────────────────

    #[test]
    fn dismiss_reason_variants_are_distinct() {
        assert_ne!(DismissReason::OutsidePointer, DismissReason::OutsideFocus);
        assert_ne!(DismissReason::Escape, DismissReason::DismissButton);
        assert_eq!(DismissReason::Escape, DismissReason::Escape);
    }

    #[test]
    fn dismiss_attempt_starts_un_prevented() {
        let attempt = DismissAttempt::new(InteractOutsideEvent::EscapeKey);

        assert!(!attempt.is_prevented());
    }

    #[test]
    fn dismiss_attempt_prevent_dismiss_sets_flag() {
        let attempt = DismissAttempt::new(InteractOutsideEvent::EscapeKey);

        attempt.prevent_dismiss();

        assert!(attempt.is_prevented());
    }

    #[test]
    fn dismiss_attempt_prevent_dismiss_is_idempotent() {
        let attempt = DismissAttempt::new(());

        attempt.prevent_dismiss();
        attempt.prevent_dismiss();
        attempt.prevent_dismiss();

        assert!(attempt.is_prevented());
    }

    #[test]
    fn dismiss_attempt_clone_shares_veto_flag() {
        let original = DismissAttempt::new(InteractOutsideEvent::EscapeKey);
        let cloned = original.clone();

        assert!(!original.is_prevented());
        assert!(!cloned.is_prevented());

        cloned.prevent_dismiss();

        assert!(
            original.is_prevented(),
            "veto from a clone must be visible through the original",
        );
    }

    #[test]
    fn dismiss_attempt_debug_includes_event_and_prevented_flag() {
        let attempt = DismissAttempt::new(InteractOutsideEvent::EscapeKey);
        let before = format!("{attempt:?}");

        assert!(before.contains("DismissAttempt"));
        assert!(before.contains("EscapeKey"));
        assert!(before.contains("prevented: false"));

        attempt.prevent_dismiss();

        let after = format!("{attempt:?}");

        assert!(after.contains("prevented: true"));
    }

    #[test]
    fn dismiss_attempt_partial_eq_requires_same_veto_arc() {
        let original = DismissAttempt::new(InteractOutsideEvent::EscapeKey);
        let same = original.clone();
        let independent = DismissAttempt::new(InteractOutsideEvent::EscapeKey);

        assert_eq!(original, same);
        assert_ne!(original, independent);
    }

    // ── Builder tests ──────────────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .on_interact_outside(|_attempt: DismissAttempt<InteractOutsideEvent>| {})
            .on_escape_key_down(|_attempt: DismissAttempt<()>| {})
            .on_dismiss(|_reason: DismissReason| {})
            .disable_outside_pointer_events(true)
            .exclude_ids(["trigger", "panel"]);

        assert!(props.on_interact_outside.is_some());
        assert!(props.on_escape_key_down.is_some());
        assert!(props.on_dismiss.is_some());
        assert!(props.disable_outside_pointer_events);
        assert_eq!(props.exclude_ids, vec!["trigger", "panel"]);
    }

    #[test]
    fn props_builder_on_dismiss_setter_invokes_supplied_closure() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_props = Arc::clone(&calls);

        let props = Props::new().on_dismiss(move |reason: DismissReason| {
            assert_eq!(reason, DismissReason::DismissButton);
            calls_for_props.fetch_add(1, Ordering::SeqCst);
        });

        props.on_dismiss.as_ref().expect("callback")(DismissReason::DismissButton);

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
