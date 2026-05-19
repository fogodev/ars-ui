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
//!
//! Leading and trailing WHATWG “C0 control or space” characters (Unicode scalar
//! values U+0000 through U+0020 inclusive) in `href` and `document_origin` are
//! stripped before classification, and embedded ASCII tab/newline characters are
//! removed — Unicode whitespace such as NBSP is **not** trimmed. Scheme-relative
//! references (`//authority/path`) use the document origin's scheme; credentials in
//! the authority (`userinfo@host`) are stripped before host comparison.
//!
//! Delimiters `/`, `?`, `#`, and `\` terminate HTTP(S) authority scanning here so
//! classification aligns with browsers that rewrite `\` as `/` for special schemes.
//!
//! Hostnames are percent-decoded (`%HH`) before IPv4 / IDNA normalization so encoded
//! ASCII labels match their decoded form.

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::net::{Ipv4Addr, Ipv6Addr};

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

/// Trims WHATWG URL preprocessing whitespace only (C0 controls U+0000–U+001F and U+0020 SPACE).
///
/// This deliberately does **not** apply Rust's Unicode-aware [`str::trim`], which would strip
/// characters such as NBSP (U+00A0) that browsers keep when resolving relative URLs.
fn trim_url_c0_control_or_space(input: &str) -> &str {
    input.trim_matches(|ch| ch <= '\u{0020}')
}

fn preprocess_url_input(input: &str) -> Cow<'_, str> {
    let trimmed = trim_url_c0_control_or_space(input);

    if !trimmed
        .as_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\t' | b'\n' | b'\r'))
    {
        return Cow::Borrowed(trimmed);
    }

    let mut out = String::with_capacity(trimmed.len());

    for ch in trimmed.chars() {
        if !matches!(ch, '\t' | '\n' | '\r') {
            out.push(ch);
        }
    }

    Cow::Owned(out)
}

fn classify_href(href: &str, document_origin: Option<&str>) -> DownloadPolicy {
    let preprocessed = preprocess_url_input(href);
    let trimmed = preprocessed.as_ref();

    if trimmed.is_empty() {
        return DownloadPolicy::NoDownloadHint;
    }

    if is_same_scheme_special_relative_http_https(trimmed, document_origin) {
        return DownloadPolicy::Native;
    }

    if let Some(href_origin) = parse_http_https_origin(trimmed, document_origin) {
        return classify_http_https_against_document(&href_origin, document_origin);
    }

    if let Some(href_origin) = parse_network_path_origin(trimmed, document_origin) {
        return classify_http_https_against_document(&href_origin, document_origin);
    }

    // Scheme-relative network-path references without a parsable authority are not path-relative URLs.
    if has_network_path_prefix(trimmed) {
        return DownloadPolicy::BlobFallbackRequired;
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

    DownloadPolicy::NoDownloadHint
}

fn classify_http_https_against_document(
    href_origin: &ParsedOrigin,
    document_origin: Option<&str>,
) -> DownloadPolicy {
    let Some(doc_raw) = document_origin else {
        return DownloadPolicy::BlobFallbackRequired;
    };

    let Some(doc_origin) = parse_document_origin(doc_raw) else {
        return DownloadPolicy::BlobFallbackRequired;
    };

    if origins_match(href_origin, &doc_origin) {
        DownloadPolicy::Native
    } else {
        DownloadPolicy::BlobFallbackRequired
    }
}

fn authorities_partition(rest: &str) -> Option<&str> {
    const HOST_TERMINATORS: [char; 4] = ['/', '?', '#', '\\'];

    let authority_end = rest
        .find(|ch| HOST_TERMINATORS.contains(&ch))
        .unwrap_or(rest.len());

    rest.get(..authority_end)
}

/// Returns the substring after `http(s):` when at least two `/` or `\` leaders open an authority.
fn http_https_authority_rest(after_colon: &str) -> Option<&str> {
    let bytes = after_colon.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() && (bytes[idx] == b'/' || bytes[idx] == b'\\') {
        idx += 1;
    }

    if idx < 2 {
        return None;
    }

    let rest = after_colon.get(idx..)?;

    Some(rest.trim_start_matches(|ch| ['/', '\\'].contains(&ch)))
}

fn leading_url_separator_count(input: &str) -> usize {
    input
        .as_bytes()
        .iter()
        .take_while(|byte| matches!(byte, b'/' | b'\\'))
        .count()
}

fn has_network_path_prefix(input: &str) -> bool {
    leading_url_separator_count(input) >= 2
}

/// Parses scheme-relative references (`//authority/…`) using [`Props::document_origin`]'s scheme.
///
/// Inputs such as `///authority/path` are normalized like browsers: excess slashes after `//`
/// are skipped so an authority token is recovered instead of mis-classifying as a path-relative URL.
fn parse_network_path_origin(trimmed: &str, document_origin: Option<&str>) -> Option<ParsedOrigin> {
    let separator_count = leading_url_separator_count(trimmed);

    if separator_count < 2 {
        return None;
    }

    let rest = trimmed.get(separator_count..)?;

    if rest.is_empty() {
        return None;
    }

    let authority = authorities_partition(rest)?;

    if authority.is_empty() {
        return None;
    }

    let doc_raw = document_origin?;
    let doc_origin = parse_document_origin(doc_raw)?;

    parse_authority(doc_origin.scheme.as_str(), authority)
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
        && canonical_host_for_origin_compare(a.host.as_str())
            == canonical_host_for_origin_compare(b.host.as_str())
        && a.port == b.port
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Applies ASCII percent-decoding to `%HH` sequences in a hostname (browser-style).
fn percent_decode_host(host: &str) -> Cow<'_, str> {
    let bytes = host.as_bytes();

    if !bytes.contains(&b'%') {
        return Cow::Borrowed(host);
    }

    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
            continue;
        }

        out.push(bytes[i]);
        i += 1;
    }

    match String::from_utf8(out) {
        Ok(decoded) => Cow::Owned(decoded),
        Err(_) => Cow::Borrowed(host),
    }
}

