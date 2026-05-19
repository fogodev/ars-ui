---
component: DownloadTrigger
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: DownloadTrigger
---

# DownloadTrigger

A stateless utility component that initiates file downloads via the browser download API. Wraps a clickable element (typically `<a>` or `<button>`) with the correct attributes to trigger a download. Matches Ark UI's `DownloadTrigger` pattern.

## 1. API

### 1.1 Props

```rust
/// Props for the `DownloadTrigger` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// URL of the file to download. Can be an absolute URL, relative path,
    /// data URI, or blob URL.
    pub href: String,
    /// Optional filename for the downloaded file. When provided, the browser
    /// uses this as the suggested filename. When `None`, the browser infers
    /// the filename from the URL.
    pub filename: Option<String>,
    /// MIME type hint for the download. Used by the fallback JS path when
    /// the native `<a download>` approach isn't viable (e.g., cross-origin).
    pub mime_type: Option<String>,
    /// Disabled state. When true, the trigger is visually and functionally disabled.
    pub disabled: bool,
    /// Adapter-supplied document origin (`scheme://host[:port]`, e.g.
    /// `https://example.com`). Used only to classify absolute `http`/`https`
    /// URLs for native `download` vs blob-fetch fallback; relative URLs ignore
    /// this field.
    pub document_origin: Option<String>,
}

impl Props {
    /// Returns fresh props with the documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // … builder methods (`id`, `href`, `filename`, …) mirror the implementation.
}
```

### 1.2 Connect / API

```rust
/// Stable `data-ars-download-fallback` attribute value when blob-fetch fallback is required.
pub const DOWNLOAD_FALLBACK_REQUIRED: &str = "required";

#[derive(ComponentPart)]
#[scope = "download-trigger"]
pub enum Part {
    Root,
}

/// Dynamic callable signature for [`Messages::download_label`].
pub type DownloadLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// API for the `DownloadTrigger` component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    pub fn new(props: Props, locale: Locale, messages: Messages) -> Self;

    pub fn props(&self) -> &Props;

    /// Root `<a>` attributes — includes `type` when [`Props::mime_type`] is set.
    ///
    /// Emits the native `download` attribute when [`Self::native_download_eligible`]
    /// is true (relative URLs, `blob:` / `data:`, or same-origin absolute `http`/`https`
    /// against [`Props::document_origin`]).
    ///
    /// When [`Self::needs_blob_fallback`] is true, omits `download` and sets
    /// `data-ars-download-fallback="required"` for adapter-side fetch/blob handling.
    ///
    /// When disabled, keeps `href`, sets `aria-disabled="true"` and
    /// `data-ars-disabled="true"`; the adapter must prevent default on activation.
    pub fn root_attrs(&self) -> AttrMap;

    pub fn native_download_eligible(&self) -> bool;
    pub fn needs_blob_fallback(&self) -> bool;
    pub fn is_disabled(&self) -> bool;
    pub fn filename(&self) -> Option<&str>;
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

### 1.3 HTTP(S) URL classification

For native `download` vs blob-fetch fallback, the core trims leading and trailing ASCII whitespace on `href` before parsing. **Absolute** `http`/`https` URLs are recognized when **at least two** leading `/` or `\` bytes follow the scheme (so `https:\\host` is absolute, not same-document-relative); additional slashes/backslashes are trimmed before authority extraction. Forms such as `https:foo` or `https:/foo` whose scheme matches [`Props::document_origin`] stay native-download eligible when they do **not** open an authority that way. Scheme-relative URLs use either `//authority` or `\\authority` (including excess slashes such as `///authority`), take the **scheme** from `document_origin`, and compare **host** and **port** like absolute URLs. An explicit colon with an empty port segment on the authority (`https://example.com:/path`) uses the scheme default port. Authority scanning treats `\` like `/` before `/`, `?`, or `#`; credentials (`userinfo@`) are stripped before host comparison. Bracketed IPv6 authorities must end the host before `]` except for an optional `:port`; junk after `]` is rejected (conservative `NoDownloadHint`). Host equality uses IDNA/punycode normalization plus inet-style IPv4 forms (including legacy leading-zero octal / `0x` hex pieces per WHATWG-style parsing). When the document origin uses a different HTTP(S) scheme than the href, spellings such as `http:host/path` on an `https` page are still classified as HTTP(S) authorities for origin comparison. Single-segment path URLs (`/report.pdf`, `./x`, …) do not use `document_origin`. Bare `//` or `\\` without a usable authority needs blob fallback when `document_origin` is present.

## 2. Anatomy

```text
DownloadTrigger
└── Root  <a>  download attribute
```

| Part | Element | Key Attributes                                                                                                                                                                                                                                                                      |
| ---- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root | `<a>`   | `data-ars-scope="download-trigger"`, `data-ars-part="root"`, `download` when same-origin / relative / `blob:` / `data:`; `data-ars-download-fallback="required"` when blob fallback is needed for HTTP(S) cross-origin or unknown document origin; optional `type` from `mime_type` |

**1 part total.** A single anchor element with the `download` attribute.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **Native `<a>` semantics**: Uses `<a href="..." download="...">` which is natively accessible. Screen readers announce it as a link. No additional `role` needed.
- **`aria-disabled`**: Set when `disabled` is true. The href is preserved so the element remains focusable for screen reader discoverability; the adapter prevents default on click instead of removing the href.
- **Screen reader label**: The `download_label` message provides a descriptive label when the trigger contains only an icon. Consumers should set `aria-label` using this message.
- When rendered via `as_child` onto a non-anchor element (e.g., `<div>` or `<span>`), the implicit link role is lost. The adapter MUST set `role="link"` explicitly on non-anchor elements to maintain screen reader discoverability.

