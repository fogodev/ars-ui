//! Dioxus-specific platform abstraction (`DioxusPlatform`) and its
//! per-target implementations.
//!
//! This module covers operations the framework-agnostic
//! [`ars_core::PlatformEffects`] trait does **not** expose: file pickers,
//! clipboard writes, drag-data extraction, bounding rects, UUID generation,
//! and high-precision timestamps. It is the Dioxus-side analog to the
//! browser-only surface that `ars-leptos` would otherwise inline.
//!
//! The trait flows into adapter components via
//! [`ArsContext::dioxus_platform`](crate::ArsContext#structfield.dioxus_platform);
//! components retrieve the active implementation through [`use_platform`].
//!
//! See `spec/foundation/09-adapter-dioxus.md` §6 for the contract.

use std::{pin::Pin, rc::Rc, sync::Arc, time::Duration};
#[cfg(feature = "desktop")]
use std::{sync::LazyLock, time::Instant};

use ars_core::Rect;
use ars_forms::field::FileRef;
use ars_interactions::{DragItem, FileHandle};
use dioxus::{
    events::{DragData as DioxusDragData, MountedData, ScrollBehavior},
    prelude::try_use_context,
};

use crate::{ArsContext, warn_missing_provider};

/// Options for opening a platform file picker.
///
/// Mirrors the subset of HTML `<input type="file">` configuration that the
/// adapter exposes. Web targets ignore these options because the real file
/// picker is hosted by the `FileUpload` component's hidden `<input>`; desktop
/// targets translate them into `rfd` native-dialog filters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePickerOptions {
    /// Accepted MIME types or extensions. Empty means accept any.
    pub accept: Vec<String>,

    /// Whether the user may select more than one file at once.
    pub multiple: bool,
}

/// Adapter-local drag payload extracted from a Dioxus drag event.
///
/// Mirrors `spec/components/utility/drop-zone.md` §1.3. The `items`
/// vector contains the structured drag payload (text, URI, files,
/// etc.) when the platform exposes it; `types` carries MIME types
/// advertised by the drag source for the formats this adapter is able
/// to detect.
///
/// **What's extracted, by target:**
///
/// - **Web (wasm32 + `feature = "web"`)**: items as `DragItem::File`
///   for each `DataTransfer.files()` entry; `types` populated from the
///   subset of default probe formats for which `data_transfer.get_data()`
///   returns a value.
/// - **Desktop (`feature = "desktop"`)**: items as `DragItem::File`
///   for each path in wry's `DragDropEvent::Drop { paths, .. }`. Each
///   item carries a real `FileHandle::from_path(_)` so the drop-zone
///   adapter can resolve content. `types` is best-effort via the same
///   `get_data()` probe.
/// - **`PlatformDragEvent::empty()`** (any target): both fields empty.
///
/// `dioxus_html::DataTransfer` does not expose a `types()` accessor on
/// the public API (only `get_data(format)`), so the type list is built
/// by probing a fixed set of common formats rather than enumerating
/// the source's full advertised list. Components that need a specific
/// format outside the default probes should call
/// `event.data_transfer().get_data(format)` directly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DragData {
    /// The dragged items.
    ///
    /// May be empty on `DragEnter` / `DragOver` when the browser
    /// restricts access to item content until drop (security
    /// restriction).
    pub items: Vec<DragItem>,

    /// MIME types advertised by the drag source for the subset of
    /// formats the adapter probes. See the type-level docs.
    pub types: Vec<String>,
}

/// Default best-effort formats to probe via `DataTransfer::get_data(format)`
/// when extracting [`DragData::types`].
///
/// Dioxus exposes `get_data(format)` but not an enumerable `types()`
/// API, so the adapter can only discover formats it explicitly asks
/// about. This list is only a fallback heuristic, not an accept/reject
/// policy. Components that care about an exact custom format should
/// call `data_transfer.get_data(...)` directly from their event handler.
const DEFAULT_DRAG_TYPE_PROBES: &[&str] = &[
    "text/plain",
    "text/html",
    "text/uri-list",
    "application/json",
];

impl DragData {
    /// Builds a [`DragData`] from a Dioxus drag event.
    ///
    /// Works on every target Dioxus supports: web (real
    /// `web_sys::DataTransfer`), desktop (`wry`-synthesized data
    /// transfer carrying file paths), SSR (empty data transfer).
    /// Items are populated from `event.data_transfer().files()` as
    /// [`DragItem::File`] entries; `types` is populated by probing
    /// each default probe format — including the synthetic
    /// `"Files"` marker when files are present.
    #[must_use]
    pub fn from_drag_data(event: &DioxusDragData) -> Self {
        let data_transfer = event.data_transfer();

        let mut types = DEFAULT_DRAG_TYPE_PROBES
            .iter()
            .filter(|&format| data_transfer.get_data(format).is_some())
            .map(|&format| String::from(format))
            .collect::<Vec<_>>();

        let items = data_transfer
            .files()
            .into_iter()
            .map(|file| {
                let mime_type = file
                    .content_type()
                    .unwrap_or_else(|| String::from("application/octet-stream"));

                // `FileData::path()` returns `PathBuf::new()` on web (no
                // native path available); on desktop it carries the real
                // path forwarded from `wry::DragDropEvent::Drop { paths }`.
                let raw_path = file.path();

                let handle = if raw_path.as_os_str().is_empty() {
                    FileHandle::opaque()
                } else {
                    FileHandle::from_path(raw_path)
                };

                DragItem::File {
                    name: file.name(),
                    mime_type,
                    size: file.size(),
                    handle,
                }
            })
            .collect::<Vec<_>>();

        // The HTML drag-and-drop spec exposes the synthetic "Files"
        // type whenever the transfer carries one or more files. We
        // synthesise that marker locally so the adapter behaves
        // identically across platforms regardless of how `get_data()`
        // is implemented.
        if !items.is_empty() {
            types.push(String::from("Files"));
        }

        Self { items, types }
    }
}