/// Normalizes hosts the way browsers do before comparing origins (IPv4
/// shorthand, IPv6 text forms, IDNA/punycode).
fn canonical_host_for_origin_compare(host: &str) -> String {
    let host = percent_decode_host(host);
    let host = host.as_ref();

    if let Some(ip) = parse_ipv4_address_relaxed(host) {
        return ip.to_string();
    }

    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        return ip.to_string();
    }

    idna::domain_to_ascii(host).map_or_else(
        |_| host.to_ascii_lowercase(),
        |ascii| ascii.to_ascii_lowercase(),
    )
}

/// inet_aton-style IPv4 parsing for forms browsers accept (for example `127.1`).
fn parse_ipv4_address_relaxed(text: &str) -> Option<Ipv4Addr> {
    if text.is_empty() || text.contains(':') {
        return None;
    }

    if let Ok(ip) = text.parse::<Ipv4Addr>() {
        return Some(ip);
    }

    let parts: Vec<&str> = text.split('.').collect();

    match parts.len() {
        4 => {
            let mut octets = [0_u8; 4];

            for (slot, part) in octets.iter_mut().zip(parts) {
                *slot = parse_ipv4_octet(part)?;
            }

            Some(Ipv4Addr::from(octets))
        }

        3 => {
            let a = parse_ipv4_octet(parts[0])?;
            let b = parse_ipv4_octet(parts[1])?;
            let merged = parse_ipv4_component_value(parts[2])?;

            if merged > u32::from(u16::MAX) {
                return None;
            }

            Some(Ipv4Addr::from(
                u32::from(a) << 24 | u32::from(b) << 16 | merged,
            ))
        }

        2 => {
            let a = parse_ipv4_octet(parts[0])?;
            let merged = parse_ipv4_component_value(parts[1])?;

            if merged > 0xFF_FFFF {
                return None;
            }

            Some(Ipv4Addr::from(u32::from(a) << 24 | merged))
        }

        1 => {
            let val = parse_ipv4_component_value(parts[0])?;

            Some(Ipv4Addr::from(val))
        }

        _ => None,
    }
}

