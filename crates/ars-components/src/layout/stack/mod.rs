//! Stack component connect API.
//!
//! `Stack` is a stateless, framework-agnostic attribute mapper for flex
//! containers that arrange children in a row or column with consistent spacing.

use alloc::{boxed::Box, format, string::String};
use core::fmt::{self, Debug};

use ars_core::{AttrMap, ComponentPart, ConnectApi, CssProperty};

/// Direction of a stack flex container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackDirection {
    /// Lay children out from inline start to inline end.
    #[default]
    Row,

    /// Lay children out from inline end to inline start.
    RowReverse,

    /// Lay children out from block start to block end.
    Column,

    /// Lay children out from block end to block start.
    ColumnReverse,

    /// Resolve to [`Self::Row`] in LTR and [`Self::RowReverse`] in RTL.
    RowLogical,

    /// Resolve to [`Self::RowReverse`] in LTR and [`Self::Row`] in RTL.
    RowReverseLogical,
}

impl StackDirection {
    /// Resolves logical directions using the current text direction.
    #[must_use]
    pub const fn resolve(self, is_rtl: bool) -> Self {
        match self {
            Self::RowLogical => {
                if is_rtl {
                    Self::RowReverse
                } else {
                    Self::Row
                }
            }

            Self::RowReverseLogical => {
                if is_rtl {
                    Self::Row
                } else {
                    Self::RowReverse
                }
            }

            other => other,
        }
    }

    /// Returns the CSS `flex-direction` value.
    #[must_use]
    pub const fn css_value(self) -> &'static str {
        match self {
            Self::Row | Self::RowLogical => "row",
            Self::RowReverse | Self::RowReverseLogical => "row-reverse",
            Self::Column => "column",
            Self::ColumnReverse => "column-reverse",
        }
    }
}

/// CSS `align-items` values used by layout primitives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlexAlign {
    /// Stretch items to fill the cross axis.
    #[default]
    Stretch,

    /// Align items to flex start.
    Start,

    /// Align items to flex end.
    End,

    /// Align items to center.
    Center,

    /// Align items by baseline.
    Baseline,
}

impl FlexAlign {
    /// Returns the CSS `align-items` value.
    #[must_use]
    pub const fn css_value(self) -> &'static str {
        match self {
            Self::Stretch => "stretch",
            Self::Start => "flex-start",
            Self::End => "flex-end",
            Self::Center => "center",
            Self::Baseline => "baseline",
        }
    }
}

/// CSS `justify-content` values used by layout primitives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlexJustify {
    /// Place items at flex start.
    #[default]
    Start,

    /// Place items at flex end.
    End,

    /// Place items in the center.
    Center,

    /// Distribute items with space between adjacent items.
    SpaceBetween,

    /// Distribute items with equal space around each item.
    SpaceAround,

    /// Distribute items with equal space between and around items.
    SpaceEvenly,
}

impl FlexJustify {
    /// Returns the CSS `justify-content` value.
    #[must_use]
    pub const fn css_value(self) -> &'static str {
        match self {
            Self::Start => "flex-start",
            Self::End => "flex-end",
            Self::Center => "center",
            Self::SpaceBetween => "space-between",
            Self::SpaceAround => "space-around",
            Self::SpaceEvenly => "space-evenly",
        }
    }
}

/// CSS `text-align` values with logical start/end handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAlign {
    /// Align text to inline start.
    Start,

    /// Align text to center.
    Center,

    /// Align text to inline end.
    End,
}

impl TextAlign {
    /// Returns the physical CSS `text-align` value for the current direction.
    #[must_use]
    pub const fn css_value(self, is_rtl: bool) -> &'static str {
        match self {
            Self::Start => {
                if is_rtl {
                    "right"
                } else {
                    "left"
                }
            }

            Self::Center => "center",

            Self::End => {
                if is_rtl {
                    "left"
                } else {
                    "right"
                }
            }
        }
    }
}

/// Spacing value for layout primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum Spacing {
    /// Raw pixel spacing.
    Px(f64),

    /// Design token resolved through [`TokenResolver`].
    Token(String),

    /// Verbatim CSS spacing value.
    Css(String),
}

