//! `DownloadTrigger` component connect API.
//!
//! Stateless attribute mapper for declarative file downloads via `<a download>`.
//! Same-origin vs cross-origin policy for absolute `http`/`https` URLs uses the
//! adapter-supplied [`Props::document_origin`] (typically `window.location.origin`).
//!
//! See [`spec/components/utility/download-trigger.md`](../../../../spec/components/utility/download-trigger.md)
//! for the authoritative contract.
//!
//! ## URL classification limits
//!
//! Origin parsing supports common `host`/`host:port` authorities and bracketed
//! IPv6 (`[::1]:443`). Hostnames with ambiguous `:` patterns outside these forms
//! may classify conservatively toward blob fallback for absolute HTTP(S) URLs.

use alloc::string::String;

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, HasId, HtmlAttr,
    IsolateDirection, Locale, MessageFn, isolate_text_safe,
};

/// Stable `data-ars-download-fallback` value when adapters must use fetch/blob
/// instead of the native `download` attribute.
pub const DOWNLOAD_FALLBACK_REQUIRED: &str = "required";

/// Props for the `DownloadTrigger` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// URL of the file to download (absolute URL, relative path, `data:` URI,
    /// or `blob:` URL).
    pub href: String,

    /// Optional filename for the downloaded file.
    pub filename: Option<String>,

    /// MIME type hint for the anchor `type` attribute and for blob fallback.
    pub mime_type: Option<String>,

    /// Disabled state — emits `aria-disabled` while preserving `href` for focus.
    pub disabled: bool,

    /// Adapter-supplied document origin (`scheme://host:port`, e.g.
    /// `https://example.com` or `https://example.com:8443`). Used only to decide
    /// whether absolute `http`/`https` URLs may use the native `download`
    /// attribute; relative URLs do not consult this field.
    pub document_origin: Option<String>,
}

impl Props {
    /// Returns fresh props with the documented defaults.
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

    /// Sets the download target URL (`href`).
    #[must_use]
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = href.into();
        self
    }

    /// Sets the suggested download filename.
    #[must_use]
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Clears the suggested download filename.
    #[must_use]
    pub fn clear_filename(mut self) -> Self {
        self.filename = None;
        self
    }

    /// Sets the MIME type hint (`type` attribute).
    #[must_use]
    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// Clears the MIME type hint.
    #[must_use]
    pub fn clear_mime_type(mut self) -> Self {
        self.mime_type = None;
        self
    }

    /// Sets the disabled state.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets the document origin used for HTTP(S) same-origin checks.
    #[must_use]
    pub fn document_origin(mut self, origin: impl Into<String>) -> Self {
        self.document_origin = Some(origin.into());
        self
    }

    /// Clears the document origin (conservative HTTP(S) classification).
    #[must_use]
    pub fn clear_document_origin(mut self) -> Self {
        self.document_origin = None;
        self
    }
}

