//! Grid component connect API.
//!
//! `Grid` is a stateless, framework-agnostic attribute mapper for CSS grid
//! containers with declarative columns, gaps, and alignment.

use alloc::{boxed::Box, format, string::String};
use core::fmt::{self, Debug};

use ars_core::{AttrMap, ComponentPart, ConnectApi, CssProperty};

use super::stack::{FlexAlign, Spacing, TokenResolver};

/// Props for the `Grid` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Number of equal-width columns.
    pub columns: Option<u32>,

    /// Minimum column width for `auto-fill`.
    pub auto_columns: Option<Spacing>,

    /// Row gap.
    pub row_gap: Option<Spacing>,

    /// Column gap.
    pub column_gap: Option<Spacing>,

    /// Uniform gap, overriding row and column gaps.
    pub gap: Option<Spacing>,

    /// Cross-axis alignment.
    pub align: Option<FlexAlign>,

    /// Whether grid items stretch to fill their cells.
    pub stretch: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            columns: Some(1),
            auto_columns: None,
            row_gap: None,
            column_gap: None,
            gap: None,
            align: None,
            stretch: false,
        }
    }
}

impl Props {
    /// Returns fresh grid props with documented defaults.
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

    /// Sets the explicit column count.
    #[must_use]
    pub const fn columns(mut self, columns: Option<u32>) -> Self {
        self.columns = columns;
        self
    }

    /// Sets the auto-fill minimum column size.
    #[must_use]
    pub fn auto_columns(mut self, auto_columns: Option<Spacing>) -> Self {
        self.auto_columns = auto_columns;
        self
    }

    /// Sets the row gap.
    #[must_use]
    pub fn row_gap(mut self, row_gap: Spacing) -> Self {
        self.row_gap = Some(row_gap);
        self
    }

    /// Sets the column gap.
    #[must_use]
    pub fn column_gap(mut self, column_gap: Spacing) -> Self {
        self.column_gap = Some(column_gap);
        self
    }

    /// Sets the uniform gap.
    #[must_use]
    pub fn gap(mut self, gap: Spacing) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Sets cross-axis alignment.
    #[must_use]
    pub const fn align(mut self, align: FlexAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// Sets whether grid items stretch to fill cells.
    #[must_use]
    pub const fn stretch(mut self, stretch: bool) -> Self {
        self.stretch = stretch;
        self
    }

    /// Applies inline styles for this grid to the given attributes.
    pub fn apply_styles(&self, attrs: &mut AttrMap, resolver: Option<&dyn TokenResolver>) {
        attrs.set_style(CssProperty::Display, "grid");

        if let Some(columns) = self.columns {
            attrs.set_style(
                CssProperty::GridTemplateColumns,
                format!("repeat({columns}, minmax(0, 1fr))"),
            );
        } else if let Some(auto_columns) = &self.auto_columns {
            attrs.set_style(
                CssProperty::GridTemplateColumns,
                format!(
                    "repeat(auto-fill, minmax({}, 1fr))",
                    auto_columns.to_css(resolver)
                ),
            );
        }

        if let Some(gap) = &self.gap {
            attrs.set_style(CssProperty::Gap, gap.to_css(resolver));
        } else {
            if let Some(row_gap) = &self.row_gap {
                attrs.set_style(CssProperty::RowGap, row_gap.to_css(resolver));
            }
            if let Some(column_gap) = &self.column_gap {
                attrs.set_style(CssProperty::ColumnGap, column_gap.to_css(resolver));
            }
        }

        if let Some(align) = self.align {
            attrs.set_style(CssProperty::AlignItems, align.css_value());
        }
    }
}

/// Structural parts exposed by the `Grid` connect API.
#[derive(ComponentPart)]
#[scope = "grid"]
pub enum Part {
    /// The root grid container.
    Root,
}

/// API for the `Grid` component.
pub struct Api {
    props: Props,
    resolver: Option<Box<dyn TokenResolver>>,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("props", &self.props)
            .finish_non_exhaustive()
    }
}

