//! `QrCode` connect API.
//!
//! `QrCode` is a stateless, declarative component that maps an input value to a
//! rendered QR code matrix with an accessible label, optional center overlay,
//! and an optional download trigger. No `Machine` is needed.
//!
//! QR matrix generation is **adapter-owned**: the agnostic core defines only the
//! rendering contract ([`QrMatrix`], [`Api`], [`Part`]). The matrix is supplied
//! to [`Api::new`] by the caller (the framework adapter encodes the value with a
//! QR library and injects the result). This keeps the core dependency-free and
//! `no_std`-compatible while letting adapters own encoding and download export.

use alloc::{format, string::String, vec::Vec};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Env, HtmlAttr,
    MessageFn,
};
use ars_i18n::Locale;

/// Formats the root `aria-label` from the encoded value (default
/// `"QR code: {value}"`, or a link-specific variant for URLs).
type LabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Returns the localized `aria-label` for the download trigger.
type DownloadLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Error correction level for QR encoding.
///
/// Higher levels recover from more damage at the cost of denser matrices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QrErrorCorrection {
    /// ~7% recovery.
    Low,

    /// ~15% recovery.
    #[default]
    Medium,

    /// ~25% recovery.
    Quartile,

    /// ~30% recovery.
    High,
}

/// The QR code matrix — a 2D grid of modules (dark/light cells).
///
/// The agnostic core does not generate matrices; it renders them. Adapters
/// encode the value with a QR library and construct a `QrMatrix` via
/// [`QrMatrix::new`], then inject it into [`Api::new`].
///
/// # Examples
///
/// ```
/// use ars_components::specialized::qr_code::{Api, Messages, Props, QrMatrix};
/// use ars_core::{AriaAttr, Env, HtmlAttr};
///
/// // The adapter encodes the value and supplies the matrix; the core renders it.
/// let matrix = QrMatrix::new(vec![
///     vec![true, false, true],
///     vec![false, true, false],
///     vec![true, false, true],
/// ]);
/// assert_eq!(matrix.size, 3);
///
/// let props = Props { value: "hello".into(), ..Props::default() };
/// let api = Api::new(&props, Some(matrix), &Env::default(), &Messages::default());
///
/// let root = api.root_attrs();
/// assert_eq!(root.get(&HtmlAttr::Role), Some("img"));
/// assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("QR code: hello"));
/// // (3 modules + 4 * 2 quiet-zone modules) * 4.0px module size = 44px.
/// assert_eq!(api.pixel_size(), 44.0);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrMatrix {
    /// Row-major module data. `true` = dark module.
    pub modules: Vec<Vec<bool>>,

    /// Size (number of modules per side).
    pub size: usize,
}

impl QrMatrix {
    /// Create a `QrMatrix` from a pre-computed module grid.
    ///
    /// QR matrix generation is delegated to a QR encoding crate in the adapter
    /// layer. The core defines the rendering contract only.
    #[must_use]
    pub const fn new(modules: Vec<Vec<bool>>) -> Self {
        Self {
            size: modules.len(),
            modules,
        }
    }

    /// Get the module value at `(row, col)`.
    ///
    /// Returns `false` for out-of-bounds coordinates; a `debug_assert!` flags
    /// such access during development.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> bool {
        debug_assert!(
            row < self.size && col < self.size,
            "QrMatrix::get() out of bounds: ({row}, {col}) for size {}",
            self.size
        );

        self.modules
            .get(row)
            .and_then(|cells| cells.get(col))
            .copied()
            .unwrap_or(false)
    }
}

/// Props for the `QrCode` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID. When non-empty it is emitted as the root `id`.
    pub id: String,

    /// The data to encode.
    pub value: String,

    /// Error correction level.
    pub error_correction: QrErrorCorrection,

    /// Size of each module in pixels (for rendering).
    pub module_size: f64,

    /// Quiet zone (border) in modules. Standard is 4.
    pub quiet_zone: usize,

    /// Foreground color (dark modules).
    pub foreground: String,

    /// Background color (light modules).
    pub background: String,

    /// Optional image overlay (e.g., logo) in the center.
    pub overlay_src: Option<String>,

    /// Overlay size as a fraction of the QR code size, in `[0.0, 0.5]`.
    pub overlay_size: f64,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: String::new(),
            error_correction: QrErrorCorrection::Medium,
            module_size: 4.0,
            quiet_zone: 4,
            foreground: "#000000".into(),
            background: "#ffffff".into(),
            overlay_src: None,
            overlay_size: 0.2,
        }
    }
}