/// Dynamic callable signature for [`Messages::download_label`].
pub type DownloadLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Messages for the `DownloadTrigger` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the download trigger.
    pub download_label: MessageFn<DownloadLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            download_label: MessageFn::new(|filename: &str, _locale: &Locale| {
                if filename.is_empty() {
                    String::from("Download file")
                } else {
                    format!(
                        "Download {}",
                        isolate_text_safe(filename, IsolateDirection::FirstStrong)
                    )
                }
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the download-trigger connect API.
#[derive(ComponentPart)]
#[scope = "download-trigger"]
pub enum Part {
    /// Root `<a>` element.
    Root,
}

/// API for the `DownloadTrigger` component.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Creates a new API from props, locale, and localized messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::utility::download_trigger::{Api, Messages, Props};
    /// use ars_i18n::locales::en_us;
    ///
    /// let api = Api::new(
    ///     Props::new().href("/report.pdf"),
    ///     en_us(),
    ///     Messages::default(),
    /// );
    /// assert!(api.native_download_eligible());
    /// ```
    #[must_use]
    pub const fn new(props: Props, locale: Locale, messages: Messages) -> Self {
        Self {
            props,
            locale,
            messages,
        }
    }

    /// Returns the underlying props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component instance ID.
    #[must_use]
    pub const fn id(&self) -> &str {
        self.props.id.as_str()
    }

    /// Returns whether the native `download` attribute should be emitted for
    /// this href/origin combination.
    #[must_use]
    pub fn native_download_eligible(&self) -> bool {
        matches!(
            classify_href(&self.props.href, self.props.document_origin.as_deref()),
            DownloadPolicy::Native
        )
    }

    /// Returns whether adapters should use fetch/blob fallback instead of
    /// relying on `<a download>` (cross-origin or unknown-origin HTTP(S)).
    #[must_use]
    pub fn needs_blob_fallback(&self) -> bool {
        matches!(
            classify_href(&self.props.href, self.props.document_origin.as_deref()),
            DownloadPolicy::BlobFallbackRequired
        )
    }

    /// Whether the trigger is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.props.disabled
    }

    /// The resolved filename from props.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        self.props.filename.as_deref()
    }

    /// Root `<a>` attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.props.id.as_str())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Href, self.props.href.as_str());

        if let Some(mime) = self.props.mime_type.as_deref() {
            attrs.set(HtmlAttr::Type, mime);
        }

        let policy = classify_href(&self.props.href, self.props.document_origin.as_deref());

        match policy {
            DownloadPolicy::Native => {
                if let Some(name) = &self.props.filename {
                    attrs.set(HtmlAttr::Download, name.as_str());
                } else {
                    attrs.set_bool(HtmlAttr::Download, true);
                }
            }

            DownloadPolicy::BlobFallbackRequired => {
                attrs.set(
                    HtmlAttr::Data("ars-download-fallback"),
                    DOWNLOAD_FALLBACK_REQUIRED,
                );
            }

            DownloadPolicy::NoDownloadHint => {}
        }

        let label = (self.messages.download_label)(
            self.props.filename.as_deref().unwrap_or(""),
            &self.locale,
        );

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);

        if self.props.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DownloadPolicy {
    /// Emit native `download` (same-origin semantics satisfied).
    Native,

    /// Omit `download`; emit `data-ars-download-fallback="required"`.
    BlobFallbackRequired,

    /// Omit `download` without blob fallback signaling (`mailto:`, unknown schemes, …).
    NoDownloadHint,
}

fn classify_href(href: &str, document_origin: Option<&str>) -> DownloadPolicy {
    let trimmed = href.trim_start();

    if trimmed.is_empty() {
        return DownloadPolicy::NoDownloadHint;
    }

    if is_relative_reference(trimmed) {
        return DownloadPolicy::Native;
    }

    let bytes = trimmed.as_bytes();

    if starts_with_ignore_case(bytes, b"blob:") || starts_with_ignore_case(bytes, b"data:") {
        return DownloadPolicy::Native;
    }

    if starts_with_ignore_case(bytes, b"mailto:") || starts_with_ignore_case(bytes, b"tel:") {
        return DownloadPolicy::NoDownloadHint;
    }

    if let Some(href_origin) = parse_http_https_origin(trimmed) {
        let Some(doc_raw) = document_origin else {
            return DownloadPolicy::BlobFallbackRequired;
        };

        let Some(doc_origin) = parse_document_origin(doc_raw) else {
            return DownloadPolicy::BlobFallbackRequired;
        };

        if origins_match(&href_origin, &doc_origin) {
            DownloadPolicy::Native
        } else {
            DownloadPolicy::BlobFallbackRequired
        }
    } else {
        DownloadPolicy::NoDownloadHint
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedOrigin {
    scheme: String,
    host: String,
    port: u16,
}

const fn default_http_port(scheme: &str) -> u16 {
    if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    }
}

fn origins_match(a: &ParsedOrigin, b: &ParsedOrigin) -> bool {
    a.scheme.eq_ignore_ascii_case(&b.scheme)
        && a.host.eq_ignore_ascii_case(&b.host)
        && a.port == b.port
}

fn parse_document_origin(origin: &str) -> Option<ParsedOrigin> {
    let trimmed = origin.trim();

    let scheme_sep = trimmed.find("://")?;
    let scheme = trimmed[..scheme_sep].to_string();

    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }

    let after_scheme = &trimmed[scheme_sep + 3..];

    let authority_end = after_scheme
        .find(|c| ['/', '?', '#'].contains(&c))
        .unwrap_or(after_scheme.len());

    let authority = after_scheme.get(..authority_end)?;

    parse_authority(scheme.as_str(), authority)
}