/// Platform-agnostic handle to a drag event.
///
/// Adapter glue wraps the framework's drag event in a
/// `PlatformDragEvent` before passing it to
/// [`DioxusPlatform::create_drag_data`]. The wrapper holds a borrowed
/// reference to the unified `dioxus::events::DragData` (via
/// [`Self::from_dioxus`]) — that abstraction works identically on web
/// (real browser event) and desktop (wry-synthesized event). On any
/// target an "empty" wrapper is constructible (via [`Self::empty`])
/// for tests and for components that compile against the trait but
/// never produce real drag data.
///
/// `Copy` is derivable because the wrapped `Option<&T>` is `Copy`,
/// letting callers pass the same wrapper to multiple platforms or
/// helpers without clones.
///
/// This replaces an earlier `&dyn Any` parameter, which silently
/// returned `None` if a caller passed the wrong type. With the typed
/// constructor, the conversion happens at the call site where the
/// type information is still in scope.
#[derive(Clone, Copy, Debug)]
pub struct PlatformDragEvent<'a> {
    inner: Option<&'a DioxusDragData>,
}

impl<'a> PlatformDragEvent<'a> {
    /// Constructs a payload-less drag event.
    ///
    /// Used by tests, by components that must call
    /// [`DioxusPlatform::create_drag_data`] without a real Dioxus
    /// event, and as the natural shape for SSR / null platforms that
    /// do not produce drag events.
    #[must_use]
    pub const fn empty() -> Self {
        Self { inner: None }
    }

    /// Wraps a Dioxus `DragData` (the platform-agnostic event payload
    /// `dioxus_html` exposes from `ondrop` / `ondragover` handlers)
    /// for [`DioxusPlatform::create_drag_data`].
    #[must_use]
    pub const fn from_dioxus(event: &'a DioxusDragData) -> Self {
        Self { inner: Some(event) }
    }

    /// Returns the underlying [`DioxusDragData`] reference if the
    /// wrapper was built from one, otherwise `None`.
    #[must_use]
    pub const fn as_dioxus(&self) -> Option<&'a DioxusDragData> {
        self.inner
    }
}

/// Dioxus-specific platform services not covered by core
/// [`PlatformEffects`](ars_core::PlatformEffects).
///
/// Implementations cover three deployment shapes (web / desktop / null);
/// see `WebPlatform`, `DesktopPlatform`, and [`NullPlatform`]. (`WebPlatform`
/// and `DesktopPlatform` are only in scope under their respective feature
/// gates, so we render them as plain code rather than intra-doc links.)
///
/// **Note on `Send` bounds.** The futures returned by async methods are
/// `!Send` on WASM (where `web_sys::JsFuture` cannot cross threads). On
/// desktop, Dioxus's runtime can run `!Send` futures on the current thread
/// via `dioxus::spawn`, so the trait keeps a single uniform return type
/// across platforms. Callers on desktop runtimes that require `Send` should
/// route the future through `dioxus::spawn` or convert it themselves.
pub trait DioxusPlatform: Send + Sync + 'static {
    /// Focuses a mounted element through Dioxus's renderer-backed element
    /// handle.
    ///
    /// This is the portable element-focus path for Dioxus renderers. Web,
    /// desktop, and future mobile renderers expose element operations through
    /// the [`MountedData`] value emitted by `onmounted`, not through a global
    /// DOM ID lookup.
    fn focus_mounted_element(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async move { element.set_focus(true).await.map_err(|err| err.to_string()) })
    }

    /// Returns the viewport-relative bounding rectangle for a mounted
    /// element through Dioxus's renderer-backed element handle.
    ///
    /// Returns `None` when the renderer does not support geometry queries or
    /// when the element is no longer mounted.
    fn get_mounted_bounding_rect(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Option<Rect>>>> {
        Box::pin(async move { element.get_client_rect().await.ok().map(rect_from_pixels) })
    }

    /// Scrolls a mounted element into view through Dioxus's renderer-backed
    /// element handle.
    fn scroll_mounted_into_view(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async move {
            element
                .scroll_to(ScrollBehavior::Instant)
                .await
                .map_err(|err| err.to_string())
        })
    }

    /// Writes text to the system clipboard. The future resolves with `Ok`
    /// on success or an error message on failure.
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>>;

    /// Opens a native file picker. The future resolves with the user's
    /// selection, or an empty vector if the user cancelled or the platform
    /// does not implement a picker (e.g., web defers to a hidden `<input>`).
    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>>;

    /// Returns a monotonically non-decreasing duration since a
    /// platform-defined start point.
    ///
    /// The start point is implementation-defined and intentionally opaque
    /// (web: page-load via `performance.now()`; desktop: UNIX epoch via
    /// `SystemTime`; null: always zero). Callers MUST use this only for
    /// relative measurements — subtract two values from the same platform
    /// instance to get an elapsed [`Duration`]. Comparing values across
    /// platforms or across processes is undefined.
    ///
    /// Returning [`Duration`] (rather than `f64`/`u64` ms) keeps the unit
    /// in the type and forbids accidental epoch-leak bugs.
    fn monotonic_now(&self) -> Duration;

    /// Generates a platform-scoped unique ID.
    ///
    /// Web returns a `UUIDv4` from `crypto.randomUUID()`; desktop returns a
    /// `uuid::Uuid::new_v4()`; the null implementation returns a sequential
    /// counter prefixed with `null-id-`.
    fn new_id(&self) -> String;

    /// Extracts adapter drag data from a platform-specific drag event.
    ///
    /// Returns `None` when the wrapper does not carry a Dioxus drag event.
    /// Callers wrap the framework's drag event in [`PlatformDragEvent`] before
    /// invoking this — the wrapper enforces the underlying event type at
    /// compile time.
    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData>;
}

