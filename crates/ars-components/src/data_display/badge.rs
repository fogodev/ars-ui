//! Badge component connect API.
//!
//! `Badge` is a stateless, framework-agnostic attribute mapper for inline
//! status, count, category, and lifecycle labels.

use alloc::{format, string::String};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, MessageFn,
};
use ars_i18n::{Locale, number};

type OverflowLabelFn = dyn Fn(u64, &Locale) -> String + Send + Sync;
type BadgeLabelFn = dyn Fn(u64, &str, &Locale) -> String + Send + Sync;

/// Props for the Badge component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Visual style variant.
    pub variant: Variant,

    /// Visual size token.
    pub size: Size,

    /// Assistive-technology exposure mode.
    pub accessibility: Accessibility,

    /// Accessible label describing the badge content when visible text is not
    /// sufficient on its own.
    pub aria_label: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            variant: Variant::Subtle,
            size: Size::Sm,
            accessibility: Accessibility::Static,
            aria_label: None,
        }
    }
}

impl Props {
    /// Returns fresh badge props with the documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the visual style variant.
    #[must_use]
    pub const fn variant(mut self, value: Variant) -> Self {
        self.variant = value;
        self
    }

    /// Sets the visual size token.
    #[must_use]
    pub const fn size(mut self, value: Size) -> Self {
        self.size = value;
        self
    }

    /// Sets how assistive technology should perceive the badge.
    #[must_use]
    pub const fn accessibility(mut self, value: Accessibility) -> Self {
        self.accessibility = value;
        self
    }

    /// Sets the accessible label for the badge.
    #[must_use]
    pub fn aria_label(mut self, value: impl Into<String>) -> Self {
        self.aria_label = Some(value.into());
        self
    }
}

/// How the badge is exposed to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Accessibility {
    /// Static visible content with an optional `aria-label`.
    Static,

    /// Visual-only badge hidden from assistive technology.
    Decorative,

    /// Polite live-region badge for dynamic, non-urgent updates.
    Status,

    /// Assertive alert badge for urgent status changes.
    Alert,
}

impl Accessibility {
    /// Returns whether this mode hides the badge from assistive technology.
    #[must_use]
    pub const fn is_decorative(self) -> bool {
        matches!(self, Self::Decorative)
    }
}

/// Visual style variant of the badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Filled badge with the strongest emphasis.
    Solid,

    /// Soft tinted badge with low emphasis.
    Soft,

    /// Subtle tinted badge with low emphasis.
    Subtle,

    /// Badge with a visible surface/background treatment.
    Surface,

    /// Outlined badge with transparent or minimal fill.
    Outline,

    /// Plain text-like badge with minimal chrome.
    Plain,
}

impl Variant {
    /// Returns the `data-ars-variant` value for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Soft => "soft",
            Self::Subtle => "subtle",
            Self::Surface => "surface",
            Self::Outline => "outline",
            Self::Plain => "plain",
        }
    }
}

/// Visual size token of the badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    /// Extra-small badge size.
    Xs,

    /// Small badge size.
    Sm,

    /// Medium badge size.
    Md,

    /// Large badge size.
    Lg,

    /// Extra-large badge size.
    Xl,
}

impl Size {
    /// Returns the `data-ars-size` value for this size.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
            Self::Xl => "xl",
        }
    }
}

/// Messages for the Badge component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the overflow label, receiving the count and current locale.
    pub overflow_label: MessageFn<OverflowLabelFn>,

    /// Returns the badge's accessible label using the count, semantic category,
    /// and current locale.
    pub badge_label: MessageFn<BadgeLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            overflow_label: MessageFn::new(|count: u64, locale: &Locale| {
                format_count(count, locale)
            }),
            badge_label: MessageFn::new(
                |count: u64, category: &str, _locale: &Locale| match count {
                    0 => format!("No unread {category}s"),
                    1 => format!("1 unread {category}"),
                    n => format!("{n} unread {category}s"),
                },
            ),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the Badge connect API.
#[derive(ComponentPart)]
#[scope = "badge"]
pub enum Part {
    /// The root inline badge element.
    Root,
}