fn parse_http_https_origin(href: &str) -> Option<ParsedOrigin> {
    let (scheme, rest) = if starts_with_ignore_case(href.as_bytes(), b"https://") {
        ("https", href.get(8..)?)
    } else if starts_with_ignore_case(href.as_bytes(), b"http://") {
        ("http", href.get(7..)?)
    } else {
        return None;
    };

    let authority_end = rest
        .find(|c| ['/', '?', '#'].contains(&c))
        .unwrap_or(rest.len());

    let authority = rest.get(..authority_end)?;

    parse_authority(scheme, authority)
}

fn parse_authority(scheme: &str, authority: &str) -> Option<ParsedOrigin> {
    if authority.is_empty() {
        return None;
    }

    if let Some(rest) = authority.strip_prefix('[') {
        let end = rest.find(']')?;

        let host = rest.get(..end)?.to_string();

        let after = rest.get(end + 1..).unwrap_or("");

        let port = if let Some(port_raw) = after.strip_prefix(':') {
            port_raw.parse::<u16>().ok()?
        } else {
            default_http_port(scheme)
        };

        return Some(ParsedOrigin {
            scheme: scheme.to_string(),
            host,
            port,
        });
    }

    if let Some((host, port_raw)) = authority.rsplit_once(':')
        && let Ok(port) = port_raw.parse::<u16>()
        && !host.is_empty()
        && !host.contains(':')
    {
        return Some(ParsedOrigin {
            scheme: scheme.to_string(),
            host: host.to_ascii_lowercase(),
            port,
        });
    }

    Some(ParsedOrigin {
        scheme: scheme.to_string(),
        host: authority.to_ascii_lowercase(),
        port: default_http_port(scheme),
    })
}

fn is_relative_reference(url: &str) -> bool {
    let bytes = url.as_bytes();

    if bytes.is_empty() {
        return false;
    }

    if bytes[0] == b'/'
        || bytes[0] == b'#'
        || bytes[0] == b'?'
        || starts_with_ignore_case(bytes, b"./")
        || starts_with_ignore_case(bytes, b"../")
    {
        return true;
    }

    !contains_scheme_separator_before_delim(bytes)
}

/// Returns true when a `:` appears before the first `/`, `?`, or `#`, treating
/// that as an absolute URL with a scheme.
fn contains_scheme_separator_before_delim(bytes: &[u8]) -> bool {
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if byte == b'/' || byte == b'?' || byte == b'#' {
            return false;
        }

        if byte == b':' {
            return true;
        }

        index += 1;
    }

    false
}

