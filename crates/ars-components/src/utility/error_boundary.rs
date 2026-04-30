//! Error boundary fallback structure and shared message bundle.
//!
//! This module owns the framework-agnostic side of the error-boundary
//! component: the localizable [`Messages`] bundle, the canonical [`Part`]
//! taxonomy that drives `data-ars-scope` / `data-ars-part` data attributes,
//! and the [`Api`] / attr helpers that build the accessible fallback
//! container's attributes.
//!
//! The adapter-side wrappers (`ars_dioxus::error_boundary::Boundary`,
//! `ars_leptos::error_boundary::Boundary`) compose around these helpers so
//! both adapters emit the **same** HTML structure for the default
//! fallback — a `<div role="alert" data-ars-error="true">` with a message
//! paragraph and a `<ul>` of `<li>` error entries — regardless of whether
//! the underlying framework primitive surfaces one error (Dioxus
//! [`ErrorContext`](https://docs.rs/dioxus-core/0.7/dioxus_core/struct.ErrorContext.html))
//! or many ([`Errors`](https://docs.rs/leptos/0.8/leptos/error/struct.Errors.html)).
//!
//! This module intentionally stays free of DOM or framework types so that
//! attribute generation can be unit-tested with pure assertions.
//!
//! See `spec/components/utility/error-boundary.md` for the canonical
//! specification.

use alloc::string::String;

use ars_core::{
    AriaAttr, AttrMap, AttrValue, ComponentMessages, ComponentPart, ConnectApi, HtmlAttr, MessageFn,
};
use ars_i18n::Locale;

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings rendered inside the default fallback UI.
///
/// The bundle currently contains only the static heading shown above the
/// list of caught errors. Override per-app by registering a custom bundle
/// with the surrounding `ArsProvider`'s `i18n_registries`, the same way
/// `Dismissable::Messages` are overridden — see
/// `spec/components/utility/error-boundary.md` §6 "Internationalization".
///
/// # Examples
///
/// ```
/// use ars_components::utility::error_boundary::Messages;
/// use ars_i18n::Locale;
///
/// let messages = Messages::default();
///
/// let locale = Locale::parse("en-US").expect("locale");
///
/// assert_eq!((messages.message)(&locale), "A component encountered an error.");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Static heading rendered above the `<ul>` of error entries. Defaults
    /// to `"A component encountered an error."`.
    pub message: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            message: MessageFn::static_str("A component encountered an error."),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// DOM parts of the error-boundary fallback.
///
/// Each part contributes a `data-ars-scope="error-boundary"` and
/// `data-ars-part="…"` pair that adapters can target for styling, tests,
/// or accessibility tree walks.
#[derive(ComponentPart)]
#[scope = "error-boundary"]
pub enum Part {
    /// The accessible alert container (`<div role="alert">`).
    Root,

    /// The static heading paragraph rendered above the list of errors.
    Message,

    /// The `<ul>` listing every captured error.
    List,

    /// An individual `<li>` entry for one captured error's `Display` text.
    Item,
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Stateless connect API for deriving the canonical fallback container
/// attributes.
///
/// `Api::new(error_count)` produces a [`ConnectApi`] whose
/// [`Part::Root`] attrs contain the accessibility primitives
/// (`role="alert"`, `aria-live="assertive"`, `aria-atomic="true"`) plus
/// the `data-ars-error="true"` test selector and a numeric
/// `data-ars-error-count` so consumers can detect the multi-error case
/// from a CSS selector alone.
#[derive(Clone, Debug)]
pub struct Api {
    error_count: usize,
}

impl Api {
    /// Creates a new fallback API for the given number of caught errors.
    ///
    /// The count is rendered into the root element's
    /// `data-ars-error-count` attribute so styling and tests can branch on
    /// "single vs many errors" without parsing the inner `<ul>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::utility::error_boundary::Api;
    /// use ars_core::HtmlAttr;
    ///
    /// let api = Api::new(3);
    ///
    /// let attrs = api.root_attrs();
    ///
    /// assert_eq!(api.error_count(), 3);
    /// assert_eq!(
    ///     attrs.get(&HtmlAttr::Data("ars-error-count")),
    ///     Some("3"),
    /// );
    /// assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
    /// ```
    #[must_use]
    pub const fn new(error_count: usize) -> Self {
        Self { error_count }
    }

    /// Returns root-container attributes for the fallback `<div>`.
    ///
    /// Adapters merge these with the inner content (message paragraph and
    /// error list) to render the canonical accessible fallback.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "alert")
            .set(HtmlAttr::Aria(AriaAttr::Live), "assertive")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true")
            // Emit `data-ars-error="true"` literally (string value, not the
            // HTML5 boolean-attribute empty-string form) to match the spec
            // contract and the original issue acceptance criteria. Tests can
            // assert on `data-ars-error="true"` without falling back to
            // attribute-presence-only checks.
            .set(HtmlAttr::Data("ars-error"), "true")
            .set(
                HtmlAttr::Data("ars-error-count"),
                AttrValue::from(self.error_count.to_string()),
            );

        attrs
    }

    /// Returns attributes for the static heading paragraph (`<p>`).
    #[must_use]
    pub fn message_attrs(&self) -> AttrMap {
        message_attrs()
    }