/// No-op platform implementation used in tests, SSR, and `mobile` builds
/// until a dedicated mobile platform is added.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullPlatform;

impl DioxusPlatform for NullPlatform {
    fn focus_mounted_element(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn get_mounted_bounding_rect(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Option<Rect>>>> {
        Box::pin(async { None })
    }

    fn scroll_mounted_into_view(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn open_file_picker(
        &self,
        _options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        Box::pin(async { Vec::new() })
    }

    fn monotonic_now(&self) -> Duration {
        Duration::ZERO
    }

    fn new_id(&self) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        format!("null-id-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn create_drag_data(&self, _event: PlatformDragEvent<'_>) -> Option<DragData> {
        None
    }
}

/// Web platform implementation backed by `web_sys`.
///
/// Only exists on `wasm32` targets even when the `web` feature is enabled —
/// every method invokes browser APIs that have no meaningful native
/// fallback. On non-wasm hosts (e.g., `cargo check --features web` on a
/// developer's Linux box, or a misconfigured production build), the type
/// is *not in scope* and [`default_dioxus_platform`] falls through to
/// [`NullPlatform`] instead. This keeps build tooling green without
/// silently degrading runtime behaviour into "clipboard writes succeed but
/// do nothing" surprises.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct WebPlatform;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl DioxusPlatform for WebPlatform {
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let text = text.to_string();

        Box::pin(async move {
            let window = web_sys::window().ok_or_else(|| String::from("no window available"))?;

            let promise = window.navigator().clipboard().write_text(&text);

            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map(|_| ())
                .map_err(|err| format!("{err:?}"))
        })
    }

    fn open_file_picker(
        &self,
        _options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        // Web defers to the FileUpload component's hidden <input type="file">.
        // See spec/foundation/09-adapter-dioxus.md §6.1, line 1561.
        Box::pin(async { Vec::new() })
    }

    fn monotonic_now(&self) -> Duration {
        let millis = web_sys::window()
            .and_then(|window| window.performance())
            .map(|performance| performance.now())
            .expect("window.performance must be available on web targets");

        // `performance.now()` returns a non-negative `f64` of milliseconds
        // since page load, with sub-millisecond resolution. Convert via
        // `from_secs_f64(millis / 1000.0)` to preserve fractional precision
        // (`Duration::from_millis` would truncate to integer ms).
        Duration::from_secs_f64(millis / 1000.0)
    }

    fn new_id(&self) -> String {
        web_sys::window()
            .map(|window| {
                window
                    .crypto()
                    .expect("window.crypto must be available on web targets")
                    .random_uuid()
            })
            .expect("window must be available on web targets")
    }

    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData> {
        event.as_dioxus().map(DragData::from_drag_data)
    }
}

const fn rect_from_pixels(rect: dioxus::html::geometry::PixelsRect) -> Rect {
    Rect {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}

/// Desktop platform implementation backed by native crates
/// (`arboard` for clipboard, `rfd` for file dialogs, `uuid` for IDs,
/// `std::time::Instant` for monotonic timestamps).
///
/// Mounted-element focus, geometry, and scrolling use the trait's
/// [`MountedData`]-backed defaults. Drag-data extraction uses Dioxus's unified
/// `events::DragData` payload, which desktop synthesizes from native drag
/// events.
#[cfg(feature = "desktop")]
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopPlatform;

#[cfg(feature = "desktop")]
impl DioxusPlatform for DesktopPlatform {
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let text = text.to_string();

        Box::pin(async move {
            let mut clipboard = arboard::Clipboard::new().map_err(|err| err.to_string())?;

            clipboard.set_text(&text).map_err(|err| err.to_string())
        })
    }

    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new();

            let extensions = file_picker_filter_extensions(&options.accept);

            if !extensions.is_empty() {
                dialog = dialog.add_filter(file_picker_filter_name(&extensions), &extensions);
            }

            let handles = if options.multiple {
                dialog.pick_files().await.unwrap_or_default()
            } else {
                dialog.pick_file().await.into_iter().collect()
            };

            handles
                .into_iter()
                .map(|file| file_ref_from_path(file.path()))
                .collect()
        })
    }

    fn monotonic_now(&self) -> Duration {
        static START: LazyLock<Instant> = LazyLock::new(Instant::now);

        START.elapsed()
    }

    fn new_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData> {
        event.as_dioxus().map(DragData::from_drag_data)
    }
}

#[cfg(feature = "desktop")]
fn file_picker_filter_extensions(accept: &[String]) -> Vec<String> {
    let mut extensions = Vec::new();

    for raw in accept {
        for token in raw
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            for extension in accept_token_extensions(token) {
                if !extensions.iter().any(|existing| existing == &extension) {
                    extensions.push(extension);
                }
            }
        }
    }

    extensions
}

