//! DOM-backed coordinate-space helpers for floating positioning.
//!
//! These helpers bridge browser DOM measurements into the pure positioning
//! engine by detecting containing-block ancestors, resolving offset-parent
//! coordinate space, and converting client-space geometry into local-space
//! geometry.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsCast;

use super::Rect;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use super::types::Strategy;
#[cfg(any(test, feature = "web"))]
use super::types::{PositioningOptions, PositioningResult};
// `PositioningOptions` and `PositioningResult` appear only in the pipeline
// helper (`measure_and_compute_position`) and its pure worker, both of which
// require either the `web` feature or a test build. `compute_position`,
// `resolve_boundary_rect`, and `Boundary` are referenced even more narrowly
// (wasm32 + web, or tests) by the pure worker's boundary-normalization path.
// `Strategy` is only used by the wasm32 measurement glue. The cfg gates match
// each import's call sites so the `ssr`-only lib build does not warn about
// unused imports.
#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use super::{compute::compute_position, overflow::resolve_boundary_rect, types::Boundary};

/// Convert a client-space point into an axis-aligned local coordinate space.
///
/// This helper is correct when the local origin differs from client-space by
/// padding-box and scroll offsets only. Transformed containing blocks that
/// scale, rotate, skew, or otherwise remap axes require matrix-aware
/// conversion instead of simple origin subtraction.
#[must_use]
pub fn client_point_to_local_space(x: f64, y: f64, local_origin: &Rect) -> (f64, f64) {
    (x - local_origin.x, y - local_origin.y)
}

/// Convert a client-space rectangle into an axis-aligned local coordinate space.
#[must_use]
pub fn client_rect_to_local_space(rect: &Rect, local_origin: &Rect) -> Rect {
    let (x, y) = client_point_to_local_space(rect.x, rect.y, local_origin);

    Rect {
        x,
        y,
        width: rect.width,
        height: rect.height,
    }
}

/// Returns the padding-box rect of the first ancestor whose computed styles
/// create a containing block and whose local coordinate space can be
/// represented by axis-aligned origin subtraction.
///
/// If the containing block is established by a non-translation `transform`,
/// this returns `None` rather than exposing an incorrect local origin.
#[cfg(feature = "web")]
#[must_use]
pub fn find_containing_block_ancestor(element: &web_sys::Element) -> Option<Rect> {
    find_containing_block_ancestor_impl(element)
}

/// Returns the scroll-adjusted padding-box rect of the element's offset parent
/// when its local coordinate space remains axis-aligned.
///
/// If the offset parent has a non-translation `transform`, this returns
/// `None` rather than exposing an incorrect local origin.
#[cfg(feature = "web")]
#[must_use]
pub fn offset_parent_rect(element: &web_sys::Element) -> Option<Rect> {
    offset_parent_rect_impl(element)
}

/// Emits a debug warning when a portal target is nested under a containing block.
#[cfg(feature = "web")]
pub fn warn_if_portal_target_has_containing_block(target: &web_sys::Element) {
    warn_if_portal_target_has_containing_block_impl(target);
}

/// Emits a debug warning when a floating element is nested under containment
/// that may clip or trap overlays and should instead be portaled.
#[cfg(feature = "web")]
pub fn warn_if_floating_element_has_containment_issue(floating: &web_sys::Element) {
    warn_if_floating_element_has_containment_issue_impl(floating);
}

