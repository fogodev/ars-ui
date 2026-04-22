//! DOM-backed coordinate-space helpers for floating positioning.
//!
//! These helpers bridge browser DOM measurements into the pure positioning
//! engine by detecting containing-block ancestors, resolving offset-parent
//! coordinate space, and converting client-space geometry into local-space
//! geometry.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsCast;

use super::Rect;

/// Convert a client-space point into the local coordinate space of `local_origin`.
#[must_use]
pub fn client_point_to_local_space(x: f64, y: f64, local_origin: &Rect) -> (f64, f64) {
    (x - local_origin.x, y - local_origin.y)
}

/// Convert a client-space rectangle into the local coordinate space of `local_origin`.
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
/// create a containing block.
#[cfg(feature = "web")]
#[must_use]
pub fn find_containing_block_ancestor(element: &web_sys::Element) -> Option<Rect> {
    find_containing_block_ancestor_impl(element)
}

/// Returns the scroll-adjusted padding-box rect of the element's offset parent
/// when available.
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
    walk_containing_block_ancestors(element).map(|(rect, _)| rect)
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn find_containing_block_ancestor_impl(_element: &web_sys::Element) -> Option<Rect> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn offset_parent_rect_impl(element: &web_sys::Element) -> Option<Rect> {
    let element = element.dyn_ref::<web_sys::HtmlElement>()?;
    let offset_parent = element.offset_parent()?;
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
    if let Some((_, reason)) = walk_containing_block_ancestors(target) {
        crate::debug::warn_message(format_args!(
            "Portal target has ancestor with {} which breaks position:fixed positioning. Overlays may be mispositioned. Move the portal target outside this ancestor.",
            reason.property_name()
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
fn walk_containing_block_ancestors(
    element: &web_sys::Element,
) -> Option<(Rect, ContainingBlockReason)> {
    let window = web_sys::window()?;

    let mut current = next_composed_ancestor_element(element.unchecked_ref());

    while let Some(ancestor) = current {
        let Some(style) = window.get_computed_style(&ancestor).ok().flatten() else {
            current = next_composed_ancestor_element(ancestor.unchecked_ref());

            continue;
        };

        let snapshot = style_snapshot(&style);

        if let Some(reason) = containing_block_reason(&snapshot) {
            return Some((padding_box_rect(&ancestor), reason));
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
fn next_composed_ancestor_element(node: &web_sys::Node) -> Option<web_sys::Element> {
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

        fn append_child_to_node(parent: &web_sys::Node, style: &str) -> web_sys::Element {
            let document = parent
                .owner_document()
                .expect("fixture parent should have a document");

            let element = document
                .create_element("div")
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
}
