//! `AspectRatio` component connect API.
//!
//! `AspectRatio` is a stateless, framework-agnostic attribute mapper for
//! responsive containers that preserve a width-to-height ratio.

use alloc::{format, string::String};

use ars_core::{AttrMap, ComponentPart, ConnectApi, CssProperty};

fn positive_finite_or_default(ratio: f64) -> f64 {
    if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    }
}

/// Props for the `AspectRatio` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Width-to-height ratio.
    ///
    /// For example, `16.0 / 9.0` represents a widescreen container. The value
    /// must be positive and finite.
    pub ratio: f64,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            ratio: 1.0,
        }
    }
}

impl Props {
    /// Returns fresh aspect-ratio props with the documented defaults.
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

    /// Sets the width-to-height ratio.
    #[must_use]
    pub const fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = ratio;
        self
    }

    /// CSS `padding-top` percentage that enforces the ratio.
    ///
    /// `padding-top: X%` is relative to element width, so `X = (1 / ratio) *
    /// 100`.
    #[must_use]
    pub fn padding_top_percent(&self) -> f64 {
        (1.0 / positive_finite_or_default(self.ratio)) * 100.0
    }
}

/// Structural parts exposed by the `AspectRatio` connect API.
#[derive(ComponentPart)]
#[scope = "aspect-ratio"]
pub enum Part {
    /// The root intrinsic-ratio container.
    Root,
}

/// API for the `AspectRatio` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Creates a new API for the aspect-ratio container.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::layout::aspect_ratio::{Api, Props};
    /// use ars_core::{CssProperty, HtmlAttr};
    ///
    /// let api = Api::new(Props::new().ratio(16.0 / 9.0));
    /// let attrs = api.root_attrs();
    ///
    /// assert_eq!(
    ///     attrs.get(&HtmlAttr::Data("ars-scope")),
    ///     Some("aspect-ratio")
    /// );
    /// assert!(attrs
    ///     .styles()
    ///     .contains(&(CssProperty::PaddingTop, "56.2500%".to_string())));
    /// ```
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns root container attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        let padding = self.props.padding_top_percent();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set_style(CssProperty::Position, "relative")
            .set_style(CssProperty::Width, "100%")
            .set_style(CssProperty::PaddingTop, format!("{padding:.4}%"));

        attrs
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

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use ars_core::{ConnectApi, CssProperty, HtmlAttr};
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new().id("media").ratio(16.0 / 9.0);

        assert_eq!(props.id, "media");
        assert_eq!(props.ratio, 16.0 / 9.0);
    }

    #[test]
    fn padding_top_percent_calculates_common_ratios() {
        let widescreen = Props::new().ratio(16.0 / 9.0);
        let standard = Props::new().ratio(4.0 / 3.0);
        let square = Props::new().ratio(1.0);

        assert!((widescreen.padding_top_percent() - 56.25).abs() < f64::EPSILON);
        assert!((standard.padding_top_percent() - 75.0).abs() < f64::EPSILON);
        assert!((square.padding_top_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn padding_top_percent_normalizes_invalid_ratio_to_default() {
        for ratio in [0.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(Props::new().ratio(ratio).padding_top_percent(), 100.0);
        }
    }

    #[test]
    fn root_attrs_emit_scope_part_and_ratio_styles() {
        let attrs = Api::new(Props::new().ratio(16.0 / 9.0)).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("aspect-ratio")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Position, String::from("relative")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Width, String::from("100%")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::PaddingTop, String::from("56.2500%")))
        );
    }

    #[test]
    fn root_attrs_normalizes_invalid_ratio_to_default_styles() {
        let attrs = Api::new(Props::new().ratio(0.0)).root_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::PaddingTop, String::from("100.0000%")))
        );
    }

    #[test]
    fn part_attrs_delegates_to_root_attrs() {
        let api = Api::new(Props::new().ratio(4.0 / 3.0));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn aspect_ratio_root_snapshot() {
        assert_snapshot!(
            "aspect_ratio_root",
            snapshot_attrs(&Api::new(Props::new().ratio(16.0 / 9.0)).root_attrs())
        );
    }
}
