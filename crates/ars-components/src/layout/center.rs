//! Center component connect API.
//!
//! `Center` is a stateless, framework-agnostic attribute mapper for horizontal
//! and vertical centering with logical CSS properties.

use alloc::{boxed::Box, string::String};
use core::fmt::{self, Debug};

use ars_core::{AttrMap, ComponentPart, ConnectApi, CssProperty};

use super::stack::{Spacing, TextAlign, TokenResolver};

/// Props for the `Center` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Maximum inline-size constraint.
    pub max_width: Option<Spacing>,

    /// Whether to center horizontally with `margin-inline: auto`.
    pub horizontal: bool,

    /// Whether to center vertically with flex centering.
    pub vertical: bool,

    /// Text alignment within the container.
    pub text_align: Option<TextAlign>,
}

impl Props {
    /// Returns fresh center props with documented defaults.
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

    /// Sets the maximum inline size.
    #[must_use]
    pub fn max_width(mut self, max_width: Spacing) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Sets whether to center horizontally.
    #[must_use]
    pub const fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Sets whether to center vertically.
    #[must_use]
    pub const fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Sets text alignment.
    #[must_use]
    pub const fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = Some(text_align);
        self
    }

    /// Applies inline styles for this center to the given attributes.
    pub fn apply_styles(
        &self,
        attrs: &mut AttrMap,
        is_rtl: bool,
        resolver: Option<&dyn TokenResolver>,
    ) {
        if let Some(max_width) = &self.max_width {
            attrs.set_style(CssProperty::MaxInlineSize, max_width.to_css(resolver));
        }
        if self.horizontal {
            attrs.set_style(CssProperty::MarginInline, "auto");
        }
        if self.vertical {
            attrs
                .set_style(CssProperty::Display, "flex")
                .set_style(CssProperty::AlignItems, "center")
                .set_style(CssProperty::JustifyContent, "center");
        }
        if let Some(text_align) = self.text_align {
            attrs.set_style(CssProperty::TextAlign, text_align.css_value(is_rtl));
        }
    }
}

/// Structural parts exposed by the `Center` connect API.
#[derive(ComponentPart)]
#[scope = "center"]
pub enum Part {
    /// The root centering container.
    Root,
}

/// API for the `Center` component.
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
    /// Creates a new center API.
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
    use crate::layout::stack::{Spacing, TextAlign, TokenResolver};

    struct TestResolver;

    impl TokenResolver for TestResolver {
        fn resolve(&self, key: &str) -> Option<String> {
            match key {
                "container" => Some("72rem".into()),
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
            .id("center")
            .max_width(Spacing::Token("container".into()))
            .horizontal(true)
            .vertical(true)
            .text_align(TextAlign::End);

        assert_eq!(props.id, "center");
        assert_eq!(props.max_width, Some(Spacing::Token("container".into())));
        assert!(props.horizontal);
        assert!(props.vertical);
        assert_eq!(props.text_align, Some(TextAlign::End));
    }

    #[test]
    fn text_align_resolves_logical_directions() {
        assert_eq!(TextAlign::Start.css_value(false), "left");
        assert_eq!(TextAlign::Start.css_value(true), "right");
        assert_eq!(TextAlign::End.css_value(false), "right");
        assert_eq!(TextAlign::End.css_value(true), "left");
        assert_eq!(TextAlign::Center.css_value(true), "center");
    }

    #[test]
    fn root_attrs_emit_scope_part_and_center_styles() {
        let attrs = Api::new(
            Props::new()
                .max_width(Spacing::Token("container".into()))
                .horizontal(true)
                .vertical(true)
                .text_align(TextAlign::Start),
            true,
            Some(Box::new(TestResolver)),
        )
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("center"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::MaxInlineSize, "72rem".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::MarginInline, "auto".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Display, "flex".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::AlignItems, "center".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::JustifyContent, "center".into()))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::TextAlign, "right".into()))
        );
    }

    #[test]
    fn part_attrs_delegates_to_root_attrs() {
        let api = Api::new(Props::new().horizontal(true), false, None);

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn center_root_default_snapshot() {
        assert_snapshot!(
            "center_root_default",
            snapshot_attrs(&Api::new(Props::new(), false, None).root_attrs())
        );
    }

    #[test]
    fn center_root_max_width_vertical_snapshot() {
        assert_snapshot!(
            "center_root_max_width_vertical",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .max_width(Spacing::Css("80ch".into()))
                        .horizontal(true)
                        .vertical(true),
                    false,
                    None,
                )
                .root_attrs()
            )
        );
    }

    #[test]
    fn center_root_rtl_text_end_snapshot() {
        assert_snapshot!(
            "center_root_rtl_text_end",
            snapshot_attrs(
                &Api::new(Props::new().text_align(TextAlign::End), true, None).root_attrs()
            )
        );
    }
}
