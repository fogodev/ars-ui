//! Frame component connect API.
//!
//! `Frame` is a stateless, framework-agnostic attribute mapper for iframe
//! embeds, including sandboxing, permissions policy, lazy loading, and
//! optional responsive aspect-ratio sizing.

use alloc::{format, string::String};

use ars_core::{AttrMap, ComponentPart, ConnectApi, CssProperty, HtmlAttr};

fn padding_percent_or_default(ratio: f64) -> f64 {
    let padding = 100.0 / ratio;

    if padding.is_finite() && padding > 0.0 {
        padding
    } else {
        100.0
    }
}

/// Loading strategy for iframe content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LoadingStrategy {
    /// Load the iframe immediately using the browser's default behavior.
    #[default]
    Eager,

    /// Defer loading until the iframe is near the viewport.
    Lazy,
}

/// Props for the `Frame` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// URL of the content to embed.
    pub src: String,

    /// Accessible title for the iframe.
    ///
    /// Screen readers announce this as the frame's accessible name.
    pub title: String,

    /// Sandbox restrictions.
    ///
    /// Space-separated tokens such as `"allow-scripts allow-same-origin"` relax
    /// the sandbox. `Some("")` applies maximum sandboxing, while `None` omits
    /// the sandbox attribute.
    pub sandbox: Option<String>,

    /// Permissions policy for cross-origin features.
    pub allow: Option<String>,

    /// Loading strategy for iframe content.
    pub loading: LoadingStrategy,

    /// Optional width-to-height ratio for responsive sizing.
    pub aspect_ratio: Option<f64>,

    /// Explicit iframe or aspect-ratio container width as a CSS value.
    pub width: String,

    /// Explicit iframe height as a CSS value when no aspect ratio is set.
    pub height: String,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: String::new(),
            title: String::new(),
            sandbox: None,
            allow: None,
            loading: LoadingStrategy::Eager,
            aspect_ratio: None,
            width: "100%".into(),
            height: "auto".into(),
        }
    }
}

impl Props {
    /// Returns fresh frame props with the documented defaults.
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

    /// Sets the iframe source URL.
    #[must_use]
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.src = src.into();
        self
    }

    /// Sets the iframe accessible title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets sandbox restrictions for the iframe.
    #[must_use]
    pub fn sandbox(mut self, sandbox: impl Into<String>) -> Self {
        self.sandbox = Some(sandbox.into());
        self
    }

    /// Sets the iframe permissions policy.
    #[must_use]
    pub fn allow(mut self, allow: impl Into<String>) -> Self {
        self.allow = Some(allow.into());
        self
    }

    /// Sets the iframe loading strategy.
    #[must_use]
    pub const fn loading(mut self, loading: LoadingStrategy) -> Self {
        self.loading = loading;
        self
    }

    /// Sets the responsive width-to-height ratio.
    #[must_use]
    pub const fn aspect_ratio(mut self, aspect_ratio: f64) -> Self {
        self.aspect_ratio = Some(aspect_ratio);
        self
    }

    /// Sets the explicit width CSS value.
    #[must_use]
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the explicit height CSS value.
    #[must_use]
    pub fn height(mut self, height: impl Into<String>) -> Self {
        self.height = height.into();
        self
    }
}

/// Structural parts exposed by the `Frame` connect API.
#[derive(ComponentPart)]
#[scope = "frame"]
pub enum Part {
    /// The outer container for the iframe.
    Root,

    /// The iframe element.
    Iframe,
}