/// Measures `anchor` and `floating` in the DOM, resolves the local coordinate
/// space (the containing block for [`Strategy::Fixed`], the offset parent for
/// [`Strategy::Absolute`]), and computes a [`PositioningResult`] expressed in
/// that local space.
///
/// This is the bundled DOM measurement-to-position pipeline that overlay
/// adapters call to avoid duplicating browser measurement and coordinate
/// conversion. For [`Strategy::Fixed`] it uses the same containing-block walk
/// as [`find_containing_block_ancestor`] (bypassing that public helper so it
/// can distinguish "no ancestor" from "ancestor with non-translation
/// transform"); for [`Strategy::Absolute`] it uses [`offset_parent_rect`].
/// The resulting local origin is then applied via [`client_rect_to_local_space`]
/// before delegating to [`super::compute::compute_position`], matching
/// `spec/foundation/11-dom-utilities.md` §2.3.1 "Step 0: Detect Containing
/// Block" through §2.8.1 "CSS Transform Ancestor Detection".
///
/// Returns `None` when:
/// - The crate is built for a non-browser target (the `web` feature is off or
///   the build is not `wasm32`). In that case the helper is a no-op stub and
///   callers must fall back to their own SSR-safe path.
/// - `web_sys::window()` is unavailable at call time.
/// - [`Strategy::Absolute`] is requested but the floating element has no
///   valid offset parent (detached element, or the offset parent uses a
///   non-translation `transform`).
/// - [`Strategy::Fixed`] is requested and a containing-block ancestor exists
///   whose `transform` is not representable by axis-aligned origin
///   subtraction (scale, rotate, skew, perspective, non-zero `z` translate).
///   Callers should recover by portaling the overlay outside the transformed
///   ancestor.
///
/// When [`Strategy::Fixed`] is requested and no containing-block ancestor
/// exists, the pipeline operates in client space directly (the common case
/// for portaled overlays) and `Some(...)` is returned.
#[cfg(feature = "web")]
#[must_use]
pub fn measure_and_compute_position(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    options: &PositioningOptions,
) -> Option<PositioningResult> {
    measure_and_compute_position_impl(anchor, floating, options)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn measure_and_compute_position_impl(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    options: &PositioningOptions,
) -> Option<PositioningResult> {
    let window = web_sys::window()?;

    let anchor_dom = anchor.get_bounding_client_rect();

    let anchor_client = Rect {
        x: anchor_dom.x(),
        y: anchor_dom.y(),
        width: anchor_dom.width(),
        height: anchor_dom.height(),
    };

    // Only the floating element's dimensions are consumed by the positioning
    // engine; its x/y are an output of the computation, so zero them here.
    let floating_dom = floating.get_bounding_client_rect();

    let floating_dims = Rect {
        x: 0.0,
        y: 0.0,
        width: floating_dom.width(),
        height: floating_dom.height(),
    };

    let viewport_client = super::viewport::viewport_rect(&window);

    let local_origin = match options.strategy {
        Strategy::Fixed => match walk_containing_block_ancestors(floating) {
            None => None,
            Some(containing_block) if supports_axis_aligned_local_space(&containing_block) => {
                Some(padding_box_rect(&containing_block.element))
            }
            Some(_) => return None,
        },

        Strategy::Absolute => Some(offset_parent_rect_impl(floating)?),
    };

    Some(compute_in_local_space(
        &anchor_client,
        &floating_dims,
        &viewport_client,
        local_origin.as_ref(),
        options,
    ))
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn measure_and_compute_position_impl(
    _anchor: &web_sys::Element,
    _floating: &web_sys::Element,
    _options: &PositioningOptions,
) -> Option<PositioningResult> {
    None
}

/// Pure coordinate-space adjustment around [`compute_position`].
///
/// Resolves the caller's [`Boundary`] choice in client space first (picking up
/// the `getBoundingClientRect()` reading for [`Boundary::Element`] on wasm, or
/// the viewport fallback on host), then converts the anchor rect and that
/// boundary rect into the `local_origin`'s local space via axis-aligned origin
/// subtraction. When `local_origin` is `None`, both rects are forwarded
/// unchanged, which matches the client-space behaviour of [`Strategy::Fixed`]
/// with no containing-block ancestor.
///
/// The boundary is pre-resolved here — not left for [`compute_position`] to
/// resolve — because `compute_position` calls `getBoundingClientRect()` itself,
/// which always reports client-space coordinates. Leaving the boundary
/// unconverted while anchor/viewport moved into local space would make
/// flip/shift/max-size compare anchor-in-local-space against
/// boundary-in-client-space, causing placement errors whenever `local_origin`
/// is non-zero.
///
/// Because [`compute_position`]'s `viewport` argument is read only to resolve
/// [`Boundary::Viewport`], this helper forwards the pre-resolved boundary as
/// that argument and swaps `options.boundary` to [`Boundary::Viewport`] in a
/// cloned copy so the engine's own resolution path is a no-op pass-through.
///
/// Only compiled for the wasm32 web impl and the host tests; the non-wasm32
/// lib build relies on the [`measure_and_compute_position`] stub returning
/// `None` directly, so this helper would otherwise be dead code there.
#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn compute_in_local_space(
    anchor_client: &Rect,
    floating_dims: &Rect,
    viewport_client: &Rect,
    local_origin: Option<&Rect>,
    options: &PositioningOptions,
) -> PositioningResult {
    let boundary_client = resolve_boundary_rect(&options.boundary, viewport_client);

    let (anchor, boundary) = if let Some(origin) = local_origin {
        (
            client_rect_to_local_space(anchor_client, origin),
            client_rect_to_local_space(&boundary_client, origin),
        )
    } else {
        (*anchor_client, boundary_client)
    };

    let mut normalized_options = options.clone();
    normalized_options.boundary = Boundary::Viewport;

    compute_position(&anchor, floating_dims, &boundary, &normalized_options)
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainingBlockReason {
    Transform,
    Perspective,
    Filter,
    BackdropFilter,
    Contain,
    ContentVisibility,
    ContainerType,
    WillChangeTransform,
    WillChangePerspective,
    WillChangeFilter,
    WillChangeBackdropFilter,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
impl ContainingBlockReason {
    fn property_name(self) -> &'static str {
        match self {
            Self::Transform => "transform",
            Self::Perspective => "perspective",
            Self::Filter => "filter",
            Self::BackdropFilter => "backdrop-filter",
            Self::Contain => "contain",
            Self::ContentVisibility => "content-visibility",
            Self::ContainerType => "container-type",
            Self::WillChangeTransform => "will-change: transform",
            Self::WillChangePerspective => "will-change: perspective",
            Self::WillChangeFilter => "will-change: filter",
            Self::WillChangeBackdropFilter => "will-change: backdrop-filter",
        }
    }
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Debug, Default)]
struct StyleSnapshot {
    transform: String,
    perspective: String,
    filter: String,
    backdrop_filter: String,
    contain: String,
    container_type: String,
    will_change: String,
    content_visibility: String,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn containing_block_reason(style: &StyleSnapshot) -> Option<ContainingBlockReason> {
    if is_active_css_value(&style.transform, "none") {
        return Some(ContainingBlockReason::Transform);
    }

    if is_active_css_value(&style.perspective, "none") {
        return Some(ContainingBlockReason::Perspective);
    }

    if is_active_css_value(&style.filter, "none") {
        return Some(ContainingBlockReason::Filter);
    }

    if is_active_css_value(&style.backdrop_filter, "none") {
        return Some(ContainingBlockReason::BackdropFilter);
    }

    if contains_whitespace_token(&style.contain, "layout")
        || contains_whitespace_token(&style.contain, "paint")
        || contains_whitespace_token(&style.contain, "strict")
        || contains_whitespace_token(&style.contain, "content")
    {
        return Some(ContainingBlockReason::Contain);
    }

    if normalized_css_value(&style.content_visibility) == "auto" {
        return Some(ContainingBlockReason::ContentVisibility);
    }

    let container_type = normalized_css_value(&style.container_type);
    if !container_type.is_empty() && container_type != "normal" {
        return Some(ContainingBlockReason::ContainerType);
    }

    if contains_comma_separated_token(&style.will_change, "transform") {
        return Some(ContainingBlockReason::WillChangeTransform);
    }

    if contains_comma_separated_token(&style.will_change, "perspective") {
        return Some(ContainingBlockReason::WillChangePerspective);
    }

    if contains_comma_separated_token(&style.will_change, "filter") {
        return Some(ContainingBlockReason::WillChangeFilter);
    }

    if contains_comma_separated_token(&style.will_change, "backdrop-filter") {
        return Some(ContainingBlockReason::WillChangeBackdropFilter);
    }

    None
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn is_active_css_value(value: &str, default: &str) -> bool {
    let normalized = normalized_css_value(value);
    !normalized.is_empty() && normalized != default
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn normalized_css_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn contains_comma_separated_token(value: &str, expected: &str) -> bool {
    normalized_css_value(value)
        .split(',')
        .map(str::trim)
        .any(|token| token == expected)
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn contains_whitespace_token(value: &str, expected: &str) -> bool {
    normalized_css_value(value)
        .split_whitespace()
        .any(|token| token == expected)
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn overlay_containment_warning(style: &StyleSnapshot) -> Option<&'static str> {
    if contains_whitespace_token(&style.contain, "layout") {
        return Some("contain: layout");
    }

    if contains_whitespace_token(&style.contain, "paint") {
        return Some("contain: paint");
    }

    if contains_whitespace_token(&style.contain, "strict") {
        return Some("contain: strict");
    }

    if contains_whitespace_token(&style.contain, "content") {
        return Some("contain: content");
    }

    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn find_containing_block_ancestor_impl(element: &web_sys::Element) -> Option<Rect> {
    let containing_block = walk_containing_block_ancestors(element)?;

    containing_block_rect(&containing_block)
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn find_containing_block_ancestor_impl(_element: &web_sys::Element) -> Option<Rect> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn offset_parent_rect_impl(element: &web_sys::Element) -> Option<Rect> {
    let window = web_sys::window()?;
    let element = element.dyn_ref::<web_sys::HtmlElement>()?;
    let offset_parent = element.offset_parent()?;
    let style = window.get_computed_style(&offset_parent).ok().flatten()?;
    let snapshot = style_snapshot(&style);

    if !transform_is_translation_only(&snapshot.transform) {
        return None;
    }

    let mut rect = padding_box_rect(&offset_parent);
    rect.x -= f64::from(offset_parent.scroll_left());
    rect.y -= f64::from(offset_parent.scroll_top());

    Some(rect)
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn offset_parent_rect_impl(_element: &web_sys::Element) -> Option<Rect> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn warn_if_portal_target_has_containing_block_impl(target: &web_sys::Element) {
    if let Some(containing_block) = walk_containing_block_ancestors(target) {
        crate::debug::warn_message(format_args!(
            "Portal target has ancestor with {} which breaks position:fixed positioning. Overlays may be mispositioned. Move the portal target outside this ancestor.",
            containing_block.reason.property_name()
        ));
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn warn_if_portal_target_has_containing_block_impl(_target: &web_sys::Element) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn warn_if_floating_element_has_containment_issue_impl(floating: &web_sys::Element) {
    if let Some(reason) = walk_overlay_containment_ancestors(floating) {
        crate::debug::warn_message(format_args!(
            "Floating element has ancestor with {reason} which may clip or trap overlays. Consider portaling the overlay content outside this ancestor."
        ));
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn warn_if_floating_element_has_containment_issue_impl(_floating: &web_sys::Element) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct ContainingBlockMatch {
    element: web_sys::Element,
    reason: ContainingBlockReason,
    style: StyleSnapshot,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn walk_containing_block_ancestors(element: &web_sys::Element) -> Option<ContainingBlockMatch> {
    let window = web_sys::window()?;

    let mut current = next_composed_ancestor_element(element.unchecked_ref());

    while let Some(ancestor) = current {
        let Some(style) = window.get_computed_style(&ancestor).ok().flatten() else {
            current = next_composed_ancestor_element(ancestor.unchecked_ref());

            continue;
        };

        let snapshot = style_snapshot(&style);

        if let Some(reason) = containing_block_reason(&snapshot) {
            return Some(ContainingBlockMatch {
                element: ancestor,
                reason,
                style: snapshot,
            });
        }

        current = next_composed_ancestor_element(ancestor.unchecked_ref());
    }

    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn walk_overlay_containment_ancestors(element: &web_sys::Element) -> Option<&'static str> {
    let window = web_sys::window()?;

    let mut current = next_composed_ancestor_element(element.unchecked_ref());

    while let Some(ancestor) = current {
        let Some(style) = window.get_computed_style(&ancestor).ok().flatten() else {
            current = next_composed_ancestor_element(ancestor.unchecked_ref());

            continue;
        };

        let snapshot = style_snapshot(&style);

        if let Some(reason) = overlay_containment_warning(&snapshot) {
            return Some(reason);
        }

        current = next_composed_ancestor_element(ancestor.unchecked_ref());
    }

    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn containing_block_rect(containing_block: &ContainingBlockMatch) -> Option<Rect> {
    if !supports_axis_aligned_local_space(containing_block) {
        return None;
    }

    Some(padding_box_rect(&containing_block.element))
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn supports_axis_aligned_local_space(containing_block: &ContainingBlockMatch) -> bool {
    match containing_block.reason {
        ContainingBlockReason::Transform => {
            transform_is_translation_only(&containing_block.style.transform)
        }
        _ => true,
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn transform_is_translation_only(transform: &str) -> bool {
    let normalized = normalized_css_value(transform);

    if normalized.is_empty() || normalized == "none" {
        return true;
    }

    let Ok(matrix) = web_sys::DomMatrixReadOnly::new_with_str(transform) else {
        return false;
    };

    is_identity_or_translation_component(matrix.m11(), 1.0)
        && is_identity_or_translation_component(matrix.m12(), 0.0)
        && is_identity_or_translation_component(matrix.m13(), 0.0)
        && is_identity_or_translation_component(matrix.m14(), 0.0)
        && is_identity_or_translation_component(matrix.m21(), 0.0)
        && is_identity_or_translation_component(matrix.m22(), 1.0)
        && is_identity_or_translation_component(matrix.m23(), 0.0)
        && is_identity_or_translation_component(matrix.m24(), 0.0)
        && is_identity_or_translation_component(matrix.m31(), 0.0)
        && is_identity_or_translation_component(matrix.m32(), 0.0)
        && is_identity_or_translation_component(matrix.m33(), 1.0)
        && is_identity_or_translation_component(matrix.m34(), 0.0)
        && is_identity_or_translation_component(matrix.m43(), 0.0)
        && is_identity_or_translation_component(matrix.m44(), 1.0)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_identity_or_translation_component(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn next_composed_ancestor_element(node: &web_sys::Node) -> Option<web_sys::Element> {
    if let Some(element) = node.dyn_ref::<web_sys::Element>()
        && let Some(slot) = element.assigned_slot()
    {
        return Some(slot.unchecked_into());
    }

    let parent = node.parent_node()?;

    if let Some(element) = parent.dyn_ref::<web_sys::Element>() {
        return Some(element.clone());
    }

    parent
        .dyn_ref::<web_sys::ShadowRoot>()
        .map(web_sys::ShadowRoot::host)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn style_snapshot(style: &web_sys::CssStyleDeclaration) -> StyleSnapshot {
    StyleSnapshot {
        transform: style.get_property_value("transform").unwrap_or_default(),
        perspective: style.get_property_value("perspective").unwrap_or_default(),
        filter: style.get_property_value("filter").unwrap_or_default(),
        backdrop_filter: style
            .get_property_value("backdrop-filter")
            .unwrap_or_default(),
        contain: style.get_property_value("contain").unwrap_or_default(),
        container_type: style
            .get_property_value("container-type")
            .unwrap_or_default(),
        will_change: style.get_property_value("will-change").unwrap_or_default(),
        content_visibility: style
            .get_property_value("content-visibility")
            .unwrap_or_default(),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn padding_box_rect(element: &web_sys::Element) -> Rect {
    let rect = element.get_bounding_client_rect();

    Rect {
        x: rect.x() + f64::from(element.client_left()),
        y: rect.y() + f64::from(element.client_top()),
        width: f64::from(element.client_width()),
        height: f64::from(element.client_height()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style_with(configure: impl FnOnce(&mut StyleSnapshot)) -> StyleSnapshot {
        let mut style = StyleSnapshot::default();

        configure(&mut style);

        style
    }

    #[test]
    fn transform_creates_containing_block() {
        let style = style_with(|style| style.transform = "translateX(12px)".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Transform)
        );
    }

    #[test]
    fn perspective_creates_containing_block() {
        let style = style_with(|style| style.perspective = "500px".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Perspective)
        );
    }

    #[test]
    fn filter_creates_containing_block() {
        let style = style_with(|style| style.filter = "blur(2px)".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Filter)
        );
    }

    #[test]
    fn backdrop_filter_creates_containing_block() {
        let style = style_with(|style| style.backdrop_filter = "blur(4px)".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::BackdropFilter)
        );
    }

    #[test]
    fn contain_layout_creates_containing_block() {
        let style = style_with(|style| style.contain = "layout".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Contain)
        );
    }

    #[test]
    fn contain_paint_creates_containing_block() {
        let style = style_with(|style| style.contain = "paint".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Contain)
        );
    }

    #[test]
    fn contain_strict_creates_containing_block() {
        let style = style_with(|style| style.contain = "strict".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Contain)
        );
    }

    #[test]
    fn contain_content_creates_containing_block() {
        let style = style_with(|style| style.contain = "content".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::Contain)
        );
    }

    #[test]
    fn content_visibility_auto_creates_containing_block() {
        let style = style_with(|style| style.content_visibility = "auto".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::ContentVisibility)
        );
    }

    #[test]
    fn non_default_container_type_creates_containing_block() {
        let style = style_with(|style| style.container_type = "inline-size".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::ContainerType)
        );
    }

    #[test]
    fn will_change_transform_creates_containing_block() {
        let style = style_with(|style| style.will_change = "transform".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::WillChangeTransform)
        );
    }

    #[test]
    fn will_change_perspective_creates_containing_block() {
        let style = style_with(|style| style.will_change = "perspective".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::WillChangePerspective)
        );
    }

    #[test]
    fn will_change_filter_creates_containing_block() {
        let style = style_with(|style| style.will_change = "filter".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::WillChangeFilter)
        );
    }

    #[test]
    fn will_change_backdrop_filter_creates_containing_block() {
        let style = style_with(|style| style.will_change = "backdrop-filter".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::WillChangeBackdropFilter)
        );
    }

    #[test]
    fn contain_style_is_ignored() {
        let style = style_with(|style| style.contain = "style".into());

        assert_eq!(containing_block_reason(&style), None);
    }

    #[test]
    fn contain_layout_warns_about_overlay_containment() {
        let style = style_with(|style| style.contain = "layout".into());

        assert_eq!(overlay_containment_warning(&style), Some("contain: layout"));
    }

    #[test]
    fn contain_paint_warns_about_overlay_containment() {
        let style = style_with(|style| style.contain = "paint".into());

        assert_eq!(overlay_containment_warning(&style), Some("contain: paint"));
    }

    #[test]
    fn contain_content_warns_about_overlay_containment() {
        let style = style_with(|style| style.contain = "content".into());

        assert_eq!(
            overlay_containment_warning(&style),
            Some("contain: content")
        );
    }

    #[test]
    fn contain_strict_warns_about_overlay_containment() {
        let style = style_with(|style| style.contain = "strict".into());

        assert_eq!(overlay_containment_warning(&style), Some("contain: strict"));
    }

    #[test]
    fn contain_style_does_not_warn_about_overlay_containment() {
        let style = style_with(|style| style.contain = "style".into());

        assert_eq!(overlay_containment_warning(&style), None);
    }

    #[test]
    fn unrelated_will_change_is_ignored() {
        let style = style_with(|style| style.will_change = "opacity, left".into());

        assert_eq!(containing_block_reason(&style), None);
    }

    #[test]
    fn comma_separated_will_change_detects_relevant_value() {
        let style = style_with(|style| style.will_change = "opacity, filter".into());

        assert_eq!(
            containing_block_reason(&style),
            Some(ContainingBlockReason::WillChangeFilter)
        );
    }

    #[test]
    fn default_values_do_not_create_containing_block() {
        let style = style_with(|style| {
            style.transform = "none".into();
            style.perspective = "none".into();
            style.filter = "none".into();
            style.backdrop_filter = "none".into();
            style.contain = "none".into();
            style.container_type = "normal".into();
            style.will_change = "auto".into();
            style.content_visibility = "visible".into();
        });

        assert_eq!(containing_block_reason(&style), None);
    }

    #[test]
    fn containing_block_reason_property_names_match_expected_css_properties() {
        assert_eq!(
            ContainingBlockReason::Transform.property_name(),
            "transform"
        );
        assert_eq!(
            ContainingBlockReason::Perspective.property_name(),
            "perspective"
        );
        assert_eq!(ContainingBlockReason::Filter.property_name(), "filter");
        assert_eq!(
            ContainingBlockReason::BackdropFilter.property_name(),
            "backdrop-filter"
        );
        assert_eq!(ContainingBlockReason::Contain.property_name(), "contain");
        assert_eq!(
            ContainingBlockReason::ContentVisibility.property_name(),
            "content-visibility"
        );
        assert_eq!(
            ContainingBlockReason::ContainerType.property_name(),
            "container-type"
        );
        assert_eq!(
            ContainingBlockReason::WillChangeTransform.property_name(),
            "will-change: transform"
        );
        assert_eq!(
            ContainingBlockReason::WillChangePerspective.property_name(),
            "will-change: perspective"
        );
        assert_eq!(
            ContainingBlockReason::WillChangeFilter.property_name(),
            "will-change: filter"
        );
        assert_eq!(
            ContainingBlockReason::WillChangeBackdropFilter.property_name(),
            "will-change: backdrop-filter"
        );
    }

    #[test]
    fn point_conversion_subtracts_local_origin() {
        let local_origin = Rect {
            x: 12.5,
            y: 20.25,
            width: 80.0,
            height: 40.0,
        };

        assert_eq!(
            client_point_to_local_space(33.75, 48.5, &local_origin),
            (21.25, 28.25)
        );
    }

    #[test]
    fn rect_conversion_preserves_dimensions_and_supports_fractional_values() {
        let rect = Rect {
            x: 100.5,
            y: 250.25,
            width: 75.75,
            height: 40.5,
        };

        let local_origin = Rect {
            x: 90.25,
            y: 200.0,
            width: 120.0,
            height: 90.0,
        };

        assert_eq!(
            client_rect_to_local_space(&rect, &local_origin),
            Rect {
                x: 10.25,
                y: 50.25,
                width: 75.75,
                height: 40.5,
            }
        );
    }

    #[test]
    fn absolute_strategy_offset_parent_adjustment_matches_spec() {
        let offset_parent = Rect {
            x: 40.0,
            y: 32.0,
            width: 240.0,
            height: 200.0,
        };

        let client_rect = Rect {
            x: 118.5,
            y: 96.25,
            width: 32.0,
            height: 18.0,
        };

        assert_eq!(
            client_rect_to_local_space(&client_rect, &offset_parent),
            Rect {
                x: 78.5,
                y: 64.25,
                width: 32.0,
                height: 18.0,
            }
        );
        assert_eq!(
            client_point_to_local_space(client_rect.x, client_rect.y, &offset_parent),
            (78.5, 64.25)
        );
    }

    #[test]
    fn absolute_strategy_offset_parent_adjustment_uses_padding_box_origin() {
        let offset_parent_padding_box = Rect {
            x: 44.0,
            y: 38.0,
            width: 240.0,
            height: 200.0,
        };

        let client_rect = Rect {
            x: 118.5,
            y: 96.25,
            width: 32.0,
            height: 18.0,
        };

        assert_eq!(
            client_rect_to_local_space(&client_rect, &offset_parent_padding_box),
            Rect {
                x: 74.5,
                y: 58.25,
                width: 32.0,
                height: 18.0,
            }
        );
        assert_eq!(
            client_point_to_local_space(client_rect.x, client_rect.y, &offset_parent_padding_box),
            (74.5, 58.25)
        );
    }

    #[test]
    fn absolute_strategy_offset_parent_adjustment_accounts_for_scroll() {
        let offset_parent_local_origin = Rect {
            x: 32.0,
            y: 26.0,
            width: 240.0,
            height: 200.0,
        };

        let client_rect = Rect {
            x: 118.5,
            y: 96.25,
            width: 32.0,
            height: 18.0,
        };

        assert_eq!(
            client_rect_to_local_space(&client_rect, &offset_parent_local_origin),
            Rect {
                x: 86.5,
                y: 70.25,
                width: 32.0,
                height: 18.0,
            }
        );
        assert_eq!(
            client_point_to_local_space(client_rect.x, client_rect.y, &offset_parent_local_origin),
            (86.5, 70.25)
        );
    }

    // -----------------------------------------------------------------
    // Pure-math tests for the measurement-to-position pipeline helper
    // (`compute_in_local_space`). These exercise the coordinate
    // adjustment that `measure_and_compute_position` applies around the
    // pure `compute_position()` engine, without touching the DOM.
    // -----------------------------------------------------------------

    use crate::positioning::{
        compute::compute_position,
        types::{Boundary, Placement, PositioningOptions},
    };

    fn pipeline_options(placement: Placement) -> PositioningOptions {
        PositioningOptions {
            placement,
            flip: false,
            shift: false,
            ..PositioningOptions::default()
        }
    }

    #[test]
    fn pipeline_without_local_origin_matches_compute_position_in_client_space() {
        // No containing block and no offset parent: the pipeline must leave
        // anchor and viewport in client space and delegate to
        // `compute_position()` unchanged.
        let anchor = Rect {
            x: 250.0,
            y: 200.0,
            width: 100.0,
            height: 50.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 40.0,
        };

        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
        };

        let options = pipeline_options(Placement::Bottom);

        let pipeline = compute_in_local_space(&anchor, &floating, &viewport, None, &options);

        let baseline = compute_position(&anchor, &floating, &viewport, &options);

        assert_eq!(pipeline, baseline);
    }

    #[test]
    fn pipeline_with_local_origin_subtracts_origin_from_anchor_and_viewport() {
        // Containing-block (or offset-parent) local-origin case: the pipeline
        // must subtract the origin from BOTH anchor and viewport rects before
        // calling `compute_position()`. The result MUST match calling
        // `compute_position()` directly with the already-adjusted rects.
        let origin = Rect {
            x: 100.0,
            y: 50.0,
            width: 400.0,
            height: 300.0,
        };

        let anchor_client = Rect {
            x: 250.0,
            y: 200.0,
            width: 100.0,
            height: 50.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 40.0,
        };

        let viewport_client = Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
        };

        let options = pipeline_options(Placement::Bottom);

        let pipeline = compute_in_local_space(
            &anchor_client,
            &floating,
            &viewport_client,
            Some(&origin),
            &options,
        );

        let anchor_local = client_rect_to_local_space(&anchor_client, &origin);

        let viewport_local = client_rect_to_local_space(&viewport_client, &origin);

        let expected = compute_position(&anchor_local, &floating, &viewport_local, &options);

        assert_eq!(pipeline, expected);
    }

    #[test]
    fn compute_in_local_space_matches_offset_parent_formula_from_spec() {
        // Cross-check that the pipeline's local-origin subtraction is
        // equivalent to the manual `Strategy::Absolute` adjustment documented
        // in spec/foundation/11-dom-utilities.md §2.8. The offset-parent rect
        // we pass already includes `client_left`/`client_top` and
        // `-scroll_left`/`-scroll_top` per `offset_parent_rect_impl`; the
        // strategy branching itself lives in `measure_and_compute_position_impl`
        // and is exercised by the wasm tests below.
        let offset_parent_local_origin = Rect {
            x: 32.0,
            y: 26.0,
            width: 240.0,
            height: 200.0,
        };

        let anchor_client = Rect {
            x: 118.5,
            y: 96.25,
            width: 32.0,
            height: 18.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 12.0,
        };

        let viewport_client = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };

        let options = pipeline_options(Placement::BottomStart);

        let pipeline = compute_in_local_space(
            &anchor_client,
            &floating,
            &viewport_client,
            Some(&offset_parent_local_origin),
            &options,
        );

        // Manually apply the spec formula: subtract padding-box local origin
        // from every client-space input, then call compute_position.
        let anchor_local = Rect {
            x: anchor_client.x - offset_parent_local_origin.x,
            y: anchor_client.y - offset_parent_local_origin.y,
            width: anchor_client.width,
            height: anchor_client.height,
        };

        let viewport_local = Rect {
            x: viewport_client.x - offset_parent_local_origin.x,
            y: viewport_client.y - offset_parent_local_origin.y,
            width: viewport_client.width,
            height: viewport_client.height,
        };

        let expected = compute_position(&anchor_local, &floating, &viewport_local, &options);

        assert_eq!(pipeline, expected);

        // Sanity-check anchor_local matches §2.8 example arithmetic.
        assert_eq!(anchor_local.x, 86.5);
        assert_eq!(anchor_local.y, 70.25);
    }

    // -----------------------------------------------------------------
    // Regression coverage for the Boundary coordinate-space fix.
    //
    // `compute_position` resolves `Boundary::Element` via
    // `getBoundingClientRect()` on wasm, which reports client-space
    // coordinates. When `compute_in_local_space` has already moved anchor
    // and viewport into the containing-block's local space, the engine
    // would otherwise compare local-space positions against a client-space
    // boundary — mispredicting flip/shift/max-size whenever `local_origin`
    // is non-zero. The fix pre-resolves the boundary in client space, runs
    // the same origin subtraction on it, and passes `Boundary::Viewport`
    // to the engine so the already-resolved rect is used as-is.
    // -----------------------------------------------------------------

    #[test]
    fn pipeline_boundary_element_fallback_is_converted_to_local_space_when_origin_is_non_zero() {
        // `Boundary::Element` cannot actually resolve a DOM element on host
        // targets, so `resolve_boundary_rect` falls back to the passed
        // viewport. The fix must still run that fallback rect through the
        // same origin subtraction as the anchor — otherwise flip would
        // compare the local-space anchor against the client-space viewport.
        //
        // Setup: an anchor whose right edge, when measured in local space,
        // overflows the (converted) viewport by enough to trigger flip.
        // The exact deltas here are chosen so the two code paths would
        // disagree if the fix regressed — a large local origin makes the
        // client-vs-local mismatch unmistakable.
        let origin = Rect {
            x: 400.0,
            y: 300.0,
            width: 0.0,
            height: 0.0,
        };

        let anchor_client = Rect {
            x: 950.0,
            y: 360.0,
            width: 40.0,
            height: 20.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 20.0,
        };

        let viewport_client = Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
        };

        #[cfg(not(target_arch = "wasm32"))]
        let element_ref: std::sync::Arc<dyn std::any::Any + Send + Sync> = std::sync::Arc::new(());
        #[cfg(target_arch = "wasm32")]
        let element_ref: std::rc::Rc<dyn std::any::Any> = std::rc::Rc::new(());

        let options = PositioningOptions {
            placement: Placement::Right,
            boundary: Boundary::Element(element_ref),
            flip: true,
            ..PositioningOptions::default()
        };

        let pipeline = compute_in_local_space(
            &anchor_client,
            &floating,
            &viewport_client,
            Some(&origin),
            &options,
        );

        // Expected: boundary fallback is `viewport_client` (since the
        // element cannot resolve on host), converted to local space, and
        // passed in via `Boundary::Viewport`.
        let anchor_local = client_rect_to_local_space(&anchor_client, &origin);
        let boundary_local = client_rect_to_local_space(&viewport_client, &origin);

        let mut expected_options = options.clone();
        expected_options.boundary = Boundary::Viewport;

        let expected =
            compute_position(&anchor_local, &floating, &boundary_local, &expected_options);

        assert_eq!(pipeline, expected);
    }

    #[test]
    fn pipeline_boundary_viewport_with_local_origin_still_matches_hand_computed_local_space() {
        // Regression guard for the common path. Even after the fix rewrote
        // the internals to pre-resolve the boundary and swap to
        // `Boundary::Viewport`, `Boundary::Viewport` + `local_origin =
        // Some(..)` must still produce the same result as hand-applying
        // origin subtraction to anchor and viewport and calling
        // `compute_position` directly. This covers the overwhelmingly
        // common overlay case (no explicit boundary element).
        let origin = Rect {
            x: 150.0,
            y: 90.0,
            width: 600.0,
            height: 500.0,
        };

        let anchor_client = Rect {
            x: 300.0,
            y: 210.0,
            width: 100.0,
            height: 40.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 24.0,
        };

        let viewport_client = Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 800.0,
        };

        let options = PositioningOptions {
            placement: Placement::Bottom,
            flip: true,
            shift: true,
            ..PositioningOptions::default()
        };

        let pipeline = compute_in_local_space(
            &anchor_client,
            &floating,
            &viewport_client,
            Some(&origin),
            &options,
        );

        // Pre-fix semantics: manual origin subtraction, then
        // `Boundary::Viewport` resolves to that converted viewport.
        let anchor_local = client_rect_to_local_space(&anchor_client, &origin);
        let viewport_local = client_rect_to_local_space(&viewport_client, &origin);
        let expected = compute_position(&anchor_local, &floating, &viewport_local, &options);

        assert_eq!(pipeline, expected);
    }

    #[test]
    fn pipeline_without_local_origin_passes_boundary_element_through_as_resolved_rect() {
        // When `local_origin` is `None`, the fix must not quietly alter
        // output either: `Boundary::Element` still falls back to viewport
        // on host, so the pipeline's result must match calling
        // `compute_position` directly with client-space anchor/viewport
        // and `Boundary::Viewport`.
        let anchor = Rect {
            x: 200.0,
            y: 160.0,
            width: 120.0,
            height: 32.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 24.0,
        };

        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
        };

        #[cfg(not(target_arch = "wasm32"))]
        let element_ref: std::sync::Arc<dyn std::any::Any + Send + Sync> = std::sync::Arc::new(());
        #[cfg(target_arch = "wasm32")]
        let element_ref: std::rc::Rc<dyn std::any::Any> = std::rc::Rc::new(());

        let options = PositioningOptions {
            placement: Placement::Bottom,
            boundary: Boundary::Element(element_ref),
            flip: true,
            ..PositioningOptions::default()
        };

        let pipeline = compute_in_local_space(&anchor, &floating, &viewport, None, &options);

        let mut expected_options = options.clone();
        expected_options.boundary = Boundary::Viewport;

        let expected = compute_position(&anchor, &floating, &viewport, &expected_options);

        assert_eq!(pipeline, expected);
    }
}

#[cfg(all(test, feature = "web", not(target_arch = "wasm32")))]
mod host_web_tests {
    use wasm_bindgen::{JsCast, JsValue};

    use super::*;

    #[test]
    fn containing_block_lookup_returns_none_without_browser_dom() {
        let value = JsValue::NULL;

        let element = value.unchecked_ref::<web_sys::Element>();

        assert_eq!(find_containing_block_ancestor(element), None);
    }

    #[test]
    fn offset_parent_rect_returns_none_without_browser_dom() {
        let value = JsValue::NULL;

        let element = value.unchecked_ref::<web_sys::Element>();

        assert_eq!(offset_parent_rect(element), None);
    }

    #[test]
    fn portal_target_warning_is_noop_without_browser_dom() {
        let value = JsValue::NULL;

        let element = value.unchecked_ref::<web_sys::Element>();

        warn_if_portal_target_has_containing_block(element);
    }

    #[test]
    fn floating_containment_warning_is_noop_without_browser_dom() {
        let value = JsValue::NULL;

        let element = value.unchecked_ref::<web_sys::Element>();

        warn_if_floating_element_has_containment_issue(element);
    }

    #[test]
    fn measure_and_compute_position_returns_none_without_browser_dom() {
        // On host builds the pipeline helper has no window or DOM to read —
        // it MUST report "unavailable" via `None` so adapters can fall back
        // to a safe no-op instead of panicking during SSR.
        let value = JsValue::NULL;

        let anchor = value.unchecked_ref::<web_sys::Element>();

        let floating = value.unchecked_ref::<web_sys::Element>();

        let options = PositioningOptions::default();

        assert_eq!(
            measure_and_compute_position(anchor, floating, &options),
            None
        );
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    struct DomFixture {
        root: web_sys::Element,
    }

    impl DomFixture {
        fn new() -> Self {
            let window = web_sys::window().expect("window should exist in browser tests");

            let document = window
                .document()
                .expect("document should exist in browser tests");

            let root = document
                .create_element("div")
                .expect("test root should be created");

            root.set_attribute(
                "style",
                "position: absolute; left: 24px; top: 18px; width: 300px; height: 300px;",
            )
            .expect("test root style should be applied");

            document
                .body()
                .expect("document body should exist in browser tests")
                .append_child(&root)
                .expect("test root should be appended to body");

            Self { root }
        }

        fn append_child(parent: &web_sys::Element, style: &str) -> web_sys::Element {
            Self::append_child_to_node(parent.unchecked_ref(), style)
        }

        fn append_tag_child_to_node(
            parent: &web_sys::Node,
            tag: &str,
            style: &str,
        ) -> web_sys::Element {
            let document = parent
                .owner_document()
                .expect("fixture parent should have a document");

            let element = document
                .create_element(tag)
                .expect("fixture child should be created");

            if !style.is_empty() {
                element
                    .set_attribute("style", style)
                    .expect("fixture child style should be applied");
            }

            parent
                .append_child(&element)
                .expect("fixture child should be appended");

            element
        }

        fn append_child_to_node(parent: &web_sys::Node, style: &str) -> web_sys::Element {
            Self::append_tag_child_to_node(parent, "div", style)
        }
    }

    impl Drop for DomFixture {
        fn drop(&mut self) {
            if let Some(parent) = self.root.parent_node() {
                drop(parent.remove_child(&self.root));
            }
        }
    }

    fn assert_rect_matches_dom(rect: Rect, element: &web_sys::Element) {
        let dom_rect = element.get_bounding_client_rect();

        assert!((rect.x - dom_rect.x()).abs() < 0.01);
        assert!((rect.y - dom_rect.y()).abs() < 0.01);
        assert!((rect.width - dom_rect.width()).abs() < 0.01);
        assert!((rect.height - dom_rect.height()).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn transformed_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(12px); width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("transform ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn containing_block_lookup_uses_padding_box_origin() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(12px); width: 120px; height: 80px; border-left: 5px solid black; border-top: 7px solid black;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("transform ancestor should create a containing block");
        let dom_rect = ancestor.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0)).abs() < 0.01);
        assert!((rect.width - f64::from(ancestor.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(ancestor.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn containing_block_lookup_crosses_shadow_root_boundary() {
        let fixture = DomFixture::new();

        let host = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(12px); width: 120px; height: 80px; border-left: 5px solid black; border-top: 7px solid black;",
        );
        let shadow_root = host
            .attach_shadow(&web_sys::ShadowRootInit::new(web_sys::ShadowRootMode::Open))
            .expect("shadow root should attach");
        let child = DomFixture::append_child_to_node(
            shadow_root.unchecked_ref(),
            "width: 20px; height: 10px;",
        );

        let rect = find_containing_block_ancestor(&child)
            .expect("shadow host should create a containing block");
        let dom_rect = host.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0)).abs() < 0.01);
        assert!((rect.width - f64::from(host.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(host.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn containing_block_lookup_follows_slot_assignment_into_shadow_tree() {
        let fixture = DomFixture::new();

        let host = DomFixture::append_child(&fixture.root, "width: 160px; height: 120px;");
        let shadow_root = host
            .attach_shadow(&web_sys::ShadowRootInit::new(web_sys::ShadowRootMode::Open))
            .expect("shadow root should attach");
        let transformed = DomFixture::append_child_to_node(
            shadow_root.unchecked_ref(),
            "transform: translateX(12px); width: 120px; height: 80px; border-left: 5px solid black; border-top: 7px solid black;",
        );
        let _slot = DomFixture::append_tag_child_to_node(transformed.unchecked_ref(), "slot", "");
        let child = DomFixture::append_child(&host, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("slotted child should resolve composed containing block");
        let dom_rect = transformed.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0)).abs() < 0.01);
        assert!((rect.width - f64::from(transformed.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(transformed.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn scaled_transform_containing_block_returns_none_without_matrix_conversion() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: scale(2); transform-origin: 0 0; width: 120px; height: 80px;",
        );
        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        assert_eq!(find_containing_block_ancestor(&child), None);
    }

    #[wasm_bindgen_test]
    fn translate3d_transform_containing_block_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translate3d(12px, 18px, 0); width: 120px; height: 80px;",
        );
        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("translation-only matrix3d transform should stay axis-aligned");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn translatez_transform_containing_block_returns_none() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateZ(8px); width: 120px; height: 80px;",
        );
        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        assert_eq!(find_containing_block_ancestor(&child), None);
    }

    #[wasm_bindgen_test]
    fn containment_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor =
            DomFixture::append_child(&fixture.root, "contain: paint; width: 120px; height: 80px;");

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("contain ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn filter_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "filter: blur(2px); width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("filter ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn perspective_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "perspective: 500px; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("perspective ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn container_type_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "container-type: inline-size; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("container-type ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn content_visibility_auto_ancestor_is_detected() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "content-visibility: auto; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        let rect = find_containing_block_ancestor(&child)
            .expect("content-visibility ancestor should create a containing block");

        assert_rect_matches_dom(rect, &ancestor);
    }

    #[wasm_bindgen_test]
    fn containing_block_lookup_returns_none_when_no_matching_ancestor_exists() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "position: relative; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(&ancestor, "");

        assert_eq!(find_containing_block_ancestor(&child), None);
    }

    #[wasm_bindgen_test]
    fn offset_parent_rect_returns_positioned_ancestor_rect() {
        let fixture = DomFixture::new();

        let offset_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; left: 30px; top: 22px; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(
            &offset_parent,
            "position: absolute; left: 14px; top: 9px; width: 20px; height: 10px;",
        );

        let rect = offset_parent_rect(&child)
            .expect("positioned ancestor should be returned as the offset parent");

        assert_rect_matches_dom(rect, &offset_parent);
    }

    #[wasm_bindgen_test]
    fn offset_parent_rect_uses_padding_box_origin() {
        let fixture = DomFixture::new();

        let offset_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; left: 30px; top: 22px; width: 120px; height: 80px; border-left: 5px solid black; border-top: 7px solid black;",
        );

        let child = DomFixture::append_child(
            &offset_parent,
            "position: absolute; left: 14px; top: 9px; width: 20px; height: 10px;",
        );

        let rect = offset_parent_rect(&child)
            .expect("positioned ancestor should be returned as the offset parent");
        let dom_rect = offset_parent.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0)).abs() < 0.01);
        assert!((rect.width - f64::from(offset_parent.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(offset_parent.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn offset_parent_rect_accounts_for_scroll_position() {
        let fixture = DomFixture::new();

        let offset_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; left: 30px; top: 22px; width: 120px; height: 80px; overflow: auto; border-left: 5px solid black; border-top: 7px solid black;",
        );
        let _spacer = DomFixture::append_child(&offset_parent, "width: 400px; height: 400px;");

        offset_parent.set_scroll_left(11);
        offset_parent.set_scroll_top(13);

        let child = DomFixture::append_child(
            &offset_parent,
            "position: absolute; left: 14px; top: 9px; width: 20px; height: 10px;",
        );

        let rect = offset_parent_rect(&child)
            .expect("positioned ancestor should be returned as the offset parent");
        let dom_rect = offset_parent.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0 - 11.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0 - 13.0)).abs() < 0.01);
        assert!((rect.width - f64::from(offset_parent.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(offset_parent.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn offset_parent_rect_accepts_translation_only_matrix3d_transform() {
        let fixture = DomFixture::new();

        let offset_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; transform: translate3d(12px, 18px, 0); width: 120px; height: 80px; border-left: 5px solid black; border-top: 7px solid black;",
        );

        let child = DomFixture::append_child(
            &offset_parent,
            "position: absolute; left: 14px; top: 9px; width: 20px; height: 10px;",
        );

        let rect = offset_parent_rect(&child)
            .expect("translation-only matrix3d offset parent should stay axis-aligned");
        let dom_rect = offset_parent.get_bounding_client_rect();

        assert!((rect.x - (dom_rect.x() + 5.0)).abs() < 0.01);
        assert!((rect.y - (dom_rect.y() + 7.0)).abs() < 0.01);
        assert!((rect.width - f64::from(offset_parent.client_width())).abs() < 0.01);
        assert!((rect.height - f64::from(offset_parent.client_height())).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    fn offset_parent_rect_returns_none_for_scaled_transform() {
        let fixture = DomFixture::new();

        let offset_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; transform: scale(2); transform-origin: 0 0; width: 120px; height: 80px;",
        );

        let child = DomFixture::append_child(
            &offset_parent,
            "position: absolute; left: 14px; top: 9px; width: 20px; height: 10px;",
        );

        assert_eq!(offset_parent_rect(&child), None);
    }

    #[wasm_bindgen_test]
    fn portal_target_warning_helper_is_callable() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateY(6px); width: 120px; height: 80px;",
        );

        let target = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        warn_if_portal_target_has_containing_block(&target);
    }

    #[wasm_bindgen_test]
    fn portal_target_warning_helper_is_noop_without_containing_block() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "position: relative; width: 120px; height: 80px;",
        );

        let target = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        warn_if_portal_target_has_containing_block(&target);
    }

    #[wasm_bindgen_test]
    fn floating_containment_warning_helper_is_callable() {
        let fixture = DomFixture::new();

        let ancestor =
            DomFixture::append_child(&fixture.root, "contain: paint; width: 120px; height: 80px;");

        let floating = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        warn_if_floating_element_has_containment_issue(&floating);
    }

    #[wasm_bindgen_test]
    fn floating_containment_warning_helper_is_noop_without_containment() {
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "position: relative; width: 120px; height: 80px;",
        );

        let floating = DomFixture::append_child(&ancestor, "width: 20px; height: 10px;");

        warn_if_floating_element_has_containment_issue(&floating);
    }

    // -----------------------------------------------------------------
    // Browser tests for `measure_and_compute_position` — the bundled
    // pipeline helper that overlay adapters call to avoid duplicating
    // DOM measurement and containing-block / offset-parent coordinate
    // conversion (spec §2.3.1–§2.8.1, issue #595).
    // -----------------------------------------------------------------

    use crate::positioning::{
        compute::compute_position,
        types::{Boundary, Placement, PositioningOptions, Strategy},
    };

    fn pipeline_options(placement: Placement, strategy: Strategy) -> PositioningOptions {
        PositioningOptions {
            placement,
            strategy,
            flip: false,
            shift: false,
            ..PositioningOptions::default()
        }
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_uses_client_space_without_containing_block() {
        // No transform ancestor and no positioned parent: the pipeline must
        // operate in client space and place the floating element flush
        // against the anchor's bottom edge (Placement::Bottom, no offset).
        let fixture = DomFixture::new();

        let anchor = DomFixture::append_child(
            &fixture.root,
            "position: absolute; left: 40px; top: 60px; width: 100px; height: 30px;",
        );

        let floating = DomFixture::append_child(
            &fixture.root,
            "position: fixed; left: 0; top: 0; width: 80px; height: 20px;",
        );

        let options = pipeline_options(Placement::Bottom, Strategy::Fixed);

        let result = measure_and_compute_position(&anchor, &floating, &options)
            .expect("pipeline should succeed without a containing block ancestor");

        let anchor_rect = anchor.get_bounding_client_rect();

        // Bottom: y = anchor.bottom; x centered horizontally on anchor.
        assert!(
            (result.y - (anchor_rect.y() + anchor_rect.height())).abs() < 0.01,
            "floating should sit flush against anchor bottom in client space"
        );

        let expected_x = anchor_rect.x() + anchor_rect.width() / 2.0 - 80.0 / 2.0;

        assert!(
            (result.x - expected_x).abs() < 0.01,
            "floating should be horizontally centered on anchor in client space"
        );
        assert_eq!(result.actual_placement, Placement::Bottom);
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_uses_containing_block_local_space_for_translation_transform() {
        // A translateX ancestor creates a containing block for
        // `position: fixed` descendants. The pipeline must subtract the
        // ancestor's padding-box origin from anchor and viewport so the
        // result is expressed in ancestor-local space rather than client
        // space.
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(30px); position: relative; left: 20px; top: 15px; width: 400px; height: 300px;",
        );

        let anchor = DomFixture::append_child(
            &ancestor,
            "position: absolute; left: 80px; top: 50px; width: 60px; height: 24px;",
        );

        let floating = DomFixture::append_child(
            &ancestor,
            "position: fixed; left: 0; top: 0; width: 40px; height: 16px;",
        );

        let options = pipeline_options(Placement::Bottom, Strategy::Fixed);

        let result = measure_and_compute_position(&anchor, &floating, &options)
            .expect("pipeline should succeed for a translation-only containing block");

        let anchor_client = anchor.get_bounding_client_rect();

        let ancestor_client = ancestor.get_bounding_client_rect();

        // Expected: result coords are in ancestor-local space, so they're
        // smaller than the client-space anchor position by the ancestor's
        // origin.
        assert!(
            result.y < anchor_client.y() + anchor_client.height(),
            "containing-block-local y should be less than client-space anchor bottom"
        );

        // Specifically, the y should equal (anchor_client.bottom - ancestor_client.y) approximately.
        let expected_y = (anchor_client.y() + anchor_client.height()) - ancestor_client.y();

        assert!(
            (result.y - expected_y).abs() < 1.0,
            "expected {expected_y}, got {}",
            result.y
        );
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_returns_none_for_non_translation_transform() {
        // `transform: scale(...)` is a non-translation transform and cannot
        // be represented by axis-aligned origin subtraction. The pipeline
        // MUST return `None` so the adapter can fall back (typically by
        // portaling the overlay outside the transformed ancestor).
        let fixture = DomFixture::new();

        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: scale(2); position: relative; width: 400px; height: 300px;",
        );

        let anchor = DomFixture::append_child(
            &ancestor,
            "position: absolute; left: 20px; top: 20px; width: 40px; height: 20px;",
        );

        let floating =
            DomFixture::append_child(&ancestor, "position: fixed; width: 32px; height: 16px;");

        let options = pipeline_options(Placement::Bottom, Strategy::Fixed);

        assert_eq!(
            measure_and_compute_position(&anchor, &floating, &options),
            None
        );
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_uses_offset_parent_space_for_absolute_strategy() {
        // Non-portaled absolute positioning: floating element lives inside a
        // `position: relative` ancestor and uses `Strategy::Absolute`. The
        // pipeline must convert client-space coordinates into the offset
        // parent's scroll-adjusted padding-box space (spec §2.8).
        let fixture = DomFixture::new();

        let positioned_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; left: 10px; top: 12px; width: 400px; height: 300px;",
        );

        let anchor = DomFixture::append_child(
            &positioned_parent,
            "position: absolute; left: 60px; top: 45px; width: 80px; height: 24px;",
        );

        let floating = DomFixture::append_child(
            &positioned_parent,
            "position: absolute; width: 50px; height: 16px;",
        );

        let options = pipeline_options(Placement::BottomStart, Strategy::Absolute);

        let result = measure_and_compute_position(&anchor, &floating, &options)
            .expect("pipeline should succeed with a positioned ancestor for Strategy::Absolute");

        let anchor_client = anchor.get_bounding_client_rect();

        // In offset-parent-local space, the result's x/y are strictly less
        // than the client-space anchor coordinates (the parent has a
        // non-zero origin).
        assert!(
            result.x < anchor_client.x(),
            "offset-parent-local x should be smaller than client-space anchor x"
        );
        assert!(
            result.y < anchor_client.y() + anchor_client.height(),
            "offset-parent-local y should be smaller than client-space anchor bottom"
        );
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_returns_none_for_absolute_strategy_on_scaled_offset_parent() {
        // `transform: scale(...)` on the positioned ancestor makes that
        // ancestor BOTH the `offsetParent` for `position: absolute` descendants
        // AND a non-translation containing block. `offset_parent_rect` MUST
        // return `None` in that case (spec §2.8: "If the offset parent has a
        // non-translation transform, this rect-only helper MUST return None
        // instead of exposing a local origin derived from transformed
        // viewport-space coordinates."). The pipeline must propagate that
        // `None` so the adapter can portal out rather than mispositioning.
        let fixture = DomFixture::new();

        let positioned_parent = DomFixture::append_child(
            &fixture.root,
            "position: relative; transform: scale(2); width: 400px; height: 300px;",
        );

        let anchor = DomFixture::append_child(
            &positioned_parent,
            "position: absolute; left: 20px; top: 20px; width: 40px; height: 20px;",
        );

        let floating = DomFixture::append_child(
            &positioned_parent,
            "position: absolute; width: 32px; height: 16px;",
        );

        let options = pipeline_options(Placement::Bottom, Strategy::Absolute);

        assert_eq!(
            measure_and_compute_position(&anchor, &floating, &options),
            None
        );
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_returns_none_for_absolute_strategy_without_offset_parent() {
        // `position: fixed` on the floating element causes `offsetParent` to
        // be `null` whenever no ancestor creates a containing block for fixed
        // descendants — the most common case for portaled overlays. The
        // pipeline's `Strategy::Absolute` branch unwraps `offset_parent_rect`
        // with `?`, so this must surface as `None` rather than falling back
        // to client space (which would silently misposition the overlay).
        let fixture = DomFixture::new();

        let anchor = DomFixture::append_child(
            &fixture.root,
            "position: absolute; left: 40px; top: 60px; width: 100px; height: 30px;",
        );

        let floating = DomFixture::append_child(
            &fixture.root,
            "position: fixed; left: 0; top: 0; width: 80px; height: 20px;",
        );

        let options = pipeline_options(Placement::Bottom, Strategy::Absolute);

        assert_eq!(
            measure_and_compute_position(&anchor, &floating, &options),
            None
        );
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_crosses_shadow_root_boundary_for_containing_block() {
        // The containing-block walk used by `Strategy::Fixed` must cross
        // shadow-root boundaries via the composed tree (spec §2.3.1 "Step 0:
        // Detect Containing Block"). Here the transformed shadow host lives
        // in the light DOM, and both the anchor and the floating element
        // live inside its shadow root. The pipeline must find the host as
        // the containing block and return a result expressed in host-local
        // space rather than falling back to client space.
        let fixture = DomFixture::new();

        let host = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(30px); position: relative; left: 20px; top: 15px; width: 400px; height: 300px;",
        );

        let shadow_root = host
            .attach_shadow(&web_sys::ShadowRootInit::new(web_sys::ShadowRootMode::Open))
            .expect("shadow root should attach");

        let anchor = DomFixture::append_child_to_node(
            shadow_root.unchecked_ref(),
            "position: absolute; left: 80px; top: 50px; width: 60px; height: 24px;",
        );

        let floating = DomFixture::append_child_to_node(
            shadow_root.unchecked_ref(),
            "position: fixed; left: 0; top: 0; width: 40px; height: 16px;",
        );

        let options = pipeline_options(Placement::Bottom, Strategy::Fixed);

        let result = measure_and_compute_position(&anchor, &floating, &options)
            .expect("pipeline should resolve the transformed shadow host as containing block");

        let anchor_client = anchor.get_bounding_client_rect();

        let host_client = host.get_bounding_client_rect();

        // Host-local space: the result y equals (anchor_client.bottom -
        // host_client.y) ± border, which is strictly less than the
        // client-space anchor bottom whenever the host has a non-zero
        // origin.
        assert!(
            result.y < anchor_client.y() + anchor_client.height(),
            "shadow-host-local y should be less than client-space anchor bottom"
        );

        let expected_y = (anchor_client.y() + anchor_client.height()) - host_client.y();

        assert!(
            (result.y - expected_y).abs() < 1.0,
            "expected {expected_y}, got {}",
            result.y
        );
        assert_eq!(result.actual_placement, Placement::Bottom);
    }

    #[wasm_bindgen_test]
    fn measure_and_compute_position_converts_boundary_element_rect_to_local_space() {
        // Regression test for the `Boundary::Element` coordinate-space fix:
        // when the floating element's containing block has a non-zero local
        // origin (here, a translateX ancestor) and the caller uses an
        // explicit `Boundary::Element`, the pipeline MUST convert the
        // boundary's `getBoundingClientRect()` reading into local space
        // before handing it to the engine. Otherwise flip/shift/max-size
        // compare local-space anchor against a client-space boundary and
        // mispredict placement.
        //
        // Host tests can only exercise the fallback path because
        // `Boundary::Element` doesn't resolve to a real rect on non-wasm;
        // this browser test is the one that exercises the actual
        // DOM-resolved element boundary path.
        let fixture = DomFixture::new();

        // Transformed containing block — creates a non-zero local origin
        // for `position: fixed` descendants (spec §2.3.1 "Step 0").
        let ancestor = DomFixture::append_child(
            &fixture.root,
            "transform: translateX(30px); position: relative; left: 20px; top: 15px; width: 400px; height: 300px;",
        );

        // Scroll container used as the explicit overflow boundary.
        // `getBoundingClientRect()` on this element returns client-space
        // coordinates; the fix must run those through origin subtraction
        // so they match the anchor/viewport conversion the pipeline does.
        let scroller = DomFixture::append_child(
            &ancestor,
            "position: absolute; left: 40px; top: 50px; width: 200px; height: 150px;",
        );

        let anchor = DomFixture::append_child(
            &scroller,
            "position: absolute; left: 40px; top: 30px; width: 60px; height: 20px;",
        );

        let floating = DomFixture::append_child(
            &ancestor,
            "position: fixed; left: 0; top: 0; width: 80px; height: 20px;",
        );

        let scroller_handle: std::rc::Rc<dyn std::any::Any> = std::rc::Rc::new(scroller.clone());

        let options = PositioningOptions {
            placement: Placement::Bottom,
            strategy: Strategy::Fixed,
            boundary: Boundary::Element(scroller_handle),
            flip: true,
            shift: true,
            ..PositioningOptions::default()
        };

        let actual = measure_and_compute_position(&anchor, &floating, &options).expect(
            "pipeline should succeed with an element boundary under a transformed ancestor",
        );

        // Manually run the same pipeline to build the expected result:
        // 1. Resolve every input in client space via `get_bounding_client_rect()`.
        // 2. Subtract the containing block's local origin (padding-box, which
        //    equals client-box here because the ancestor has no border) from
        //    every rect.
        // 3. Call `compute_position` with `Boundary::Viewport` so the engine
        //    uses the already-resolved boundary rect as-is.
        let ancestor_client = ancestor.get_bounding_client_rect();
        let anchor_client = anchor.get_bounding_client_rect();
        let scroller_client = scroller.get_bounding_client_rect();
        let floating_client = floating.get_bounding_client_rect();

        let anchor_local = Rect {
            x: anchor_client.x() - ancestor_client.x(),
            y: anchor_client.y() - ancestor_client.y(),
            width: anchor_client.width(),
            height: anchor_client.height(),
        };

        let floating_dims = Rect {
            x: 0.0,
            y: 0.0,
            width: floating_client.width(),
            height: floating_client.height(),
        };

        let boundary_local = Rect {
            x: scroller_client.x() - ancestor_client.x(),
            y: scroller_client.y() - ancestor_client.y(),
            width: scroller_client.width(),
            height: scroller_client.height(),
        };

        let mut expected_options = options.clone();
        expected_options.boundary = Boundary::Viewport;

        let expected = compute_position(
            &anchor_local,
            &floating_dims,
            &boundary_local,
            &expected_options,
        );

        assert_eq!(
            actual, expected,
            "pipeline must convert Boundary::Element to local space before running the engine",
        );
    }
}