#[cfg(feature = "desktop")]
fn accept_token_extensions(token: &str) -> Vec<String> {
    match token.strip_prefix('.').unwrap_or(token) {
        "txt" => vec![String::from("txt")],

        "html" | "text/html" => vec![String::from("html"), String::from("htm")],

        "css" | "text/css" => vec![String::from("css")],

        "csv" | "text/csv" => vec![String::from("csv")],

        "json" | "application/json" => vec![String::from("json")],

        "pdf" | "application/pdf" => vec![String::from("pdf")],

        "png" | "image/png" => vec![String::from("png")],

        "jpg" | "jpeg" | "image/jpeg" => vec![String::from("jpg"), String::from("jpeg")],

        "gif" | "image/gif" => vec![String::from("gif")],

        "webp" | "image/webp" => vec![String::from("webp")],

        "svg" | "image/svg+xml" => vec![String::from("svg")],

        "image/*" => ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico"]
            .into_iter()
            .map(String::from)
            .collect(),

        "mp3" | "audio/mpeg" => vec![String::from("mp3")],

        "wav" | "audio/wav" => vec![String::from("wav")],

        "ogg" | "audio/ogg" => vec![String::from("ogg")],

        "audio/*" => ["mp3", "wav", "ogg", "flac", "aac"]
            .into_iter()
            .map(String::from)
            .collect(),

        "mp4" | "video/mp4" => vec![String::from("mp4")],

        "webm" | "video/webm" => vec![String::from("webm")],

        "mov" | "video/quicktime" => vec![String::from("mov")],

        "video/*" => ["mp4", "webm", "mov", "avi", "mkv"]
            .into_iter()
            .map(String::from)
            .collect(),

        extension if !extension.contains('/') && !extension.contains('*') => {
            vec![extension.to_ascii_lowercase()]
        }

        _ => Vec::new(),
    }
}

#[cfg(feature = "desktop")]
fn file_picker_filter_name(extensions: &[String]) -> String {
    if extensions.is_empty() {
        String::from("All files")
    } else {
        extensions
            .iter()
            .map(|extension| format!("*.{extension}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(feature = "desktop")]
fn file_ref_from_path(path: &std::path::Path) -> FileRef {
    FileRef {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        size: std::fs::metadata(path).map_or(0, |metadata| metadata.len()),
        mime_type: mime_type_for_path(path),
    }
}

#[cfg(feature = "desktop")]
fn mime_type_for_path(path: &std::path::Path) -> String {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return String::from("application/octet-stream");
    };

    match extension.to_ascii_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
    .to_owned()
}

/// Returns the [`DioxusPlatform`] implementation chosen by feature gating.
///
/// Resolution order:
///
/// 1. `WebPlatform` when `web` is on **and** the build target is wasm32.
/// 2. `DesktopPlatform` when `desktop` is on (and we did not match step 1).
/// 3. [`NullPlatform`] otherwise.
///
/// `web` on a non-wasm host (e.g., `cargo check --features web` from a
/// developer's Linux box) deliberately falls through to step 2 or 3 —
/// `WebPlatform` only exists on wasm32, so this avoids silently shipping
/// "clipboard writes succeed but do nothing" semantics into a misconfigured
/// production build. The `mobile` feature also currently lands at step 3.
///
/// `WebPlatform` and `DesktopPlatform` are rendered as plain code rather
/// than intra-doc links because their types are cfg-gated out of scope on
/// the docs build target.
///
/// This function exists alongside [`use_platform`] for callers that need a
/// platform handle outside a Dioxus render scope (test harnesses, SSR
/// bootstrap, benches). Inside components, prefer [`use_platform`].
#[must_use]
pub fn default_dioxus_platform() -> Arc<dyn DioxusPlatform> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        Arc::new(WebPlatform)
    }

    #[cfg(all(feature = "desktop", not(all(feature = "web", target_arch = "wasm32"))))]
    {
        Arc::new(DesktopPlatform)
    }

    #[cfg(all(
        not(all(feature = "web", target_arch = "wasm32")),
        not(feature = "desktop")
    ))]
    {
        Arc::new(NullPlatform)
    }
}