/// API for the `Frame` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Creates a new API for the frame.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::layout::frame::{Api, LoadingStrategy, Props};
    /// use ars_core::HtmlAttr;
    ///
    /// let api = Api::new(
    ///     Props::new()
    ///         .src("https://example.com/embed")
    ///         .title("Example embed")
    ///         .loading(LoadingStrategy::Lazy),
    /// );
    /// let attrs = api.iframe_attrs();
    ///
    /// assert_eq!(attrs.get(&HtmlAttr::Src), Some("https://example.com/embed"));
    /// assert_eq!(attrs.get(&HtmlAttr::Title), Some("Example embed"));
    /// assert_eq!(attrs.get(&HtmlAttr::Loading), Some("lazy"));
    /// ```
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns attributes for the outer container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(ratio) = self.props.aspect_ratio {
            let padding = padding_percent_or_default(ratio);

            attrs
                .set_style(CssProperty::Position, "relative")
                .set_style(CssProperty::Width, self.props.width.as_str())
                .set_style(CssProperty::PaddingTop, format!("{padding:.4}%"));
        }

        attrs
    }

    /// Returns attributes for the iframe element.
    ///
    /// # Panics
    ///
    /// Panics when the frame title is empty or whitespace-only, because iframe
    /// titles are required accessible names.
    #[must_use]
    pub fn iframe_attrs(&self) -> AttrMap {
        assert!(
            !self.props.title.trim().is_empty(),
            "Frame title must be non-empty"
        );

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Iframe.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if !self.props.src.is_empty() {
            attrs.set(HtmlAttr::Src, self.props.src.as_str());
        }

        attrs.set(HtmlAttr::Title, self.props.title.as_str());

        if let Some(sandbox) = &self.props.sandbox {
            attrs.set(HtmlAttr::Sandbox, sandbox.as_str());
        }

        if let Some(allow) = &self.props.allow {
            attrs.set(HtmlAttr::Allow, allow.as_str());
        }

        if self.props.loading == LoadingStrategy::Lazy {
            attrs.set(HtmlAttr::Loading, "lazy");
        }

        if self.props.aspect_ratio.is_some() {
            attrs
                .set_style(CssProperty::Position, "absolute")
                .set_style(CssProperty::Inset, "0")
                .set_style(CssProperty::Width, "100%")
                .set_style(CssProperty::Height, "100%")
                .set_style(CssProperty::Border, "0");
        } else {
            attrs
                .set_style(CssProperty::Width, self.props.width.as_str())
                .set_style(CssProperty::Height, self.props.height.as_str())
                .set_style(CssProperty::Border, "0");
        }

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Iframe => self.iframe_attrs(),
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
        let props = Props::new()
            .id("demo-frame")
            .src("https://example.com/embed")
            .title("Demo embed")
            .sandbox("allow-scripts")
            .allow("camera; microphone")
            .loading(LoadingStrategy::Lazy)
            .aspect_ratio(16.0 / 9.0)
            .width("640px")
            .height("360px");

        assert_eq!(props.id, "demo-frame");
        assert_eq!(props.src, "https://example.com/embed");
        assert_eq!(props.title, "Demo embed");
        assert_eq!(props.sandbox.as_deref(), Some("allow-scripts"));
        assert_eq!(props.allow.as_deref(), Some("camera; microphone"));
        assert_eq!(props.loading, LoadingStrategy::Lazy);
        assert_eq!(props.aspect_ratio, Some(16.0 / 9.0));
        assert_eq!(props.width, "640px");
        assert_eq!(props.height, "360px");
    }

    #[test]
    fn loading_strategy_default_is_eager() {
        assert_eq!(LoadingStrategy::default(), LoadingStrategy::Eager);
    }

    #[test]
    fn root_attrs_emit_scope_and_part_without_aspect_styles() {
        let attrs = Api::new(Props::new()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("frame"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert!(attrs.styles().is_empty());
    }

    #[test]
    fn root_attrs_emit_aspect_ratio_sizing_styles() {
        let attrs = Api::new(Props::new().width("720px").aspect_ratio(4.0 / 3.0)).root_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Position, String::from("relative")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Width, String::from("720px")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::PaddingTop, String::from("75.0000%")))
        );
    }

    #[test]
    fn root_attrs_normalizes_invalid_aspect_ratio_to_default_styles() {
        for ratio in [
            0.0,
            -1.0,
            1.0e-308,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let attrs = Api::new(Props::new().aspect_ratio(ratio)).root_attrs();

            assert!(
                attrs
                    .styles()
                    .contains(&(CssProperty::PaddingTop, String::from("100.0000%")))
            );
        }
    }

    #[test]
    fn iframe_attrs_emit_required_src_title_and_default_sizing() {
        let attrs = Api::new(
            Props::new()
                .src("https://example.com/embed")
                .title("Example embed")
                .width("640px")
                .height("360px"),
        )
        .iframe_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("frame"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("iframe"));
        assert_eq!(attrs.get(&HtmlAttr::Src), Some("https://example.com/embed"));
        assert_eq!(attrs.get(&HtmlAttr::Title), Some("Example embed"));
        assert!(!attrs.contains(&HtmlAttr::Sandbox));
        assert!(!attrs.contains(&HtmlAttr::Allow));
        assert!(!attrs.contains(&HtmlAttr::Loading));
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Width, String::from("640px")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Height, String::from("360px")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Border, String::from("0")))
        );
    }

    #[test]
    #[should_panic(expected = "Frame title must be non-empty")]
    fn iframe_attrs_requires_non_empty_title() {
        let _attrs = Api::new(Props::new()).iframe_attrs();
    }

    #[test]
    #[should_panic(expected = "Frame title must be non-empty")]
    fn iframe_attrs_rejects_whitespace_only_title() {
        let _attrs = Api::new(Props::new().title("   ")).iframe_attrs();
    }

    #[test]
    fn iframe_attrs_omits_empty_src_after_title_is_configured() {
        let attrs = Api::new(Props::new().title("Example embed")).iframe_attrs();

        assert!(!attrs.contains(&HtmlAttr::Src));
        assert_eq!(attrs.get(&HtmlAttr::Title), Some("Example embed"));
    }

    #[test]
    fn iframe_attrs_emit_sandbox_allow_and_lazy_loading() {
        let attrs = Api::new(
            Props::new()
                .src("https://example.com/embed")
                .title("Example embed")
                .sandbox("allow-scripts allow-same-origin")
                .allow("camera; microphone")
                .loading(LoadingStrategy::Lazy),
        )
        .iframe_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Sandbox),
            Some("allow-scripts allow-same-origin")
        );
        assert_eq!(attrs.get(&HtmlAttr::Allow), Some("camera; microphone"));
        assert_eq!(attrs.get(&HtmlAttr::Loading), Some("lazy"));
    }

    #[test]
    fn iframe_attrs_emit_absolute_fill_styles_when_aspect_ratio_is_set() {
        let attrs =
            Api::new(Props::new().title("Example embed").aspect_ratio(16.0 / 9.0)).iframe_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Position, String::from("absolute")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Inset, String::from("0")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Width, String::from("100%")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Height, String::from("100%")))
        );
        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Border, String::from("0")))
        );
    }

    #[test]
    fn part_attrs_delegates_for_all_parts() {
        let api = Api::new(Props::new().title("Example embed").aspect_ratio(16.0 / 9.0));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Iframe), api.iframe_attrs());
    }

    #[test]
    fn frame_root_snapshots() {
        assert_snapshot!(
            "frame_root_default",
            snapshot_attrs(&Api::new(Props::new()).root_attrs())
        );
        assert_snapshot!(
            "frame_root_aspect_ratio",
            snapshot_attrs(
                &Api::new(Props::new().width("720px").aspect_ratio(4.0 / 3.0)).root_attrs()
            )
        );
    }

    #[test]
    fn frame_iframe_snapshots() {
        assert_snapshot!(
            "frame_iframe_default",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .src("https://example.com/embed")
                        .title("Example embed")
                )
                .iframe_attrs()
            )
        );
        assert_snapshot!(
            "frame_iframe_lazy_sandboxed",
            snapshot_attrs(
                &Api::new(
                    Props::new()
                        .src("https://example.com/embed")
                        .title("Example embed")
                        .sandbox("allow-scripts allow-same-origin")
                        .allow("camera; microphone")
                        .loading(LoadingStrategy::Lazy)
                )
                .iframe_attrs()
            )
        );
        assert_snapshot!(
            "frame_iframe_aspect_ratio",
            snapshot_attrs(
                &Api::new(Props::new().title("Example embed").aspect_ratio(16.0 / 9.0))
                    .iframe_attrs()
            )
        );
    }
}
