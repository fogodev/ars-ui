//! Skeleton component connect API.
//!
//! `Skeleton` is a stateless, framework-agnostic attribute mapper for loading
//! placeholder shapes.

use alloc::string::{String, ToString as _};
use core::{
    fmt::{self, Debug},
    num::NonZero,
    ops::Range,
};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Env, HtmlAttr,
    MessageFn,
};
use ars_i18n::Locale;

/// Props for the Skeleton component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Number of skeleton item placeholders to render.
    pub count: NonZero<u32>,

    /// Animation variant.
    pub variant: Variant,

    /// Shape variant for repeated item placeholders.
    pub shape: Shape,

    /// Height of each skeleton item in CSS units.
    pub line_height: String,

    /// Gap between skeleton items in CSS units.
    pub gap: String,

    /// Size of the optional leading circle in CSS units.
    pub leading_circle_size: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            count: NonZero::<u32>::MIN,
            variant: Variant::Pulse,
            shape: Shape::Text,
            line_height: "1rem".into(),
            gap: "0.5rem".into(),
            leading_circle_size: None,
        }
    }
}

impl Props {
    /// Returns fresh skeleton props with the documented defaults.
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

    /// Sets the number of skeleton item placeholders to render.
    #[must_use]
    pub const fn count(mut self, count: NonZero<u32>) -> Self {
        self.count = count;
        self
    }

    /// Sets the animation variant.
    #[must_use]
    pub const fn variant(mut self, value: Variant) -> Self {
        self.variant = value;
        self
    }

    /// Sets the shape variant for repeated item placeholders.
    #[must_use]
    pub const fn shape(mut self, value: Shape) -> Self {
        self.shape = value;
        self
    }

    /// Sets the skeleton item height CSS value.
    #[must_use]
    pub fn line_height(mut self, value: impl Into<String>) -> Self {
        self.line_height = value.into();
        self
    }

    /// Sets the gap CSS value.
    #[must_use]
    pub fn gap(mut self, value: impl Into<String>) -> Self {
        self.gap = value.into();
        self
    }

    /// Sets the optional leading circle size CSS value.
    #[must_use]
    pub fn leading_circle_size(mut self, value: impl Into<String>) -> Self {
        self.leading_circle_size = Some(value.into());
        self
    }
}

/// Animation variant for skeleton loading placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Opacity fades in and out.
    Pulse,

    /// Left-to-right sweep highlight.
    Wave,

    /// Diagonal gradient shimmer.
    Shimmer,

    /// Highlight shine animation.
    Shine,

    /// Static placeholder without animation.
    None,
}

impl Variant {
    /// Returns the `data-ars-variant` value for this animation variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pulse => "pulse",
            Self::Wave => "wave",
            Self::Shimmer => "shimmer",
            Self::Shine => "shine",
            Self::None => "none",
        }
    }

    /// Returns whether this variant has an animation by default.
    #[must_use]
    pub const fn is_animated(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Shape variant for skeleton item placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// Text-line placeholder.
    Text,

    /// Circular placeholder.
    Circle,

    /// Rectangular placeholder.
    Rect,
}

impl Shape {
    /// Returns the `data-ars-shape` value for this shape.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Circle => "circle",
            Self::Rect => "rect",
        }
    }
}

/// Messages for the Skeleton component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the localized loading label.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the Skeleton connect API.
#[derive(ComponentPart)]
#[scope = "skeleton"]
pub enum Part {
    /// The root loading-status container.
    Root,

    /// The optional leading circle placeholder.
    Circle,

    /// A repeated skeleton item placeholder.
    Item {
        /// Zero-based item index.
        index: usize,
    },
}