/// Resolves the active [`DioxusPlatform`] from the surrounding
/// [`ArsProvider`](crate::ArsProvider) context.
///
/// When no [`ArsProvider`](crate::ArsProvider) is mounted, falls back to
/// [`default_dioxus_platform`] using the resolution order
/// **`web` → `desktop` → [`NullPlatform`]**. The `mobile` feature currently
/// falls through to [`NullPlatform`]; when a dedicated mobile platform is
/// added, update [`default_dioxus_platform`] with a
/// `#[cfg(feature = "mobile")]` arm.
#[must_use]
pub fn use_platform() -> Arc<dyn DioxusPlatform> {
    try_use_context::<ArsContext>().map_or_else(
        || {
            warn_missing_provider("use_platform");

            default_dioxus_platform()
        },
        |ctx| Arc::clone(&ctx.dioxus_platform),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        collections::BTreeMap,
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    #[cfg(feature = "desktop")]
    use dioxus::html::SerializedFileData;
    use dioxus::html::{
        DataTransfer as DioxusDataTransfer, HasDataTransferData, HasDragData, HasFileData,
        HasMouseData, InteractionElementOffset, InteractionLocation, Modifiers,
        ModifiersInteraction, MountedError, MountedResult, NativeDataTransfer, PointerInteraction,
        RenderedElementBacking,
        geometry::{
            ClientPoint, Coordinates, ElementPoint, PagePoint, PixelsRect, ScreenPoint,
            euclid::{point2, size2},
        },
        input_data::{MouseButton, MouseButtonSet},
    };

    use super::*;

    #[derive(Clone, Debug, Default)]
    struct TestNativeDataTransfer {
        data: BTreeMap<String, String>,
        #[cfg(feature = "desktop")]
        files: Vec<SerializedFileData>,
    }

    impl TestNativeDataTransfer {
        fn with_data(pairs: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
            Self {
                data: pairs
                    .into_iter()
                    .map(|(key, value)| (String::from(key), String::from(value)))
                    .collect(),
                #[cfg(feature = "desktop")]
                files: Vec::new(),
            }
        }

        #[cfg(feature = "desktop")]
        fn with_files(files: impl IntoIterator<Item = SerializedFileData>) -> Self {
            Self {
                data: BTreeMap::new(),
                files: files.into_iter().collect(),
            }
        }
    }

    impl NativeDataTransfer for TestNativeDataTransfer {
        fn get_data(&self, format: &str) -> Option<String> {
            self.data.get(format).cloned()
        }

        fn set_data(&self, _format: &str, _data: &str) -> Result<(), String> {
            Ok(())
        }

        fn clear_data(&self, _format: Option<&str>) -> Result<(), String> {
            Ok(())
        }

        fn effect_allowed(&self) -> String {
            String::from("all")
        }

        fn set_effect_allowed(&self, _effect: &str) {}

        fn drop_effect(&self) -> String {
            String::from("none")
        }

        fn set_drop_effect(&self, _effect: &str) {}

        fn files(&self) -> Vec<dioxus::html::FileData> {
            #[cfg(not(feature = "desktop"))]
            {
                Vec::new()
            }

            #[cfg(feature = "desktop")]
            self.files
                .iter()
                .cloned()
                .map(dioxus::html::FileData::new)
                .collect()
        }
    }

    #[derive(Clone, Debug)]
    struct TestDragData {
        transfer: TestNativeDataTransfer,
    }

    impl TestDragData {
        fn new(transfer: TestNativeDataTransfer) -> Self {
            Self { transfer }
        }
    }

    impl InteractionLocation for TestDragData {
        fn client_coordinates(&self) -> ClientPoint {
            ClientPoint::new(0.0, 0.0)
        }

        fn screen_coordinates(&self) -> ScreenPoint {
            ScreenPoint::new(0.0, 0.0)
        }

        fn page_coordinates(&self) -> PagePoint {
            PagePoint::new(0.0, 0.0)
        }
    }

    impl InteractionElementOffset for TestDragData {
        fn coordinates(&self) -> Coordinates {
            Coordinates::new(
                self.screen_coordinates(),
                self.client_coordinates(),
                self.element_coordinates(),
                self.page_coordinates(),
            )
        }

        fn element_coordinates(&self) -> ElementPoint {
            ElementPoint::new(0.0, 0.0)
        }
    }

    impl ModifiersInteraction for TestDragData {
        fn modifiers(&self) -> Modifiers {
            Modifiers::empty()
        }
    }

    impl PointerInteraction for TestDragData {
        fn trigger_button(&self) -> Option<MouseButton> {
            None
        }

        fn held_buttons(&self) -> MouseButtonSet {
            MouseButtonSet::empty()
        }
    }

    impl HasMouseData for TestDragData {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl HasFileData for TestDragData {
        fn files(&self) -> Vec<dioxus::html::FileData> {
            Vec::new()
        }
    }

    impl HasDataTransferData for TestDragData {
        fn data_transfer(&self) -> DioxusDataTransfer {
            DioxusDataTransfer::new(self.transfer.clone())
        }
    }

    impl HasDragData for TestDragData {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[derive(Clone, Debug)]
    struct TestMountedBacking {
        focused: Rc<Cell<Option<bool>>>,
        scrolled: Rc<Cell<bool>>,
        rect: Option<PixelsRect>,
    }

    impl TestMountedBacking {
        fn new(rect: Option<PixelsRect>) -> Self {
            Self {
                focused: Rc::new(Cell::new(None)),
                scrolled: Rc::new(Cell::new(false)),
                rect,
            }
        }
    }

    impl RenderedElementBacking for TestMountedBacking {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
            let rect = self.rect;

            Box::pin(async move { rect.ok_or(MountedError::NotSupported) })
        }

        fn scroll_to(
            &self,
            _options: dioxus::html::ScrollToOptions,
        ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
            let scrolled = Rc::clone(&self.scrolled);

            Box::pin(async move {
                scrolled.set(true);
                Ok(())
            })
        }

        fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
            let focused = Rc::clone(&self.focused);

            Box::pin(async move {
                focused.set(Some(focus));
                Ok(())
            })
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct MountedDelegatingPlatform;

    impl DioxusPlatform for MountedDelegatingPlatform {
        fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
            Box::pin(async { Ok(()) })
        }

        fn open_file_picker(
            &self,
            _options: FilePickerOptions,
        ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
            Box::pin(async { Vec::new() })
        }

        fn monotonic_now(&self) -> Duration {
            Duration::ZERO
        }

        fn new_id(&self) -> String {
            String::from("mounted-delegating-platform")
        }

        fn create_drag_data(&self, _event: PlatformDragEvent<'_>) -> Option<DragData> {
            None
        }
    }

    /// Drives a `Pin<Box<dyn Future<…>>>` to completion synchronously,
    /// panicking if the future returns `Pending` (every `NullPlatform`
    /// future is immediately ready).
    fn block_on_ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        let mut context = Context::from_waker(Waker::noop());

        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test future unexpectedly returned Pending"),
        }
    }

    #[test]
    fn null_platform_mounted_element_methods_are_noops() {
        let backing =
            TestMountedBacking::new(Some(PixelsRect::new(point2(1.0, 2.0), size2(3.0, 4.0))));

        let focused = Rc::clone(&backing.focused);

        let scrolled = Rc::clone(&backing.scrolled);

        let element = Rc::new(MountedData::new(backing));

        assert_eq!(
            block_on_ready(NullPlatform.focus_mounted_element(Rc::clone(&element))),
            Ok(())
        );
        assert!(
            block_on_ready(NullPlatform.get_mounted_bounding_rect(Rc::clone(&element))).is_none()
        );
        assert_eq!(
            block_on_ready(NullPlatform.scroll_mounted_into_view(element)),
            Ok(())
        );

        assert_eq!(focused.get(), None);
        assert!(!scrolled.get());
    }

    #[test]
    fn default_mounted_element_methods_delegate_to_mounted_data() {
        let backing =
            TestMountedBacking::new(Some(PixelsRect::new(point2(1.0, 2.0), size2(3.0, 4.0))));

        let focused = Rc::clone(&backing.focused);

        let scrolled = Rc::clone(&backing.scrolled);

        let element = Rc::new(MountedData::new(backing));

        assert_eq!(
            block_on_ready(MountedDelegatingPlatform.focus_mounted_element(Rc::clone(&element))),
            Ok(())
        );

        let rect = block_on_ready(
            MountedDelegatingPlatform.get_mounted_bounding_rect(Rc::clone(&element)),
        )
        .expect("mounted rect");

        assert_eq!(
            block_on_ready(MountedDelegatingPlatform.scroll_mounted_into_view(element)),
            Ok(())
        );

        assert_eq!(focused.get(), Some(true));
        assert!(scrolled.get());
        assert_eq!(
            rect,
            Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            }
        );
    }

    #[test]
    fn null_platform_set_clipboard_returns_ok() {
        assert_eq!(block_on_ready(NullPlatform.set_clipboard("text")), Ok(()));
    }

    #[test]
    fn null_platform_open_file_picker_returns_empty() {
        assert!(
            block_on_ready(NullPlatform.open_file_picker(FilePickerOptions::default())).is_empty()
        );
    }

    #[test]
    fn null_platform_monotonic_now_is_zero() {
        assert_eq!(NullPlatform.monotonic_now(), Duration::ZERO);
    }

    #[test]
    fn null_platform_new_id_is_unique_and_sequential() {
        let first = NullPlatform.new_id();
        let second = NullPlatform.new_id();

        assert!(first.starts_with("null-id-"));
        assert!(second.starts_with("null-id-"));
        assert_ne!(first, second);

        // Suffix monotonicity: the trailing counter on `second` must be
        // strictly greater than the one on `first`. Other tests share the
        // global counter, so we compare relatively rather than to fixed
        // values.
        let suffix = |id: &str| -> usize {
            id.strip_prefix("null-id-")
                .and_then(|s| s.parse::<usize>().ok())
                .expect("null-id should have numeric suffix")
        };

        assert!(suffix(&second) > suffix(&first));
    }

    #[test]
    fn null_platform_create_drag_data_returns_none() {
        // `NullPlatform` ignores the event entirely; an empty wrapper is
        // the cheapest representative on a native test host.
        assert!(
            NullPlatform
                .create_drag_data(PlatformDragEvent::empty())
                .is_none()
        );
    }

    #[test]
    fn file_picker_options_default_has_empty_accept_and_not_multiple() {
        let options = FilePickerOptions::default();

        assert!(options.accept.is_empty());
        assert!(!options.multiple);
    }

    #[test]
    fn drag_data_default_is_empty() {
        let data = DragData::default();

        assert!(data.items.is_empty());
        assert!(data.types.is_empty());
    }

    #[test]
    fn drag_data_from_dioxus_drag_data_extracts_known_mime_types() {
        let native = TestNativeDataTransfer::with_data([
            ("text/plain", "hello"),
            ("text/html", "<p>hello</p>"),
            ("application/x-ars-custom", "custom"),
        ]);

        let event = DioxusDragData::new(TestDragData::new(native));

        let data = DragData::from_drag_data(&event);

        assert!(data.items.is_empty());
        assert_eq!(
            data.types,
            vec![String::from("text/plain"), String::from("text/html")]
        );
    }

    #[test]
    #[cfg(feature = "desktop")]
    fn drag_data_from_dioxus_drag_data_extracts_file_items_and_files_type() {
        let real_path = std::path::PathBuf::from("/tmp/report.JSON");

        let native = TestNativeDataTransfer::with_files([
            SerializedFileData {
                path: real_path.clone(),
                size: 42,
                last_modified: 100,
                content_type: Some(String::from("application/json")),
                contents: None,
            },
            SerializedFileData {
                path: std::path::PathBuf::from("untitled"),
                size: 7,
                last_modified: 101,
                content_type: None,
                contents: None,
            },
        ]);

        let event = DioxusDragData::new(TestDragData::new(native));

        let data = DragData::from_drag_data(&event);

        assert_eq!(data.types, vec![String::from("Files")]);
        assert_eq!(data.items.len(), 2);

        match &data.items[0] {
            DragItem::File {
                name,
                mime_type,
                size,
                handle,
            } => {
                assert_eq!(name, "report.JSON");
                assert_eq!(mime_type, "application/json");
                assert_eq!(*size, 42);
                assert_eq!(handle.as_path(), Some(real_path.as_path()));
            }

            other => panic!("unexpected first drag item: {other:?}"),
        }

        match &data.items[1] {
            DragItem::File {
                name,
                mime_type,
                size,
                handle,
            } => {
                assert_eq!(name, "untitled");
                assert_eq!(mime_type, "application/octet-stream");
                assert_eq!(*size, 7);
                assert_eq!(handle.as_path(), Some(std::path::Path::new("untitled")));
            }

            other => panic!("unexpected second drag item: {other:?}"),
        }
    }

    #[test]
    fn platform_drag_event_round_trips_dioxus_drag_data() {
        let event = DioxusDragData::new(TestDragData::new(TestNativeDataTransfer::default()));

        let wrapper = PlatformDragEvent::from_dioxus(&event);

        assert!(wrapper.as_dioxus().is_some());
        assert!(PlatformDragEvent::empty().as_dioxus().is_none());
    }

    #[cfg(not(any(feature = "web", feature = "desktop")))]
    #[test]
    fn default_dioxus_platform_falls_back_to_null_when_no_platform_feature() {
        let platform = default_dioxus_platform();

        // Only NullPlatform reports `Duration::ZERO`; the other impls
        // return a real clock value, so this is a precise probe.
        assert_eq!(platform.monotonic_now(), Duration::ZERO);
        assert!(platform.new_id().starts_with("null-id-"));
    }

    #[cfg(all(feature = "desktop", not(all(feature = "web", target_arch = "wasm32"))))]
    #[test]
    fn default_dioxus_platform_uses_desktop_when_web_is_unreachable() {
        // Fires whenever desktop is on AND we can't pick web (either web
        // is off, or we're not on wasm32). DesktopPlatform mints UUIDv4
        // strings (36 chars with dashes); the null fallback would return
        // a `null-id-N` instead.
        let platform = default_dioxus_platform();

        let id = platform.new_id();

        assert_eq!(id.len(), 36, "expected UUIDv4 length, got {id:?}");
        assert!(!id.starts_with("null-id-"));
    }

    #[cfg(all(feature = "web", not(feature = "desktop"), not(target_arch = "wasm32")))]
    #[test]
    fn default_dioxus_platform_falls_back_to_null_on_non_wasm_web_only_host() {
        // `WebPlatform` is wasm32-only, so `--features web` on a native
        // dev host (e.g., `cargo check --features web`) lands at the null
        // arm. This is by design — silent web-style behaviour on a native
        // process would mask misconfigured production builds.
        let platform = default_dioxus_platform();

        assert_eq!(platform.monotonic_now(), Duration::ZERO);
        assert!(platform.new_id().starts_with("null-id-"));
    }

    // -- DesktopPlatform method coverage --------------------------------
    //
    // These tests fire on any native build of `ars-dioxus` with the
    // `desktop` feature enabled (the workspace's `cargo xci` adapter
    // step runs them). They lift `DesktopPlatform` from ~12% method
    // coverage (only `new_id`, incidentally) to 100% method coverage,
    // minus `set_clipboard`'s arboard happy path which is not safe to
    // run in CI (it requires a display server and writes the user's
    // real clipboard).

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_file_picker_filter_extensions_parse_accept_tokens() {
        let extensions = file_picker_filter_extensions(&[
            String::from(".txt"),
            String::from("image/png,image/jpeg"),
            String::from("application/json"),
            String::from("image/*"),
        ]);

        assert_eq!(
            extensions,
            vec![
                String::from("txt"),
                String::from("png"),
                String::from("jpg"),
                String::from("jpeg"),
                String::from("json"),
                String::from("gif"),
                String::from("webp"),
                String::from("svg"),
                String::from("bmp"),
                String::from("ico"),
            ]
        );
        assert_eq!(
            file_picker_filter_name(&extensions[..3]),
            String::from("*.txt, *.png, *.jpg")
        );
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_file_picker_filter_extensions_cover_media_and_fallback_tokens() {
        let extensions = file_picker_filter_extensions(&[
            String::from("audio/*"),
            String::from("video/mp4,video/webm,video/quicktime"),
            String::from(".CUSTOM"),
            String::from("application/x-unknown"),
            String::from("*/*"),
        ]);

        assert_eq!(
            extensions,
            vec![
                String::from("mp3"),
                String::from("wav"),
                String::from("ogg"),
                String::from("flac"),
                String::from("aac"),
                String::from("mp4"),
                String::from("webm"),
                String::from("mov"),
                String::from("custom"),
            ]
        );
        assert_eq!(file_picker_filter_name(&[]), "All files");
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_mime_type_for_path_covers_known_and_fallback_extensions() {
        let cases = [
            ("note.TXT", "text/plain"),
            ("page.htm", "text/html"),
            ("style.css", "text/css"),
            ("table.csv", "text/csv"),
            ("doc.pdf", "application/pdf"),
            ("image.png", "image/png"),
            ("photo.jpeg", "image/jpeg"),
            ("anim.gif", "image/gif"),
            ("picture.webp", "image/webp"),
            ("icon.svg", "image/svg+xml"),
            ("bitmap.bmp", "image/bmp"),
            ("favicon.ico", "image/x-icon"),
            ("song.mp3", "audio/mpeg"),
            ("sound.wav", "audio/wav"),
            ("voice.ogg", "audio/ogg"),
            ("lossless.flac", "audio/flac"),
            ("clip.aac", "audio/aac"),
            ("movie.mp4", "video/mp4"),
            ("movie.webm", "video/webm"),
            ("movie.mov", "video/quicktime"),
            ("movie.avi", "video/x-msvideo"),
            ("movie.mkv", "video/x-matroska"),
            ("unknown.bin", "application/octet-stream"),
            ("no-extension", "application/octet-stream"),
        ];

        for (path, expected) in cases {
            assert_eq!(mime_type_for_path(std::path::Path::new(path)), expected);
        }
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_file_ref_from_path_reads_name_size_and_mime_type() {
        let path =
            std::env::temp_dir().join(format!("ars-dioxus-platform-{}.json", uuid::Uuid::new_v4()));

        std::fs::write(&path, br#"{"id":1}"#).expect("write temp file");

        let file = file_ref_from_path(&path);

        assert_eq!(
            file.name,
            path.file_name().unwrap().to_string_lossy().into_owned()
        );
        assert_eq!(file.size, 8);
        assert_eq!(file.mime_type, "application/json");

        std::fs::remove_file(path).expect("remove temp file");
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_platform_create_drag_data_returns_none_for_empty_event() {
        assert!(
            DesktopPlatform
                .create_drag_data(PlatformDragEvent::empty())
                .is_none()
        );
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_platform_create_drag_data_extracts_dioxus_payload() {
        let native = TestNativeDataTransfer::with_data([("application/json", "{\"id\":1}")]);

        let event = DioxusDragData::new(TestDragData::new(native));

        let data = DesktopPlatform
            .create_drag_data(PlatformDragEvent::from_dioxus(&event))
            .expect("desktop platform should extract Dioxus drag data");

        assert!(data.items.is_empty());
        assert_eq!(data.types, vec![String::from("application/json")]);
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_platform_monotonic_now_is_elapsed_from_process_start() {
        let now = DesktopPlatform.monotonic_now();

        let later = DesktopPlatform.monotonic_now();

        assert!(later >= now, "expected {later:?} >= {now:?}");
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn desktop_platform_new_id_yields_unique_uuid_v4_strings() {
        let first = DesktopPlatform.new_id();
        let second = DesktopPlatform.new_id();

        // Canonical `UUIDv4` representation is 36 characters with
        // dashes at fixed positions and version nibble '4' at index 14.
        for id in [&first, &second] {
            assert_eq!(id.len(), 36, "unexpected length: {id:?}");
            assert_eq!(id.as_bytes()[8], b'-');
            assert_eq!(id.as_bytes()[13], b'-');
            assert_eq!(id.as_bytes()[18], b'-');
            assert_eq!(id.as_bytes()[23], b'-');
            assert_eq!(id.as_bytes()[14], b'4');
        }

        // Two `Uuid::new_v4()` calls colliding has probability ~2^-122;
        // a failure here is a real signal, not a flake.
        assert_ne!(first, second);
    }

    /// Opt-in clipboard happy-path test.
    ///
    /// Default-skipped via `#[ignore]` because:
    ///
    /// 1. `arboard::Clipboard::new()` requires a display server
    ///    (X11 / Wayland / macOS `WindowServer` / Windows `USER32`)
    ///    and fails under headless CI runners.
    /// 2. Even when it succeeds, the test would clobber the
    ///    developer's real clipboard contents.
    ///
    /// Run with `cargo test -p ars-dioxus --features desktop -- --ignored`
    /// when validating arboard integration interactively.
    #[cfg(feature = "desktop")]
    #[test]
    #[ignore = "writes to the real OS clipboard; run manually with --ignored"]
    fn desktop_platform_set_clipboard_round_trips_via_arboard() {
        use arboard::Clipboard;

        let payload = "ars-ui desktop platform clipboard probe";

        let result = block_on_ready(DesktopPlatform.set_clipboard(payload));

        assert!(
            result.is_ok(),
            "set_clipboard returned an error on a host with a display server: {result:?}",
        );

        // Round-trip: read it back through a fresh arboard handle.
        let mut reader = Clipboard::new().expect("clipboard reader");

        let actual = reader.get_text().expect("clipboard read");

        assert_eq!(
            actual, payload,
            "clipboard round-trip lost or corrupted the payload"
        );
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn web_platform_monotonic_now_is_non_decreasing() {
        let platform = WebPlatform;

        let first = platform.monotonic_now();
        let second = platform.monotonic_now();

        assert!(second >= first, "expected {second:?} >= {first:?}");
    }

    #[wasm_bindgen_test]
    fn web_platform_new_id_yields_uuid_shape() {
        let id = WebPlatform.new_id();

        // crypto.randomUUID returns canonical UUIDv4: 36 chars, dashes at
        // positions 8/13/18/23, version nibble '4' at position 14.
        assert_eq!(id.len(), 36, "unexpected length: {id:?}");
        assert_eq!(id.as_bytes()[8], b'-');
        assert_eq!(id.as_bytes()[13], b'-');
        assert_eq!(id.as_bytes()[18], b'-');
        assert_eq!(id.as_bytes()[23], b'-');
        assert_eq!(id.as_bytes()[14], b'4');
    }

    #[wasm_bindgen_test]
    fn web_platform_create_drag_data_returns_none_for_empty_event() {
        // An empty wrapper carries no inner Dioxus drag event —
        // `create_drag_data` must early-return without touching the
        // event payload.
        assert!(
            WebPlatform
                .create_drag_data(PlatformDragEvent::empty())
                .is_none()
        );
    }

    #[wasm_bindgen_test]
    fn default_dioxus_platform_picks_web_on_wasm32() {
        // Under `--features web` on wasm32 the resolution ladder must
        // pick `WebPlatform`. Probe via `monotonic_now`: `WebPlatform`
        // returns a non-zero `performance.now()` reading (the page is
        // already loaded by the time the test runs); `NullPlatform`
        // would return `Duration::ZERO`.
        let platform = default_dioxus_platform();

        assert!(
            platform.monotonic_now() > Duration::ZERO,
            "expected a non-zero performance.now() reading on wasm32"
        );

        // UUIDv4 length confirms `crypto.randomUUID()` was used, not
        // the null-id counter.
        let id = platform.new_id();

        assert_eq!(id.len(), 36, "expected UUIDv4 length, got {id:?}");
    }

    #[wasm_bindgen_test]
    fn web_platform_open_file_picker_returns_empty() {
        use std::task::{Context, Poll, Waker};

        // The web impl deliberately defers to the FileUpload component's
        // hidden `<input>`; the trait method must resolve immediately
        // to an empty `Vec<FileRef>` regardless of options.
        let opts = FilePickerOptions {
            accept: vec![String::from("image/*")],
            multiple: true,
        };

        let pollable = WebPlatform.open_file_picker(opts);

        // We can't `block_on_ready` on wasm-test (no executor); use a
        // raw poll like the native helper does.
        let mut pinned = pollable;

        let waker = Waker::noop();

        let mut cx = Context::from_waker(waker);

        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(files) => assert!(files.is_empty()),
            Poll::Pending => panic!("open_file_picker future returned Pending on web"),
        }
    }

    #[wasm_bindgen_test]
    fn web_platform_new_id_is_unique_across_calls() {
        // crypto.randomUUID is RFC 4122 random; collision probability
        // ~2^-122 means a duplicate here is a real signal.
        let first = WebPlatform.new_id();
        let second = WebPlatform.new_id();
        let third = WebPlatform.new_id();

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(first, third);
    }
}