impl Spacing {
    /// Converts this spacing value to a CSS string.
    #[must_use]
    pub fn to_css(&self, resolver: Option<&dyn TokenResolver>) -> String {
        match self {
            Self::Px(value) => format!("{value}px"),

            Self::Token(key) => resolver
                .and_then(|resolver| resolver.resolve(key))
                .unwrap_or_else(|| String::from("0px")),

            Self::Css(value) => value.clone(),
        }
    }
}

/// Design-token resolver used by layout primitive props.
#[cfg(not(target_arch = "wasm32"))]
pub trait TokenResolver: Send + Sync {
    /// Resolves a token key to a CSS value.
    fn resolve(&self, key: &str) -> Option<String>;
}

/// Design-token resolver used by layout primitive props.
#[cfg(target_arch = "wasm32")]
pub trait TokenResolver {
    /// Resolves a token key to a CSS value.
    fn resolve(&self, key: &str) -> Option<String>;
}

/// Props for the `Stack` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Flex direction, including logical RTL-aware variants.
    pub direction: StackDirection,

    /// Gap between children using CSS `gap`.
    pub spacing: Option<Spacing>,

    /// Cross-axis alignment.
    pub align: FlexAlign,

    /// Main-axis distribution.
    pub justify: FlexJustify,

    /// Whether the flex container wraps.
    pub wrap: bool,

    /// Whether consumers should render visual dividers between children.
    pub divider: bool,

    /// Whether to set `width: 100%`.
    pub full_width: bool,

    /// Whether to set `height: 100%`.
    pub full_height: bool,
}

impl Props {
    /// Returns fresh stack props with documented defaults.
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

    /// Sets the flex direction.
    #[must_use]
    pub const fn direction(mut self, direction: StackDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Sets child spacing.
    #[must_use]
    pub fn spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Sets cross-axis alignment.
    #[must_use]
    pub const fn align(mut self, align: FlexAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets main-axis distribution.
    #[must_use]
    pub const fn justify(mut self, justify: FlexJustify) -> Self {
        self.justify = justify;
        self
    }

    /// Sets whether children wrap.
    #[must_use]
    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Sets whether consumers should render visual dividers.
    #[must_use]
    pub const fn divider(mut self, divider: bool) -> Self {
        self.divider = divider;
        self
    }

    /// Sets whether the stack fills available width.
    #[must_use]
    pub const fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Sets whether the stack fills available height.
    #[must_use]
    pub const fn full_height(mut self, full_height: bool) -> Self {
        self.full_height = full_height;
        self
    }

    /// Applies inline styles for this stack to the given attributes.
    pub fn apply_styles(
        &self,
        attrs: &mut AttrMap,
        is_rtl: bool,
        resolver: Option<&dyn TokenResolver>,
    ) {
        let direction = self.direction.resolve(is_rtl);

        attrs
            .set_style(CssProperty::Display, "flex")
            .set_style(CssProperty::FlexDirection, direction.css_value())
            .set_style(CssProperty::AlignItems, self.align.css_value())
            .set_style(CssProperty::JustifyContent, self.justify.css_value());

        if let Some(spacing) = &self.spacing {
            attrs.set_style(CssProperty::Gap, spacing.to_css(resolver));
        }
        if self.wrap {
            attrs.set_style(CssProperty::FlexWrap, "wrap");
        }
        if self.full_width {
            attrs.set_style(CssProperty::Width, "100%");
        }
        if self.full_height {
            attrs.set_style(CssProperty::Height, "100%");
        }
    }
}

/// Structural parts exposed by the `Stack` connect API.
#[derive(ComponentPart)]
#[scope = "stack"]
pub enum Part {
    /// The root flex container.
    Root,
}

/// API for the `Stack` component.
pub struct Api {
    props: Props,
    is_rtl: bool,
    resolver: Option<Box<dyn TokenResolver>>,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("props", &self.props)
            .field("is_rtl", &self.is_rtl)
            .finish_non_exhaustive()
    }
}

impl Api {
    /// Creates a new stack API.
    #[must_use]
    pub const fn new(props: Props, is_rtl: bool, resolver: Option<Box<dyn TokenResolver>>) -> Self {
        Self {
            props,
            is_rtl,
            resolver,
        }
    }