/// Messages for the `QrCode` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Root `aria-label` template (default: `"QR code: {value}"`).
    pub label: MessageFn<LabelFn>,

    /// Label when the value is a URL (default: `"QR code linking to {url}"`).
    pub link_label: MessageFn<LabelFn>,

    /// Label for the download trigger button (default: `"Download QR code"`).
    pub download_label: MessageFn<DownloadLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::new(|value: &str, _locale: &Locale| format!("QR code: {value}")),
            link_label: MessageFn::new(|url: &str, _locale: &Locale| {
                format!("QR code linking to {url}")
            }),
            download_label: MessageFn::static_str("Download QR code"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the `QrCode` connect API.
#[derive(ComponentPart)]
#[scope = "qr-code"]
pub enum Part {
    /// Container with `role="img"`, sized to the rendered QR code.
    Root,

    /// Optional decorative frame around the code.
    Frame,

    /// The QR module grid, rendered as SVG or canvas by the adapter.
    Pattern,

    /// Optional centered image/logo overlay.
    Overlay,

    /// Optional button to download the QR code as an image.
    DownloadTrigger,
}

/// API for the `QrCode` component.
#[derive(Debug)]
pub struct Api<'a> {
    props: &'a Props,
    matrix: Option<QrMatrix>,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    /// Creates a new API from props, an (adapter-supplied) matrix, env, and
    /// messages.
    ///
    /// The `matrix` is `None` until the adapter has encoded `props.value`; the
    /// core renders whatever it is given.
    #[must_use]
    pub fn new(props: &'a Props, matrix: Option<QrMatrix>, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            matrix,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// The injected QR matrix, if one has been supplied.
    #[must_use]
    pub const fn matrix(&self) -> Option<&QrMatrix> {
        self.matrix.as_ref()
    }

    /// Total pixel size of the rendered QR code (including the quiet zone).
    ///
    /// Returns `0.0` when no matrix has been supplied.
    #[must_use]
    pub fn pixel_size(&self) -> f64 {
        match &self.matrix {
            Some(matrix) => {
                (matrix.size + self.props.quiet_zone * 2) as f64 * self.props.module_size
            }

            None => 0.0,
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

        let label = if self.props.value.starts_with("http://")
            || self.props.value.starts_with("https://")
        {
            (self.messages.link_label)(&self.props.value, &self.locale)
        } else {
            (self.messages.label)(&self.props.value, &self.locale)
        };

        let size = self.pixel_size();

        attrs
            .set(HtmlAttr::Role, "img")
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            .set_style(CssProperty::Width, format!("{size}px"))
            .set_style(CssProperty::Height, format!("{size}px"));

        attrs
    }

    /// Attributes for the optional decorative frame.
    #[must_use]
    pub fn frame_attrs(&self) -> AttrMap {
        part_attrs(&Part::Frame)
    }

    /// Attributes for the QR module grid.
    #[must_use]
    pub fn pattern_attrs(&self) -> AttrMap {
        part_attrs(&Part::Pattern)
    }

    /// Attributes for the optional centered overlay image.
    #[must_use]
    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Overlay);

        if let Some(src) = &self.props.overlay_src {
            attrs.set(HtmlAttr::Src, src.clone());
        }

        attrs
    }

    /// Attributes for the optional download trigger button.
    #[must_use]
    pub fn download_trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::DownloadTrigger);

        attrs.set(HtmlAttr::Type, "button").set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.messages.download_label)(&self.locale),
        );

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Frame => self.frame_attrs(),
            Part::Pattern => self.pattern_attrs(),
            Part::Overlay => self.overlay_attrs(),
            Part::DownloadTrigger => self.download_trigger_attrs(),
        }
    }
}

