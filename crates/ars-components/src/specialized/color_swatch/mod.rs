//! `ColorSwatch` connect API.
//!
//! `ColorSwatch` is a stateless, non-interactive display element that renders a
//! color preview with an accessible, perceptual color name for screen readers.
//! No `Machine` is needed — it maps a [`ColorValue`] to attributes directly.
//!
//! The accessible name is derived from [`ColorValue::color_name_parts`] (which
//! returns English keys) assembled by the [`Messages::format_name`] message, so
//! the locale-appropriate ordering and token translations live in the i18n
//! layer rather than in the color math.

use alloc::{string::String, vec::Vec};

use ars_core::{
    AriaAttr, AttrMap, ColorNameParts, ColorValue, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, MessageFn,
};
use ars_i18n::Locale;

/// Formats [`ColorNameParts`] into a single localized accessible name.
type FormatNameFn = dyn Fn(&ColorNameParts, &Locale) -> String + Send + Sync;
/// Returns the localized `aria-roledescription` for the swatch root.
type RoleDescriptionFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Props for the `ColorSwatch` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID. When non-empty it is emitted as the root `id`.
    pub id: String,

    /// The color to display.
    pub color: ColorValue,

    /// Optional override for the auto-generated accessible color name.
    ///
    /// When `None`, the name is derived from
    /// [`ColorValue::color_name_parts`] + [`Messages::format_name`].
    pub color_name: Option<String>,

    /// Whether to visually represent the alpha channel (checkerboard pattern).
    pub respect_alpha: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            color: ColorValue::default(),
            color_name: None,
            respect_alpha: true,
        }
    }
}

/// Messages for the `ColorSwatch` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Formats the color name parts into a localized string.
    ///
    /// Default (en): `"{lightness} {chroma} {hue}"` — non-empty parts joined by
    /// spaces. Other locales may reorder the parts.
    pub format_name: MessageFn<FormatNameFn>,

    /// Role description for the swatch element (default: `"color swatch"`).
    pub role_description: MessageFn<RoleDescriptionFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            format_name: MessageFn::new(|parts: &ColorNameParts, _locale: &Locale| {
                [
                    parts.lightness.as_str(),
                    parts.chroma.as_str(),
                    parts.hue.as_str(),
                ]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
            }),
            role_description: MessageFn::static_str("color swatch"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the `ColorSwatch` connect API.
#[derive(ComponentPart)]
#[scope = "color-swatch"]
pub enum Part {
    /// Container with `role="img"` and the accessible color name.
    Root,

    /// Color fill element, with an optional checkerboard for alpha.
    Inner,
}

/// API for the `ColorSwatch` component.
#[derive(Debug)]
pub struct Api<'a> {
    props: &'a Props,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    /// Creates a new API for the swatch.
    #[must_use]
    pub fn new(props: &'a Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// The resolved accessible color name — either the explicit override or the
    /// auto-generated perceptual name.
    #[must_use]
    pub fn color_name(&self) -> String {
        if let Some(name) = &self.props.color_name {
            name.clone()
        } else {
            let parts = self.props.color.color_name_parts();

            (self.messages.format_name)(&parts, &self.locale)
        }
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        attrs
            .set(HtmlAttr::Role, "img")
            .set(HtmlAttr::Aria(AriaAttr::Label), self.color_name())
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.messages.role_description)(&self.locale),
            )
            .set_style(
                CssProperty::Custom("ars-swatch-color"),
                self.props.color.to_css_hsl(),
            );

        attrs
    }

    /// Attributes for the inner color-fill element.
    #[must_use]
    pub fn inner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Inner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set_style(CssProperty::Background, self.props.color.to_css_hsl());

        if self.props.respect_alpha && self.props.color.alpha < 1.0 {
            attrs.set_bool(HtmlAttr::Data("ars-alpha"), true);
        }

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Inner => self.inner_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use ars_core::ConnectApi;
    use insta::assert_snapshot;

    use super::*;

    fn api(props: &Props) -> Api<'_> {
        Api::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    #[test]
    fn renders_color_sample_with_img_role_and_label() {
        let props = Props {
            color: ColorValue::from_rgb(0, 0, 255),
            ..Props::default()
        };

        let attrs = api(&props).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("img"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::RoleDescription)),
            Some("color swatch")
        );
        // Auto-generated perceptual name describes the color.
        assert!(
            attrs
                .get(&HtmlAttr::Aria(AriaAttr::Label))
                .unwrap()
                .contains("blue")
        );
    }

    #[test]
    fn root_sets_background_color_css_variable() {
        let props = Props {
            color: ColorValue::from_rgb(255, 0, 0),
            ..Props::default()
        };

        let attrs = api(&props).root_attrs();

        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-swatch-color"),
            "hsl(0, 100.0%, 50.0%)".to_string()
        )));
    }

    #[test]
    fn inner_sets_background_and_alpha_flag() {
        let opaque = Props {
            color: ColorValue::from_rgb(0, 128, 0),
            ..Props::default()
        };

        let inner = api(&opaque).inner_attrs();

        assert!(
            inner
                .styles()
                .iter()
                .any(|(p, _)| *p == CssProperty::Background)
        );
        assert!(!inner.contains(&HtmlAttr::Data("ars-alpha")));

        let translucent = Props {
            color: ColorValue::new(120.0, 1.0, 0.25, 0.5),
            ..Props::default()
        };

        let inner = api(&translucent).inner_attrs();

        assert_eq!(inner.get(&HtmlAttr::Data("ars-alpha")), Some("true"));
    }

    #[test]
    fn respect_alpha_false_suppresses_alpha_flag() {
        let props = Props {
            color: ColorValue::new(120.0, 1.0, 0.25, 0.5),
            respect_alpha: false,
            ..Props::default()
        };

        assert!(
            !api(&props)
                .inner_attrs()
                .contains(&HtmlAttr::Data("ars-alpha"))
        );
    }

    #[test]
    fn explicit_color_name_overrides_perceptual_name() {
        let props = Props {
            color: ColorValue::from_rgb(255, 0, 0),
            color_name: Some("Brand Red".to_string()),
            ..Props::default()
        };

        assert_eq!(api(&props).color_name(), "Brand Red");
        assert_eq!(
            api(&props)
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Brand Red")
        );
    }

    #[test]
    fn id_is_emitted_only_when_non_empty() {
        let without = Props::default();

        assert!(!api(&without).root_attrs().contains(&HtmlAttr::Id));

        let with = Props {
            id: "swatch-1".to_string(),
            ..Props::default()
        };

        assert_eq!(api(&with).root_attrs().get(&HtmlAttr::Id), Some("swatch-1"));
    }

    #[test]
    fn part_attrs_delegates() {
        let props = Props::default();

        let swatch = api(&props);

        assert_eq!(swatch.part_attrs(Part::Root), swatch.root_attrs());
        assert_eq!(swatch.part_attrs(Part::Inner), swatch.inner_attrs());
    }

    #[test]
    fn root_default_snapshot() {
        assert_snapshot!(
            "color_swatch_root_default",
            snapshot_attrs(&api(&Props::default()).root_attrs())
        );
    }

    #[test]
    fn inner_translucent_snapshot() {
        let props = Props {
            color: ColorValue::new(210.0, 0.8, 0.5, 0.4),
            ..Props::default()
        };

        assert_snapshot!(
            "color_swatch_inner_translucent",
            snapshot_attrs(&api(&props).inner_attrs())
        );
    }
}