/// Parses a single dotted IPv4 piece using WHATWG-style leading-zero octal and `0x` hex.
fn parse_ipv4_component_value(raw: &str) -> Option<u32> {
    if raw.is_empty() {
        return None;
    }

    if raw.len() >= 2 {
        let head = raw.get(..2)?;
        if head.eq_ignore_ascii_case("0x") {
            return u32::from_str_radix(raw.get(2..)?, 16).ok();
        }
    }

    if raw.starts_with('0') && raw.len() > 1 {
        return u32::from_str_radix(raw, 8).ok();
    }

    raw.parse::<u32>().ok()
}

fn parse_ipv4_octet(raw: &str) -> Option<u8> {
    let value = parse_ipv4_component_value(raw)?;

    if value <= u8::MAX.into() {
        Some(value as u8)
    } else {
        None
    }
}

/// Browser “same-scheme relative” URLs: `https:foo` / `https:/foo` resolve against
/// the document base when the scheme matches `document_origin`.
fn is_same_scheme_special_relative_http_https(href: &str, document_origin: Option<&str>) -> bool {
    let Some(doc_raw) = document_origin else {
        return false;
    };

    let doc_preprocessed = preprocess_url_input(doc_raw);
    let doc_trim = doc_preprocessed.as_ref();

    let Some(scheme_sep) = doc_trim.find("://") else {
        return false;
    };

    let doc_scheme = &doc_trim[..scheme_sep];

    if !doc_scheme.eq_ignore_ascii_case("http") && !doc_scheme.eq_ignore_ascii_case("https") {
        return false;
    }

    let bytes = href.as_bytes();

    let (href_scheme, prefix_len) = if starts_with_ignore_case(bytes, b"https:") {
        ("https", 6)
    } else if starts_with_ignore_case(bytes, b"http:") {
        ("http", 5)
    } else {
        return false;
    };

    if !href_scheme.eq_ignore_ascii_case(doc_scheme) {
        return false;
    }

    let after_scheme = href.get(prefix_len..).unwrap_or("");

    http_https_authority_rest(after_scheme).is_none()
}

fn document_origin_scheme(document_origin: Option<&str>) -> Option<&str> {
    let preprocessed = preprocess_url_input(document_origin?);
    let raw = preprocessed.as_ref();
    let sep = raw.find("://")?;
    let candidate = raw.get(..sep)?;

    if candidate.eq_ignore_ascii_case("http") {
        Some("http")
    } else if candidate.eq_ignore_ascii_case("https") {
        Some("https")
    } else {
        None
    }
}

fn parse_document_origin(origin: &str) -> Option<ParsedOrigin> {
    let preprocessed = preprocess_url_input(origin);
    let trimmed = preprocessed.as_ref();

    let scheme_sep = trimmed.find("://")?;
    let scheme = trimmed[..scheme_sep].to_string();

    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }

    let after_scheme = &trimmed[scheme_sep + 3..];

    let authority_end = after_scheme
        .find(|ch| ['/', '?', '#', '\\'].contains(&ch))
        .unwrap_or(after_scheme.len());

    let authority = after_scheme.get(..authority_end)?;

    parse_authority(scheme.as_str(), authority)
}

fn parse_http_https_origin(href: &str, document_origin: Option<&str>) -> Option<ParsedOrigin> {
    let bytes = href.as_bytes();

    let (scheme, after_colon) = if starts_with_ignore_case(bytes, b"https:") {
        ("https", href.get(6..)?)
    } else if starts_with_ignore_case(bytes, b"http:") {
        ("http", href.get(5..)?)
    } else {
        return None;
    };

    if let Some(rest) = http_https_authority_rest(after_colon) {
        if rest.is_empty() {
            return None;
        }

        let authority = authorities_partition(rest)?;

        return parse_authority(scheme, authority);
    }

    let scheme_mismatch = document_origin_scheme(document_origin)
        .is_none_or(|doc_s| !doc_s.eq_ignore_ascii_case(scheme));

    if !scheme_mismatch {
        return None;
    }

    let tail = after_colon.trim_start_matches(|ch| ['/', '\\'].contains(&ch));

    if tail.is_empty() {
        return None;
    }

    let authority = authorities_partition(tail)?;

    parse_authority(scheme, authority)
}