## 4. Internationalization

### 4.1 Messages

```rust
pub type DownloadLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the download trigger (e.g., "Download file").
    pub download_label: MessageFn<DownloadLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            download_label: MessageFn::new(|filename, _locale| {
                if filename.is_empty() {
                    "Download file".to_string()
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
```

Per foundation/04-internationalization.md §2.1.1, dynamic values interpolated into localized messages MUST use `isolate_text_safe()` to prevent BiDi reordering when the filename contains characters from a different script direction.

## 5. Cross-Origin Download Fallback

When downloading cross-origin resources, the HTML `download` attribute is ignored by browsers (per the HTML spec security model). The adapter layer should detect this case and attach a click handler that:

1. Calls `fetch()` on the href.
2. Creates a `Blob` from the response.
3. Creates an object URL via `URL.createObjectURL()`.
4. Programmatically creates a temporary `<a>` with the blob URL and `download` attribute.
5. Clicks the temporary anchor and revokes the object URL.

This pattern is adapter-specific (requires DOM access) and is NOT implemented in the headless core.

> **SSR safety:** The cross-origin download fallback handler MUST only be attached on the client. During SSR, render `href` plus native `download` only when the headless core classifies the URL as native-download eligible; when `data-ars-download-fallback="required"` is present, omit `download` — attach the fallback handler after hydration via `use_effect`.
>
> **Platform Note:** The cross-origin download fallback using `fetch` + `Blob` + `URL.createObjectURL` works in browsers and Dioxus Web. On Dioxus Desktop, downloads should use the native file system APIs. On Dioxus Mobile, download behavior is OS-dependent. The adapter should abstract the download mechanism.
>
> **Adapter Note:** Blob URLs created for cross-origin downloads must be revoked on unmount via `URL.revokeObjectURL`. Leptos uses `on_cleanup`; Dioxus uses `use_drop`.
>
> **Dioxus multi-platform:** The `fetch()` + `Blob` + `URL.createObjectURL()` fallback is web-only. Gate with `#[cfg(feature = "web")]`. On Dioxus Desktop/Mobile, use platform-specific download APIs (e.g., `rfd::AsyncFileDialog` for native file save dialogs or direct filesystem writes).

## 6. Rendering Semantics

`DownloadTrigger` is a thin wrapper component — a pure rendering component with no state machine:

- **Renders an `<a>` element** with `href`. Emits the `download` attribute when native same-origin / relative / `blob:` / `data:` rules apply; omits `download` and emits `data-ars-download-fallback="required"` when HTTP(S) blob-fetch fallback is needed.
- If `filename` is provided, the element renders as `<a href="..." download="filename">`. If `filename` is `None`, renders as `<a href="..." download>` (browser infers the filename from the URL).
- **No state machine needed** — `DownloadTrigger` has no internal states, events, or transitions. It is a direct `Props`-to-`AttrMap` mapping.
- **Blob URL cleanup**: If `href` is a blob URL (starts with `blob:`), the adapter MUST call `URL.revokeObjectURL(href)` on component unmount to release the underlying memory. This is handled in the adapter's cleanup/dispose lifecycle, not in the headless core.

Adapters MUST clean up blob URLs on unmount to prevent memory leaks:

- **Leptos:** `on_cleanup(move || { web_sys::Url::revoke_object_url(&blob_url).ok(); })`
- **Dioxus:** `use_drop(move || { web_sys::Url::revoke_object_url(&blob_url).ok(); })`
- **Accessibility**: Inherits native `<a>` link semantics. Screen readers announce it as a link. Consumers SHOULD include "download" in the accessible label (e.g., `aria-label="Download report.pdf"`) to distinguish download links from navigation links.

## 7. Library Parity

> Compared against: Ark UI (`DownloadTrigger`).

### 7.1 Props

| Feature         | ars-ui            | Ark UI                | Notes                                                                    |
| --------------- | ----------------- | --------------------- | ------------------------------------------------------------------------ |
| href            | `href`            | (via native `<a>`)    | Both libraries                                                           |
| Filename        | `filename`        | (via `download` attr) | Both libraries                                                           |
| MIME type       | `mime_type`       | --                    | ars-ui addition for cross-origin content-type                            |
| Document origin | `document_origin` | --                    | Adapter-supplied `scheme://host[:port]` for HTTP(S) download eligibility |

**Gaps:** None.

### 7.2 Anatomy

| Part | ars-ui         | Ark UI         | Notes          |
| ---- | -------------- | -------------- | -------------- |
| Root | `Root` (`<a>`) | `Root` (`<a>`) | Both libraries |

**Gaps:** None.

### 7.3 Features

| Feature                   | ars-ui             | Ark UI |
| ------------------------- | ------------------ | ------ |
| Native download attribute | Yes                | Yes    |
| Cross-origin fallback     | Yes (fetch + blob) | --     |
| Blob URL cleanup          | Yes                | --     |

**Gaps:** None.

### 7.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui adds cross-origin download support via fetch/blob fallback and automatic blob URL cleanup. Ark's DownloadTrigger is a thin `<a download>` wrapper.
- **Recommended additions:** None.