    /// Returns root container attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        self.props
            .apply_styles(&mut attrs, self.is_rtl, self.resolver.as_deref());

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use ars_core::{AttrMap, ConnectApi, CssProperty, HtmlAttr};
    use insta::assert_snapshot;

    use super::*;

    struct TestResolver;

    impl TokenResolver for TestResolver {
        fn resolve(&self, key: &str) -> Option<String> {
            match key {
                "md" => Some("1rem".into()),
                _ => None,
            }
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("stack")
            .direction(StackDirection::Column)
            .spacing(Spacing::Px(12.0))
            .align(FlexAlign::Center)
            .justify(FlexJustify::SpaceBetween)
            .wrap(true)
            .divider(true)
            .full_width(true)
            .full_height(true);

        assert_eq!(props.id, "stack");
        assert_eq!(props.direction, StackDirection::Column);
        assert_eq!(props.spacing, Some(Spacing::Px(12.0)));
        assert_eq!(props.align, FlexAlign::Center);
        assert_eq!(props.justify, FlexJustify::SpaceBetween);
        assert!(props.wrap);
        assert!(props.divider);
        assert!(props.full_width);
        assert!(props.full_height);
    }

    #[test]
    fn logical_direction_resolves_for_ltr_and_rtl() {
        assert_eq!(
            StackDirection::RowLogical.resolve(false),
            StackDirection::Row
        );
        assert_eq!(
            StackDirection::RowLogical.resolve(true),
            StackDirection::RowReverse
        );
        assert_eq!(
            StackDirection::RowReverseLogical.resolve(false),
            StackDirection::RowReverse
        );
        assert_eq!(
            StackDirection::RowReverseLogical.resolve(true),
            StackDirection::Row
        );
        assert_eq!(StackDirection::Column.resolve(true), StackDirection::Column);
    }

    #[test]
    fn spacing_renders_pixels_tokens_css_and_missing_tokens() {
        let resolver = TestResolver;

        assert_eq!(Spacing::Px(8.5).to_css(None), "8.5px");
        assert_eq!(Spacing::Css("2ch".into()).to_css(None), "2ch");
        assert_eq!(Spacing::Token("md".into()).to_css(Some(&resolver)), "1rem");
        assert_eq!(
            Spacing::Token("missing".into()).to_css(Some(&resolver)),
            "0px"
        );
    }

    #[test]
    fn root_attrs_emit_scope_part_and_flex_styles() {
        let attrs = Api::new(
            Props::new()
                .direction(StackDirection::Column)
                .spacing(Spacing::Token("md".into()))
                .align(FlexAlign::Baseline)
                .justify(FlexJustify::SpaceEvenly)
                .wrap(true),
            false,
            Some(Box::new(TestResolver)),
        )
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("stack"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Display, "flex".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::FlexDirection, "column".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::AlignItems, "baseline".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::JustifyContent, "space-evenly".into()))
        );
        assert!(attrs.styles().contains(&(CssProperty::Gap, "1rem".into())));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::FlexWrap, "wrap".into()))
        );
    }

    #[test]
    fn root_attrs_resolve_rtl_logical_direction() {
        let attrs = Api::new(
            Props::new().direction(StackDirection::RowLogical),
            true,
            None,
        )
        .root_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::FlexDirection, "row-reverse".into()))
        );
    }

    #[test]
    fn part_attrs_delegates_to_root_attrs() {
        let api = Api::new(Props::new().full_width(true), false, None);

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn stack_root_default_snapshot() {
        assert_snapshot!(
            "stack_root_default",
            snapshot_attrs(&Api::new(Props::new(), false, None).root_attrs())
        );
    }

    #[test]
    fn stack_root_rtl_logical_snapshot() {
        assert_snapshot!(
            "stack_root_rtl_logical",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .direction(StackDirection::RowLogical)
                        .spacing(Spacing::Css("0.5rem".into())),
                    true,
                    None,
                )
                .root_attrs()
            )
        );
    }

    #[test]
    fn stack_root_full_size_snapshot() {
        assert_snapshot!(
            "stack_root_full_size",
            snapshot_attrs(
                &Api::new(Props::new().full_width(true).full_height(true), false, None)
                    .root_attrs()
            )
        );
    }
}