/// Returns the host/port portion of an authority, dropping `userinfo@` when the
/// `@` is not inside `[...]` (IPv6 literals).
fn authority_without_userinfo(authority: &str) -> &str {
    let mut bracket_depth = 0_i32;
    let mut split_at = None;

    for (index, ch) in authority.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '@' if bracket_depth == 0 => split_at = Some(index),
            _ => {}
        }
    }

    split_at
        .and_then(|idx| authority.get(idx.saturating_add(1)..))
        .filter(|host_port| !host_port.is_empty())
        .unwrap_or(authority)
}

fn parse_authority(scheme: &str, authority: &str) -> Option<ParsedOrigin> {
    let authority = authority_without_userinfo(authority);

    if authority.is_empty() {
        return None;
    }

    if let Some(rest) = authority.strip_prefix('[') {
        let end = rest.find(']')?;

        let host = rest.get(..end)?.to_string();

        let after = rest.get(end + 1..).unwrap_or("");

        let port = if after.is_empty() {
            default_http_port(scheme)
        } else if let Some(port_raw) = after.strip_prefix(':') {
            if port_raw.is_empty() {
                default_http_port(scheme)
            } else {
                port_raw.parse::<u16>().ok()?
            }
        } else {
            return None;
        };

        return Some(ParsedOrigin {
            scheme: scheme.to_string(),
            host,
            port,
        });
    }

    if let Some((host, port_raw)) = authority.rsplit_once(':')
        && !host.is_empty()
        && !host.contains(':')
    {
        if port_raw.is_empty() {
            return Some(ParsedOrigin {
                scheme: scheme.to_string(),
                host: host.to_ascii_lowercase(),
                port: default_http_port(scheme),
            });
        }

        if let Ok(port) = port_raw.parse::<u16>() {
            return Some(ParsedOrigin {
                scheme: scheme.to_string(),
                host: host.to_ascii_lowercase(),
                port,
            });
        }
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
        if has_network_path_prefix(url) {
            return false;
        }

        return true;
    }

    !contains_scheme_separator_before_delim(bytes)
}

/// Returns true when a `:` appears before the first `/`, `?`, or `#` and the preceding
/// bytes form an RFC 3986 `scheme` (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`).
///
/// Colons inside opaque paths such as `:/report.pdf` or `1:2` therefore do **not**
/// force absolute-scheme classification.
fn contains_scheme_separator_before_delim(bytes: &[u8]) -> bool {
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if byte == b'/' || byte == b'?' || byte == b'#' {
            return false;
        }

        if byte == b':' && scheme_prefix_bytes_valid(&bytes[..index]) {
            return true;
        }

        index += 1;
    }

    false
}