/// Builds an [`AttrMap`] seeded with a part's `data-ars-scope`/`data-ars-part`.
fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use ars_core::ConnectApi;
    use insta::assert_snapshot;

    use super::*;

    fn api<'a>(props: &'a Props, matrix: Option<QrMatrix>) -> Api<'a> {
        Api::new(props, matrix, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    /// A small fixture grid standing in for an encoded matrix. The core does not
    /// validate QR semantics, so any square grid exercises the rendering path.
    fn sample_matrix() -> QrMatrix {
        QrMatrix::new(vec![
            vec![true, false, true],
            vec![false, true, false],
            vec![true, false, true],
        ])
    }

    #[test]
    fn connect_produces_img_role_with_aria_label() {
        let props = Props {
            value: "hello".to_string(),
            ..Props::default()
        };

        let attrs = api(&props, Some(sample_matrix())).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("img"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("QR code: hello")
        );
    }

    #[test]
    fn url_value_uses_link_label() {
        let https = Props {
            value: "https://example.com".to_string(),
            ..Props::default()
        };

        assert_eq!(
            api(&https, Some(sample_matrix()))
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("QR code linking to https://example.com")
        );

        let http = Props {
            value: "http://example.com".to_string(),
            ..Props::default()
        };

        assert_eq!(
            api(&http, Some(sample_matrix()))
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("QR code linking to http://example.com")
        );
    }

    #[test]
    fn error_correction_has_four_levels_with_medium_default() {
        assert_eq!(QrErrorCorrection::default(), QrErrorCorrection::Medium);

        for level in [
            QrErrorCorrection::Low,
            QrErrorCorrection::Medium,
            QrErrorCorrection::Quartile,
            QrErrorCorrection::High,
        ] {
            let props = Props {
                error_correction: level,
                ..Props::default()
            };

            assert_eq!(props.error_correction, level);
        }
    }

    #[test]
    fn injected_matrix_drives_pixel_size_and_dimensions() {
        let props = Props {
            module_size: 5.0,
            quiet_zone: 2,
            ..Props::default()
        };

        let connected = api(&props, Some(sample_matrix()));

        // (size 3 + quiet_zone 2 * 2) * module_size 5 = 35
        assert!((connected.pixel_size() - 35.0).abs() < f64::EPSILON);

        let attrs = connected.root_attrs();

        assert_eq!(
            attrs
                .styles()
                .iter()
                .find(|(p, _)| *p == CssProperty::Width),
            Some(&(CssProperty::Width, "35px".to_string()))
        );
        assert_eq!(
            attrs
                .styles()
                .iter()
                .find(|(p, _)| *p == CssProperty::Height),
            Some(&(CssProperty::Height, "35px".to_string()))
        );
    }

    #[test]
    fn no_matrix_yields_zero_pixel_size() {
        let props = Props::default();
        let connected = api(&props, None);

        assert_eq!(connected.matrix(), None);
        assert!((connected.pixel_size() - 0.0).abs() < f64::EPSILON);

        let attrs = connected.root_attrs();

        assert_eq!(
            attrs
                .styles()
                .iter()
                .find(|(p, _)| *p == CssProperty::Width),
            Some(&(CssProperty::Width, "0px".to_string()))
        );
    }

    #[test]
    fn qr_matrix_new_computes_size_and_get() {
        let matrix = sample_matrix();

        assert_eq!(matrix.size, 3);
        assert!(matrix.get(0, 0));
        assert!(!matrix.get(0, 1));
        assert!(matrix.get(1, 1));
        assert!(!matrix.get(2, 1));
    }

    #[test]
    fn overlay_attrs_sets_src_only_when_present() {
        let without = Props::default();

        assert!(!api(&without, None).overlay_attrs().contains(&HtmlAttr::Src));

        let with = Props {
            overlay_src: Some("/logo.png".to_string()),
            ..Props::default()
        };

        assert_eq!(
            api(&with, None).overlay_attrs().get(&HtmlAttr::Src),
            Some("/logo.png")
        );
    }

    #[test]
    fn download_trigger_sets_type_button_and_label() {
        let props = Props::default();

        let attrs = api(&props, None).download_trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Download QR code")
        );
    }

    #[test]
    fn id_emitted_only_when_non_empty() {
        let without = Props::default();

        assert!(!api(&without, None).root_attrs().contains(&HtmlAttr::Id));

        let with = Props {
            id: "qr-1".to_string(),
            ..Props::default()
        };

        assert_eq!(
            api(&with, None).root_attrs().get(&HtmlAttr::Id),
            Some("qr-1")
        );
    }

    #[test]
    fn color_props_default_and_override() {
        let defaults = Props::default();

        assert_eq!(defaults.foreground, "#000000");
        assert_eq!(defaults.background, "#ffffff");

        let custom = Props {
            foreground: "#112233".to_string(),
            background: "#fefefe".to_string(),
            ..Props::default()
        };

        assert_eq!(custom.foreground, "#112233");
        assert_eq!(custom.background, "#fefefe");
    }

    #[test]
    fn part_attrs_delegates() {
        let props = Props {
            id: "qr-1".to_string(),
            value: "hello".to_string(),
            overlay_src: Some("/logo.png".to_string()),
            ..Props::default()
        };

        let connected = api(&props, Some(sample_matrix()));

        assert_eq!(connected.part_attrs(Part::Root), connected.root_attrs());
        assert_eq!(connected.part_attrs(Part::Frame), connected.frame_attrs());
        assert_eq!(
            connected.part_attrs(Part::Pattern),
            connected.pattern_attrs()
        );
        assert_eq!(
            connected.part_attrs(Part::Overlay),
            connected.overlay_attrs()
        );
        assert_eq!(
            connected.part_attrs(Part::DownloadTrigger),
            connected.download_trigger_attrs()
        );
    }

    #[test]
    fn root_default_snapshot() {
        let props = Props {
            value: "hello".to_string(),
            ..Props::default()
        };

        assert_snapshot!(
            "qr_code_root_default",
            snapshot_attrs(&api(&props, Some(sample_matrix())).root_attrs())
        );
    }

    #[test]
    fn root_url_with_id_snapshot() {
        let props = Props {
            id: "qr-1".to_string(),
            value: "https://example.com".to_string(),
            module_size: 8.0,
            quiet_zone: 2,
            ..Props::default()
        };

        assert_snapshot!(
            "qr_code_root_url_with_id",
            snapshot_attrs(&api(&props, Some(sample_matrix())).root_attrs())
        );
    }

    #[test]
    fn root_no_matrix_snapshot() {
        let props = Props {
            value: "hello".to_string(),
            ..Props::default()
        };

        assert_snapshot!(
            "qr_code_root_no_matrix",
            snapshot_attrs(&api(&props, None).root_attrs())
        );
    }

    #[test]
    fn frame_snapshot() {
        let props = Props::default();

        assert_snapshot!(
            "qr_code_frame",
            snapshot_attrs(&api(&props, None).frame_attrs())
        );
    }

    #[test]
    fn pattern_snapshot() {
        let props = Props::default();

        assert_snapshot!(
            "qr_code_pattern",
            snapshot_attrs(&api(&props, None).pattern_attrs())
        );
    }

    #[test]
    fn overlay_with_src_snapshot() {
        let props = Props {
            overlay_src: Some("/logo.png".to_string()),
            ..Props::default()
        };

        assert_snapshot!(
            "qr_code_overlay_with_src",
            snapshot_attrs(&api(&props, None).overlay_attrs())
        );
    }

    #[test]
    fn overlay_without_src_snapshot() {
        let props = Props::default();

        assert_snapshot!(
            "qr_code_overlay_without_src",
            snapshot_attrs(&api(&props, None).overlay_attrs())
        );
    }

    #[test]
    fn download_trigger_snapshot() {
        let props = Props::default();

        assert_snapshot!(
            "qr_code_download_trigger",
            snapshot_attrs(&api(&props, None).download_trigger_attrs())
        );
    }
}