/// API for the Badge component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("badge::Api")
            .field("props", &self.props)
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Api {
    /// Creates a new API for the badge.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns the overflow label for the given count, such as `"99+"`.
    #[must_use]
    pub fn overflow_label(&self, count: u64) -> String {
        (self.messages.overflow_label)(count, &self.locale)
    }

    /// Returns the badge's accessible label for a count and category.
    #[must_use]
    pub fn badge_label(&self, count: u64, category: &str) -> String {
        (self.messages.badge_label)(count, category, &self.locale)
    }

    /// Returns root attributes for the badge.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str())
            .set(HtmlAttr::Data("ars-size"), self.props.size.as_str());

        match self.props.accessibility {
            Accessibility::Decorative => {
                attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
                return attrs;
            }

            Accessibility::Alert => {
                attrs.set(HtmlAttr::Role, "alert");
            }

            Accessibility::Status => {
                attrs
                    .set(HtmlAttr::Role, "status")
                    .set(HtmlAttr::Aria(AriaAttr::Live), "polite");
            }

            Accessibility::Static => {}
        }

        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }

        attrs
    }

    /// Returns the badge accessibility mode.
    #[must_use]
    pub const fn accessibility(&self) -> Accessibility {
        self.props.accessibility
    }

    /// Returns the badge aria-label override, when present.
    #[must_use]
    pub fn aria_label(&self) -> Option<&str> {
        self.props.aria_label.as_deref()
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

/// Formats a count value as locale-aware text with a maximum visible value of
/// `99+`.
#[must_use]
pub fn format_count(value: u64, locale: &Locale) -> String {
    let formatter = number::Formatter::new(locale, number::FormatOptions::default());

    if value > 99 {
        format!("{}+", formatter.format(99.0))
    } else {
        formatter.format(value as f64)
    }
}

#[cfg(test)]
mod tests {
    use ars_core::ConnectApi;
    use insta::assert_snapshot;

    use super::*;

    fn api(props: Props) -> Api {
        Api::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("badge-1")
            .variant(Variant::Outline)
            .size(Size::Xl)
            .accessibility(Accessibility::Alert)
            .aria_label("3 unread messages");

        assert_eq!(props.id, "badge-1");
        assert_eq!(props.variant, Variant::Outline);
        assert_eq!(props.size, Size::Xl);
        assert_eq!(props.accessibility, Accessibility::Alert);
        assert_eq!(props.aria_label.as_deref(), Some("3 unread messages"));
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn accessibility_reports_decorative_mode() {
        assert!(Accessibility::Decorative.is_decorative());
        assert!(!Accessibility::Static.is_decorative());
        assert!(!Accessibility::Status.is_decorative());
        assert!(!Accessibility::Alert.is_decorative());
    }

    #[test]
    fn badge_root_default_snapshot() {
        assert_snapshot!(
            "badge_root_default",
            snapshot_attrs(&api(Props::new()).root_attrs())
        );
    }

    #[test]
    fn badge_root_status_snapshot() {
        assert_snapshot!(
            "badge_root_status",
            snapshot_attrs(
                &api(Props::new()
                    .accessibility(Accessibility::Status)
                    .aria_label("3 unread notifications"))
                .root_attrs()
            )
        );
    }

    #[test]
    fn badge_root_alert_snapshot() {
        assert_snapshot!(
            "badge_root_alert",
            snapshot_attrs(
                &api(Props::new()
                    .accessibility(Accessibility::Alert)
                    .aria_label("Payment failed"))
                .root_attrs()
            )
        );
    }

    #[test]
    fn badge_root_decorative_snapshot() {
        assert_snapshot!(
            "badge_root_decorative",
            snapshot_attrs(
                &api(Props::new()
                    .accessibility(Accessibility::Decorative)
                    .aria_label("Hidden"))
                .root_attrs()
            )
        );
    }

    #[test]
    fn every_variant_emits_expected_data_attr() {
        let cases = [
            (Variant::Solid, "solid"),
            (Variant::Soft, "soft"),
            (Variant::Subtle, "subtle"),
            (Variant::Surface, "surface"),
            (Variant::Outline, "outline"),
            (Variant::Plain, "plain"),
        ];

        for (variant, expected) in cases {
            assert_eq!(variant.as_str(), expected);

            let attrs = api(Props::new().variant(variant)).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-variant")), Some(expected));
        }
    }

    #[test]
    fn every_size_emits_expected_data_attr() {
        let cases = [
            (Size::Xs, "xs"),
            (Size::Sm, "sm"),
            (Size::Md, "md"),
            (Size::Lg, "lg"),
            (Size::Xl, "xl"),
        ];

        for (size, expected) in cases {
            assert_eq!(size.as_str(), expected);

            let attrs = api(Props::new().size(size)).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-size")), Some(expected));
        }
    }

    #[test]
    fn status_attrs_set_status_live_and_label() {
        let attrs = api(Props::new()
            .accessibility(Accessibility::Status)
            .aria_label("3 unread messages"))
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("3 unread messages")
        );
    }

    #[test]
    fn static_attrs_set_only_label_when_provided() {
        let attrs = api(Props::new().aria_label("Beta feature")).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Beta feature")
        );
        assert!(!attrs.contains(&HtmlAttr::Role));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Live)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
    }

    #[test]
    fn alert_attrs_set_alert_and_label() {
        let attrs = api(Props::new()
            .accessibility(Accessibility::Alert)
            .aria_label("Payment failed"))
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Payment failed")
        );
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Live)));
    }

    #[test]
    fn decorative_attrs_set_aria_hidden_and_suppress_all_a11y_semantics() {
        let attrs = api(Props::new()
            .accessibility(Accessibility::Decorative)
            .aria_label("Hidden"))
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Role));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Live)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
    }

    #[test]
    fn api_exposes_accessibility_and_label() {
        let api = api(Props::new()
            .accessibility(Accessibility::Status)
            .aria_label("3 unread messages"));

        assert_eq!(api.accessibility(), Accessibility::Status);
        assert_eq!(api.aria_label(), Some("3 unread messages"));
    }

    #[test]
    fn api_debug_is_stable_and_redacts_messages() {
        let api = api(Props::new()
            .id("badge-1")
            .accessibility(Accessibility::Status));

        let debug = format!("{api:?}");

        assert!(debug.contains("badge::Api"));
        assert!(debug.contains("badge-1"));
        assert!(debug.contains("Status"));
        assert!(debug.contains("locale"));
        assert!(!debug.contains("messages"));
    }

    #[test]
    fn part_attrs_delegates_to_root_attrs() {
        let api = api(Props::new().variant(Variant::Solid));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn message_helpers_return_overflow_and_accessible_labels() {
        let messages = Messages {
            overflow_label: MessageFn::new(|count: u64, _locale: &Locale| {
                format!("more than {count}")
            }),
            badge_label: MessageFn::new(|count: u64, category: &str, _locale: &Locale| {
                format!("{count} {category} ready")
            }),
        };

        let api = Api::new(Props::new(), &Env::default(), &messages);

        assert_eq!(api.overflow_label(99), "more than 99");
        assert_eq!(api.badge_label(3, "notification"), "3 notification ready");
    }

    #[test]
    fn default_badge_label_handles_zero_one_and_plural_counts() {
        let api = api(Props::new());

        assert_eq!(
            api.badge_label(0, "notification"),
            "No unread notifications"
        );
        assert_eq!(api.badge_label(1, "notification"), "1 unread notification");
        assert_eq!(api.badge_label(2, "notification"), "2 unread notifications");
    }

    #[test]
    fn default_overflow_label_uses_capped_count_formatting() {
        let api = api(Props::new());

        assert_eq!(api.overflow_label(100), "99+");
    }

    #[test]
    fn format_count_caps_at_99_plus() {
        let locale = Env::default().locale;

        assert_eq!(format_count(0, &locale), "0");
        assert_eq!(format_count(99, &locale), "99");
        assert_eq!(format_count(100, &locale), "99+");
    }
}
