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
#[derive(Clone, Debug, PartialEq, HasId)]
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
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable strings. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            href: String::new(),
            filename: None,
            mime_type: None,
            disabled: false,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "download-trigger"]
pub enum Part {
    Root,
}

/// API for the `DownloadTrigger` component.
pub struct Api<'a> {
    props: &'a Props,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    pub fn new(props: &'a Props) -> Self {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        Self { props, locale, messages }
    }

    /// Root element attributes. Returns attrs suitable for an `<a>` element.
    ///
    /// When the href is same-origin, uses native `<a download="filename">`.
    /// When cross-origin (where the `download` attribute is ignored by browsers),
    /// the adapter should attach a click handler that fetches the resource as a
    /// blob and triggers download via `URL.createObjectURL`.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, &self.props.id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Href, &self.props.href);

        // Set the download attribute with optional filename.
        match &self.props.filename {
            Some(name) => attrs.set(HtmlAttr::Download, name),
            None       => attrs.set_bool(HtmlAttr::Download, true),
        }

        // Apply accessible label from Messages.
        // Consumer-provided aria-label takes precedence via AttrMap merge.
        let label = (self.messages.download_label)(
            self.props.filename.as_deref().unwrap_or(""),
            &self.locale,
        );
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);

        if self.props.disabled {
            // Use aria-disabled="true" with href preserved so the element
            // remains focusable for screen reader discoverability. The adapter
            // must prevent default on click when aria-disabled is true.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Whether the trigger is disabled.
    pub fn is_disabled(&self) -> bool {
        self.props.disabled
    }

    /// The resolved filename for the download.
    pub fn filename(&self) -> Option<&str> {
        self.props.filename.as_deref()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
DownloadTrigger
└── Root  <a>  download attribute
```

| Part | Element | Key Attributes                                                          |
| ---- | ------- | ----------------------------------------------------------------------- |
| Root | `<a>`   | `data-ars-scope="download-trigger"`, `data-ars-part="root"`, `download` |

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
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the download trigger (e.g., "Download file").
    pub download_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            download_label: MessageFn::new(|filename, _locale| {
                if filename.is_empty() {
                    "Download file".to_string()
                } else {
                    format!("Download {}", isolate_text_safe(filename))
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

> **SSR safety:** The cross-origin download fallback handler MUST only be attached on the client. During SSR, render the `<a>` element with `href` and `download` attributes only — the fallback handler is attached after hydration via `use_effect`.
>
> **Platform Note:** The cross-origin download fallback using `fetch` + `Blob` + `URL.createObjectURL` works in browsers and Dioxus Web. On Dioxus Desktop, downloads should use the native file system APIs. On Dioxus Mobile, download behavior is OS-dependent. The adapter should abstract the download mechanism.
>
> **Adapter Note:** Blob URLs created for cross-origin downloads must be revoked on unmount via `URL.revokeObjectURL`. Leptos uses `on_cleanup`; Dioxus uses `use_drop`.
>
> **Dioxus multi-platform:** The `fetch()` + `Blob` + `URL.createObjectURL()` fallback is web-only. Gate with `#[cfg(feature = "web")]`. On Dioxus Desktop/Mobile, use platform-specific download APIs (e.g., `rfd::AsyncFileDialog` for native file save dialogs or direct filesystem writes).

## 6. Rendering Semantics

`DownloadTrigger` is a thin wrapper component — a pure rendering component with no state machine:

- **Renders an `<a>` element** with the `download` attribute. This is the only DOM element produced.
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

| Feature   | ars-ui      | Ark UI                | Notes                                         |
| --------- | ----------- | --------------------- | --------------------------------------------- |
| href      | `href`      | (via native `<a>`)    | Both libraries                                |
| Filename  | `filename`  | (via `download` attr) | Both libraries                                |
| MIME type | `mime_type` | --                    | ars-ui addition for cross-origin content-type |

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