impl Api {
    /// Creates a new grid API.
    #[must_use]
    pub const fn new(props: Props, resolver: Option<Box<dyn TokenResolver>>) -> Self {
        Self { props, resolver }
    }

    /// Returns root container attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        self.props
            .apply_styles(&mut attrs, self.resolver.as_deref());

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
    use crate::layout::stack::{FlexAlign, Spacing, TokenResolver};

    struct TestResolver;

    impl TokenResolver for TestResolver {
        fn resolve(&self, key: &str) -> Option<String> {
            match key {
                "gap" => Some("1.25rem".into()),
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
            .id("grid")
            .columns(Some(3))
            .auto_columns(Some(Spacing::Css("12rem".into())))
            .row_gap(Spacing::Px(8.0))
            .column_gap(Spacing::Px(16.0))
            .gap(Spacing::Token("gap".into()))
            .align(FlexAlign::Center)
            .stretch(true);

        assert_eq!(props.id, "grid");
        assert_eq!(props.columns, Some(3));
        assert_eq!(props.auto_columns, Some(Spacing::Css("12rem".into())));
        assert_eq!(props.row_gap, Some(Spacing::Px(8.0)));
        assert_eq!(props.column_gap, Some(Spacing::Px(16.0)));
        assert_eq!(props.gap, Some(Spacing::Token("gap".into())));
        assert_eq!(props.align, Some(FlexAlign::Center));
        assert!(props.stretch);
    }

    #[test]
    fn default_root_attrs_emit_one_column_grid() {
        let attrs = Api::new(Props::new(), None).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("grid"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Display, "grid".into()))
        );
        assert!(attrs.styles().contains(&(
            CssProperty::GridTemplateColumns,
            "repeat(1, minmax(0, 1fr))".into()
        )));
    }

    #[test]
    fn auto_columns_are_used_when_columns_are_none() {
        let attrs = Api::new(
            Props::new()
                .columns(None)
                .auto_columns(Some(Spacing::Css("14rem".into()))),
            None,
        )
        .root_attrs();

        assert!(attrs.styles().contains(&(
            CssProperty::GridTemplateColumns,
            "repeat(auto-fill, minmax(14rem, 1fr))".into()
        )));
    }

    #[test]
    fn uniform_gap_overrides_axis_gaps() {
        let attrs = Api::new(
            Props::new()
                .row_gap(Spacing::Px(4.0))
                .column_gap(Spacing::Px(8.0))
                .gap(Spacing::Token("gap".into())),
            Some(Box::new(TestResolver)),
        )
        .root_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Gap, "1.25rem".into()))
        );
        assert!(
            !attrs
                .styles()
                .iter()
                .any(|(property, _)| *property == CssProperty::RowGap)
        );
        assert!(
            !attrs
                .styles()
                .iter()
                .any(|(property, _)| *property == CssProperty::ColumnGap)
        );
    }

    #[test]
    fn part_attrs_delegates_to_root_attrs() {
        let api = Api::new(Props::new().columns(Some(4)), None);

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn grid_root_default_snapshot() {
        assert_snapshot!(
            "grid_root_default",
            snapshot_attrs(&Api::new(Props::new(), None).root_attrs())
        );
    }

    #[test]
    fn grid_root_auto_columns_snapshot() {
        assert_snapshot!(
            "grid_root_auto_columns",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .columns(None)
                        .auto_columns(Some(Spacing::Css("16rem".into()))),
                    None,
                )
                .root_attrs()
            )
        );
    }

    #[test]
    fn grid_root_gap_overrides_axis_gaps_snapshot() {
        assert_snapshot!(
            "grid_root_gap_overrides_axis_gaps",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .row_gap(Spacing::Px(4.0))
                        .column_gap(Spacing::Px(8.0))
                        .gap(Spacing::Css("2rem".into())),
                    None,
                )
                .root_attrs()
            )
        );
    }
}