fn scheme_prefix_bytes_valid(prefix: &[u8]) -> bool {
    let Some(&first) = prefix.first() else {
        return false;
    };

    if !first.is_ascii_alphabetic() {
        return false;
    }

    prefix[1..]
        .iter()
        .copied()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.')
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
    fn scheme_relative_https_cross_origin_sets_fallback() {
        let props = Props::new()
            .href("//cdn.other.test/asset.dat")
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
    fn scheme_relative_same_origin_sets_native_download() {
        let props = Props::new()
            .href("//example.com/files/x.bin")
            .filename("x.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());

        assert_eq!(api.root_attrs().get(&HtmlAttr::Download), Some("x.bin"));
    }

    #[test]
    fn scheme_relative_uses_document_scheme_http() {
        let props = Props::new()
            .href("//example.com/a")
            .document_origin("http://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
    }

    #[test]
    fn triple_slash_scheme_relative_respects_origin_same_origin_native() {
        let props = Props::new()
            .href("///example.com/path/doc.pdf")
            .filename("doc.pdf")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn triple_slash_scheme_relative_cross_origin_requires_fallback() {
        let props = Props::new()
            .href("///evil.cdn/asset.bin")
            .filename("a.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn https_single_slash_prefix_matches_browser_origin_rules() {
        let props = Props::new()
            .href("https:/example.com/file.bin ")
            .filename("f.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn https_colon_segment_relative_matches_browser_semantics() {
        let props = Props::new()
            .href("https:assets/pkg.tar")
            .filename("pkg.tar")
            .document_origin("https://app.example");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn ipv4_shorthand_host_matches_dotted_decimal_origin() {
        let props = Props::new()
            .href("https://127.1/readme.txt")
            .filename("r.txt")
            .document_origin("https://127.0.0.1");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn percent_encoded_ascii_host_matches_decoded_document_origin() {
        let props = Props::new()
            .href("https://%65xample.com/report.pdf")
            .filename("r.pdf")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn idn_unicode_host_matches_punycode_document_origin() {
        let props = Props::new()
            .href("https://xn--bcher-kva.example/file.bin")
            .filename("f.bin")
            .document_origin("https://bücher.example");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn http_without_slashes_after_scheme_matches_browser_origin_rules() {
        let props = Props::new()
            .href("http:example.com/report.pdf")
            .filename("r.pdf")
            .document_origin("http://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn backslash_splits_https_authority_before_userinfo_normalization() {
        let props = Props::new()
            .href("https://evil.com\\@example.com/trick.bin")
            .filename("t.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn scheme_relative_backslash_splits_authority() {
        let props = Props::new()
            .href("//evil.com\\@example.com/a.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn https_two_backslashes_after_scheme_cross_origin_requires_fallback() {
        let props = Props::new()
            .href("https:\\\\cdn.other.test/asset.zip")
            .filename("bundle.zip")
            .document_origin("https://app.example");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn scheme_relative_two_backslashes_cross_origin_requires_fallback() {
        let props = Props::new()
            .href(r"\\cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn slash_backslash_network_path_cross_origin_requires_fallback() {
        let props = Props::new()
            .href(r"/\cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn backslash_slash_network_path_cross_origin_requires_fallback() {
        let props = Props::new()
            .href(r"\/cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn empty_explicit_port_matches_scheme_default_origin() {
        let props = Props::new()
            .href("https://example.com:/file.bin")
            .filename("f.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn ipv6_bracket_with_illegal_suffix_is_not_native_eligible() {
        let props = Props::new()
            .href("https://[::1]evil.com/file.bin")
            .filename("f.bin")
            .document_origin("https://[::1]");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn scheme_relative_extra_backslashes_normalize_like_browser() {
        let props = Props::new()
            .href(r"\\\example.com/a.bin")
            .filename("a.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn ipv4_legacy_octal_form_is_not_same_origin_as_decimal_octets() {
        let props = Props::new()
            .href("http://0177.1/file.bin")
            .filename("f.bin")
            .document_origin("http://177.0.0.1");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn http_authority_cross_scheme_https_document_requires_fallback() {
        let props = Props::new()
            .href("http:evil.example/phish.bin")
            .filename("p.bin")
            .document_origin("https://trusted.example");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn embedded_ascii_tab_newline_are_removed_before_classification() {
        let props = Props::new()
            .href("ht\ntps://cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn embedded_carriage_return_is_removed_before_classification() {
        let props = Props::new()
            .href("https://exa\rmple.com/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn bare_double_slash_requires_fallback() {
        let props = Props::new()
            .href("//")
            .filename("x.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(!api.native_download_eligible());
        assert!(api.needs_blob_fallback());
    }

    #[test]
    fn trailing_whitespace_on_https_href_still_matches_origin() {
        let props = Props::new()
            .href("https://example.com/path/file.bin ")
            .filename("f.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn leading_nbsp_before_https_is_not_trimmed_stays_native_relative() {
        let props = Props::new()
            .href("\u{00A0}https://cdn.other.test/asset.dat")
            .filename("a.dat")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn colon_only_path_href_is_native_eligible() {
        let props = Props::new()
            .href(":/report.pdf")
            .filename("r.pdf")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn digit_prefixed_colon_segment_href_is_native_eligible() {
        let props = Props::new()
            .href("1:2")
            .filename("out.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
    }

    #[test]
    fn userinfo_on_https_authority_does_not_force_cross_origin() {
        let props = Props::new()
            .href("https://user@example.com/secret.bin")
            .filename("s.bin")
            .document_origin("https://example.com");

        let api = api(props);

        assert!(api.native_download_eligible());
        assert!(!api.needs_blob_fallback());
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