const fn starts_with_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    let mut i = 0;

    while i < needle.len() {
        let h = haystack[i];
        let n = needle[i];

        let h_lower = if h >= b'A' && h <= b'Z' { h + 32 } else { h };

        if h_lower != n {
            return false;
        }

        i += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    use ars_core::HasId;
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn en_locale() -> Locale {
        ars_i18n::locales::en_us()
    }

    fn api(props: Props) -> Api {
        Api::new(props, en_locale(), Messages::default())
    }

    #[test]
    fn props_default_matches_spec() {
        let p = Props::default();
        assert_eq!(p.id, "");
        assert_eq!(p.href, "");
        assert!(p.filename.is_none());
        assert!(p.mime_type.is_none());
        assert!(!p.disabled);
        assert!(p.document_origin.is_none());
    }

    #[test]
    fn props_builder_round_trips() {
        let p = Props::new()
            .id("dl-1")
            .href("/files/a.pdf")
            .filename("save-as.pdf")
            .mime_type("application/pdf")
            .disabled(true)
            .document_origin("https://example.com");

        assert_eq!(p.id, "dl-1");
        assert_eq!(p.href, "/files/a.pdf");
        assert_eq!(p.filename.as_deref(), Some("save-as.pdf"));
        assert_eq!(p.mime_type.as_deref(), Some("application/pdf"));
        assert!(p.disabled);
        assert_eq!(p.document_origin.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn has_id_derive_round_trips() {
        let mut p = Props::default().with_id(String::from("x"));

        assert_eq!(HasId::id(&p), "x");

        p.set_id(String::from("y"));

        assert_eq!(HasId::id(&p), "y");
    }

    #[test]
    fn same_origin_absolute_https_sets_download_with_filename() {
        let props = Props::new()
            .id("a")
            .href("https://example.com/path/file.bin")
            .filename("out.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Download), Some("out.bin"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-download-fallback")), None);
    }

    #[test]
    fn same_origin_absolute_https_sets_bool_download_without_filename() {
        let props = Props::new()
            .href("https://example.com/x")
            .document_origin("https://example.com");

        let attrs = api(props).root_attrs();

        assert_eq!(
            attrs.get_value(&HtmlAttr::Download),
            Some(&ars_core::AttrValue::Bool(true))
        );
    }

    #[test]
    fn relative_href_sets_native_download_without_document_origin() {
        let props = Props::new().href("/docs/report.pdf").filename("r.pdf");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Href), Some("/docs/report.pdf"));
        assert_eq!(attrs.get(&HtmlAttr::Download), Some("r.pdf"));
    }

    #[test]
    fn mime_type_sets_type_attribute() {
        let props = Props::new().href("/x").mime_type("application/pdf");

        let attrs = api(props).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("application/pdf"));
    }

    #[test]
    fn disabled_keeps_href_and_sets_aria_disabled() {
        let props = Props::new()
            .href("https://ex.com/f")
            .document_origin("https://ex.com")
            .disabled(true);

        let attrs = api(props).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Href), Some("https://ex.com/f"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&ars_core::AttrValue::Bool(true))
        );
    }

    #[test]
    fn cross_origin_omits_download_and_sets_fallback_data_attr() {
        let props = Props::new()
            .href("https://cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Download), None);
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-download-fallback")),
            Some(DOWNLOAD_FALLBACK_REQUIRED)
        );
    }

    #[test]
    fn unknown_document_origin_on_absolute_https_is_conservative() {
        let props = Props::new()
            .href("https://example.com/file.pdf")
            .filename("f.pdf");
        // document_origin None

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn mailto_omits_download_and_fallback_hint() {
        let props = Props::new().href("mailto:user@example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(!api.needs_blob_fallback());

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Download), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-download-fallback")), None);
    }

    #[test]
    fn part_attrs_dispatches_root() {
        let props = Props::new().href("/x");

        let api = api(props);

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    // ── Snapshots ─────────────────────────────────────────────────────

    #[test]
    fn download_trigger_root_same_origin_with_filename_snapshot() {
        assert_snapshot!(
            "download_trigger_root_same_origin_with_filename",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-sf")
                    .href("https://my.app/doc.pdf")
                    .filename("manual.pdf")
                    .document_origin("https://my.app"),)
                .root_attrs(),
            )
        );
    }

    #[test]
    fn download_trigger_root_same_origin_bool_download_snapshot() {
        assert_snapshot!(
            "download_trigger_root_same_origin_bool_download",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-sb")
                    .href("./asset.bin")
                    .document_origin("https://ignored.example"),)
                .root_attrs(),
            )
        );
    }

    #[test]
    fn download_trigger_root_cross_origin_snapshot() {
        assert_snapshot!(
            "download_trigger_root_cross_origin",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-co")
                    .href("https://cdn.example/resource.zip")
                    .filename("bundle.zip")
                    .document_origin("https://app.example"),)
                .root_attrs(),
            )
        );
    }

    #[test]
    fn download_trigger_root_mime_type_snapshot() {
        assert_snapshot!(
            "download_trigger_root_mime_type",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-mt")
                    .href("/export.csv")
                    .mime_type("text/csv"),)
                .root_attrs(),
            )
        );
    }

    #[test]
    fn download_trigger_root_disabled_snapshot() {
        assert_snapshot!(
            "download_trigger_root_disabled",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-dis")
                    .href("https://ex.org/a.png")
                    .document_origin("https://ex.org")
                    .disabled(true),)
                .root_attrs(),
            )
        );
    }

    #[test]
    fn download_trigger_root_unknown_origin_https_snapshot() {
        assert_snapshot!(
            "download_trigger_root_unknown_origin_https",
            snapshot_attrs(
                &api(Props::new()
                    .id("dl-uo")
                    .href("https://static.dev/file.tgz")
                    .filename("archive.tgz"),)
                .root_attrs(),
            )
        );
    }
}