    /// Returns attributes for the `<ul>` enclosing every error entry.
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        list_attrs()
    }

    /// Returns attributes for an individual `<li>` error entry.
    #[must_use]
    pub fn item_attrs(&self) -> AttrMap {
        item_attrs()
    }

    /// Returns the number of errors this fallback represents.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.error_count
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Message => self.message_attrs(),
            Part::List => self.list_attrs(),
            Part::Item => self.item_attrs(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Free attr helpers
// ────────────────────────────────────────────────────────────────────

/// Returns scope/part attrs for the static heading paragraph.
#[must_use]
pub fn message_attrs() -> AttrMap {
    part_attrs(&Part::Message)
}

/// Returns scope/part attrs for the `<ul>` enclosing the error list.
#[must_use]
pub fn list_attrs() -> AttrMap {
    part_attrs(&Part::List)
}

/// Returns scope/part attrs for one `<li>` error entry.
#[must_use]
pub fn item_attrs() -> AttrMap {
    part_attrs(&Part::Item)
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString};

    use super::*;

    #[test]
    fn root_attrs_emit_role_alert_aria_live_and_count() {
        let api = Api::new(3);

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("assertive")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-error")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-error-count")), Some("3"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("error-boundary")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn root_attrs_with_zero_errors_still_emit_count() {
        // The boundary may be invoked with no errors during the brief window
        // between `clear_errors()` and the next render; the attribute must
        // still exist so tests can assert on its presence.
        let api = Api::new(0);

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-error-count")), Some("0"));
    }

    #[test]
    fn message_list_item_attrs_share_scope_and_carry_their_part() {
        for (helper, expected_part) in [
            (message_attrs(), "message"),
            (list_attrs(), "list"),
            (item_attrs(), "item"),
        ] {
            assert_eq!(
                helper.get(&HtmlAttr::Data("ars-scope")),
                Some("error-boundary"),
                "expected scope for part {expected_part}"
            );
            assert_eq!(helper.get(&HtmlAttr::Data("ars-part")), Some(expected_part));
        }
    }

    #[test]
    fn connect_api_dispatches_to_each_part() {
        let api = Api::new(1);

        for (part, expected_part) in [
            (Part::Root, "root"),
            (Part::Message, "message"),
            (Part::List, "list"),
            (Part::Item, "item"),
        ] {
            let attrs = api.part_attrs(part);

            assert_eq!(
                attrs.get(&HtmlAttr::Data("ars-part")),
                Some(expected_part),
                "wrong part attr for {expected_part}"
            );
        }
    }

    #[test]
    fn default_messages_uses_canonical_english_string() {
        let messages = Messages::default();

        let locale = Locale::parse("en-US").expect("locale should parse");

        let resolved = (messages.message)(&locale);

        assert_eq!(resolved, "A component encountered an error.");
    }

    #[test]
    fn error_count_round_trips() {
        let api = Api::new(42);

        assert_eq!(api.error_count(), 42);

        // And `Display` round-trips via the attr.
        assert_eq!(
            api.root_attrs()
                .get(&HtmlAttr::Data("ars-error-count"))
                .map(ToString::to_string),
            Some(format!("{}", 42))
        );
    }

    // ── Snapshots ──────────────────────────────────────────────────
    //
    // Per `spec/testing/03-snapshot-tests.md` §1.3 "Multi-Part Anatomy
    // Snapshot Rule": every component MUST have snapshot tests for each
    // anatomy part that produces ARIA attributes. Error boundary's four
    // parts (Root, Message, List, Item) each get one snapshot, plus
    // `Root` is exercised at three error-count cardinalities (0, 1, 3)
    // because the count drives `data-ars-error-count` and is the only
    // input that affects Root's attr output.
    //
    // The snapshots catch silent regressions in attribute *order* /
    // *value* / *presence* that the per-attribute `assert_eq!` tests
    // above would not — e.g. accidentally dropping `aria-atomic`, or
    // emitting `data-ars-error=""` (boolean form) instead of `="true"`.

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn error_boundary_root_no_errors_snapshot() {
        insta::assert_snapshot!(
            "error_boundary_root_no_errors",
            snapshot_attrs(&Api::new(0).root_attrs())
        );
    }

    #[test]
    fn error_boundary_root_single_error_snapshot() {
        insta::assert_snapshot!(
            "error_boundary_root_single_error",
            snapshot_attrs(&Api::new(1).root_attrs())
        );
    }

    #[test]
    fn error_boundary_root_multi_error_snapshot() {
        insta::assert_snapshot!(
            "error_boundary_root_multi_error",
            snapshot_attrs(&Api::new(3).root_attrs())
        );
    }

    #[test]
    fn error_boundary_message_snapshot() {
        insta::assert_snapshot!("error_boundary_message", snapshot_attrs(&message_attrs()));
    }

    #[test]
    fn error_boundary_list_snapshot() {
        insta::assert_snapshot!("error_boundary_list", snapshot_attrs(&list_attrs()));
    }

    #[test]
    fn error_boundary_item_snapshot() {
        insta::assert_snapshot!("error_boundary_item", snapshot_attrs(&item_attrs()));
    }
}