/// API for the Skeleton component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("skeleton::Api")
            .field("props", &self.props)
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Api {
    /// Creates a new API for the skeleton.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns the number of repeated item placeholders.
    #[must_use]
    pub const fn count(&self) -> NonZero<u32> {
        self.props.count
    }

    /// Returns the repeated item shape.
    #[must_use]
    pub const fn shape(&self) -> Shape {
        self.props.shape
    }

    /// Returns the animation variant.
    #[must_use]
    pub const fn variant(&self) -> Variant {
        self.props.variant
    }

    /// Returns the zero-based item indices adapters should render.
    #[must_use]
    pub const fn item_indices(&self) -> Range<usize> {
        0..self.props.count.get() as usize
    }

    /// Returns whether a leading circle placeholder should be rendered.
    #[must_use]
    pub const fn has_leading_circle(&self) -> bool {
        self.props.leading_circle_size.is_some()
    }

    /// Returns root container attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "status")
            .set(HtmlAttr::Aria(AriaAttr::Busy), "true")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.messages.loading_label)(&self.locale),
            )
            .set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str())
            .set(HtmlAttr::Data("ars-shape"), self.props.shape.as_str())
            .set_style(
                CssProperty::Custom("ars-skeleton-line-height"),
                self.props.line_height.as_str(),
            )
            .set_style(
                CssProperty::Custom("ars-skeleton-gap"),
                self.props.gap.as_str(),
            );

        if self.props.variant.is_animated() {
            attrs.set_bool(HtmlAttr::Data("ars-animated"), true);
        }

        attrs
    }

    /// Returns attributes for the optional leading circle element.
    #[must_use]
    pub fn circle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Circle.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(size) = &self.props.leading_circle_size {
            attrs.set_style(
                CssProperty::Custom("ars-skeleton-circle-size"),
                size.as_str(),
            );
        }

        attrs
    }

    /// Returns attributes for a skeleton item element.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Circle => self.circle_attrs(),
            Part::Item { index } => self.item_attrs(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, sync::Arc, vec};

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
        let count = NonZero::new(3).expect("3 is non-zero");

        let props = Props::new()
            .id("skeleton-1")
            .count(count)
            .variant(Variant::Shine)
            .shape(Shape::Circle)
            .line_height("2rem")
            .gap("1rem")
            .leading_circle_size("3rem");

        assert_eq!(props.id, "skeleton-1");
        assert_eq!(props.count, count);
        assert_eq!(props.variant, Variant::Shine);
        assert_eq!(props.shape, Shape::Circle);
        assert_eq!(props.line_height, "2rem");
        assert_eq!(props.gap, "1rem");
        assert_eq!(props.leading_circle_size.as_deref(), Some("3rem"));
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn default_count_is_non_zero() {
        assert_eq!(Props::new().count.get(), 1);
    }

    #[test]
    fn skeleton_root_default_snapshot() {
        assert_snapshot!(
            "skeleton_root_default",
            snapshot_attrs(&api(Props::new()).root_attrs())
        );
    }

    #[test]
    fn skeleton_root_variant_none_snapshot() {
        assert_snapshot!(
            "skeleton_root_variant_none",
            snapshot_attrs(&api(Props::new().variant(Variant::None)).root_attrs())
        );
    }

    #[test]
    fn every_variant_emits_expected_data_attr() {
        let cases = [
            (Variant::Pulse, "pulse", true),
            (Variant::Wave, "wave", true),
            (Variant::Shimmer, "shimmer", true),
            (Variant::Shine, "shine", true),
            (Variant::None, "none", false),
        ];

        for (variant, expected, animated) in cases {
            assert_eq!(variant.as_str(), expected);
            assert_eq!(variant.is_animated(), animated);

            let attrs = api(Props::new().variant(variant)).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-variant")), Some(expected));
        }
    }

    #[test]
    fn every_shape_emits_expected_data_attr() {
        let cases = [
            (Shape::Text, "text"),
            (Shape::Circle, "circle"),
            (Shape::Rect, "rect"),
        ];

        for (shape, expected) in cases {
            assert_eq!(shape.as_str(), expected);

            let attrs = api(Props::new().shape(shape)).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-shape")), Some(expected));
        }
    }

    #[test]
    fn variant_none_suppresses_animation_marker() {
        let attrs = api(Props::new().variant(Variant::None)).root_attrs();

        assert!(!attrs.contains(&HtmlAttr::Data("ars-animated")));
    }

    #[test]
    fn api_exposes_render_plan_helpers() {
        let api = api(Props::new()
            .count(NonZero::new(3).expect("3 is non-zero"))
            .variant(Variant::Wave)
            .shape(Shape::Rect)
            .leading_circle_size("2rem"));

        assert_eq!(api.count().get(), 3);
        assert_eq!(api.variant(), Variant::Wave);
        assert_eq!(api.shape(), Shape::Rect);
        assert_eq!(api.item_indices().collect::<Vec<_>>(), vec![0, 1, 2]);
        assert!(api.has_leading_circle());
    }

    #[test]
    fn api_debug_is_stable_and_redacts_messages() {
        let api = api(Props::new().id("skeleton-1").variant(Variant::Wave));

        let debug = format!("{api:?}");

        assert!(debug.contains("skeleton::Api"));
        assert!(debug.contains("skeleton-1"));
        assert!(debug.contains("Wave"));
        assert!(debug.contains("locale"));
        assert!(!debug.contains("messages"));
    }

    #[test]
    fn root_attrs_include_custom_line_height_and_gap() {
        let attrs = api(Props::new().line_height("2rem").gap("0.75rem")).root_attrs();

        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-skeleton-line-height"),
            String::from("2rem")
        )));
        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-skeleton-gap"),
            String::from("0.75rem")
        )));
    }

    #[test]
    fn circle_attrs_with_and_without_size() {
        let attrs = api(Props::new()).circle_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));

        let sized = api(Props::new().leading_circle_size("2rem")).circle_attrs();

        assert_eq!(sized.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert!(sized.styles().contains(&(
            CssProperty::Custom("ars-skeleton-circle-size"),
            String::from("2rem")
        )));
    }

    #[test]
    fn skeleton_circle_attrs_snapshots() {
        assert_snapshot!(
            "skeleton_circle_default",
            snapshot_attrs(&api(Props::new()).circle_attrs())
        );
        assert_snapshot!(
            "skeleton_circle_sized",
            snapshot_attrs(&api(Props::new().leading_circle_size("2rem")).circle_attrs())
        );
    }

    #[test]
    fn item_attrs_include_index_and_aria_hidden() {
        let attrs = api(Props::new()).item_attrs(2);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-index")), Some("2"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn skeleton_item_snapshot() {
        assert_snapshot!(
            "skeleton_item_index_2",
            snapshot_attrs(&api(Props::new()).item_attrs(2))
        );
    }

    #[test]
    fn part_attrs_delegates_for_all_parts() {
        let api = api(Props::new().leading_circle_size("2rem"));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Circle), api.circle_attrs());
        assert_eq!(api.part_attrs(Part::Item { index: 2 }), api.item_attrs(2));
    }

    #[test]
    fn loading_label_message_can_be_overridden() {
        let messages = Messages {
            loading_label: MessageFn::new(Arc::new(|_locale: &Locale| String::from("Cargando"))
                as Arc<dyn Fn(&Locale) -> String + Send + Sync>),
        };

        let api = Api::new(Props::new(), &Env::default(), &messages);

        let attrs = api.root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Cargando")
        );
    }
}
