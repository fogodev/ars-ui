//! Scroll management utilities for browser-backed adapters.

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use core::fmt;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use js_sys::Reflect;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsValue;

/// Options controlling how an element should be brought into view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollIntoViewOptions {
    /// Vertical alignment preference within the scrollable viewport.
    pub block: Option<ScrollLogicalPosition>,
    /// Horizontal alignment preference within the scrollable viewport.
    pub inline: Option<ScrollLogicalPosition>,
    /// Whether smooth scrolling should be requested when native APIs are available.
    pub smooth: bool,
}

impl Default for ScrollIntoViewOptions {
    fn default() -> Self {
        Self {
            block: Some(ScrollLogicalPosition::Nearest),
            inline: Some(ScrollLogicalPosition::Nearest),
            smooth: false,
        }
    }
}

/// Logical alignment positions for scrolling an item into view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollLogicalPosition {
    /// Align the leading edge of the item with the leading edge of the container.
    Start,
    /// Center the item in the container.
    Center,
    /// Align the trailing edge of the item with the trailing edge of the container.
    End,
    /// Scroll only when needed and only by the minimum required offset.
    Nearest,
}

/// Returns whether the current runtime supports `scrollIntoView(options)`.
#[must_use]
pub fn supports_scroll_into_view_options() -> bool {
    supports_scroll_into_view_options_impl()
}

#[cfg(feature = "web")]
/// Returns the nearest ancestor that can scroll the given element into view.
pub fn nearest_scrollable_ancestor(element: &web_sys::Element) -> Option<web_sys::Element> {
    nearest_scrollable_ancestor_impl(element)
}

#[cfg(feature = "web")]
/// Returns all scrollable ancestors from nearest to furthest, including the document root once.
pub fn scrollable_ancestors(element: &web_sys::Element) -> Vec<web_sys::Element> {
    scrollable_ancestors_impl(element)
}

#[cfg(feature = "web")]
/// Scrolls an element into view, using a manual fallback when native options support is absent.
pub fn scroll_into_view_if_needed(element: &web_sys::Element, options: ScrollIntoViewOptions) {
    scroll_into_view_if_needed_impl(element, options);
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Clone, Copy, PartialEq)]
struct AxisBounds {
    start: f64,
    end: f64,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
impl fmt::Debug for AxisBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AxisBounds")
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn is_scrollable_axis_value(value: &str) -> bool {
    matches!(value.trim(), "auto" | "scroll" | "hidden")
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn has_scrollable_overflow(
    overflow_x: &str,
    overflow_y: &str,
    scroll_width: i32,
    client_width: i32,
    scroll_height: i32,
    client_height: i32,
) -> bool {
    (is_scrollable_axis_value(overflow_y) && scroll_height > client_height)
        || (is_scrollable_axis_value(overflow_x) && scroll_width > client_width)
}

#[cfg(test)]
fn find_nearest_matching_ancestor(
    start: usize,
    parents: &[Option<usize>],
    matches: &[bool],
) -> Option<usize> {
    let mut current = parents[start];
    while let Some(index) = current {
        if matches[index] {
            return Some(index);
        }
        current = parents[index];
    }
    None
}

#[cfg(test)]
fn collect_matching_ancestors(
    start: usize,
    parents: &[Option<usize>],
    matches: &[bool],
    root: Option<usize>,
) -> Vec<usize> {
    let mut ancestors = Vec::new();
    let mut current = find_nearest_matching_ancestor(start, parents, matches);

    while let Some(index) = current {
        ancestors.push(index);
        current = find_nearest_matching_ancestor(index, parents, matches);
    }

    if let Some(root_index) = root
        && ancestors.last().copied() != Some(root_index)
    {
        ancestors.push(root_index);
    }

    ancestors
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn target_scroll_offset(
    position: ScrollLogicalPosition,
    element: AxisBounds,
    container: AxisBounds,
    current_scroll: f64,
) -> Option<f64> {
    match position {
        ScrollLogicalPosition::Start => Some(current_scroll + (element.start - container.start)),
        ScrollLogicalPosition::Center => {
            let element_center = (element.start + element.end) / 2.0;
            let container_center = (container.start + container.end) / 2.0;
            Some(current_scroll + (element_center - container_center))
        }
        ScrollLogicalPosition::End => Some(current_scroll + (element.end - container.end)),
        ScrollLogicalPosition::Nearest => {
            if element.start < container.start {
                Some(current_scroll - (container.start - element.start))
            } else if element.end > container.end {
                Some(current_scroll + (element.end - container.end))
            } else {
                None
            }
        }
    }
}

#[must_use]
fn supports_scroll_into_view_options_impl() -> bool {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return false;
        };
        let Ok(_element) = document.create_element("div") else {
            return false;
        };

        Reflect::get(
            &js_sys::global(),
            &JsValue::from_str("ScrollIntoViewOptions"),
        )
        .map(|value| !value.is_undefined())
        .unwrap_or(false)
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
fn nearest_scrollable_ancestor_impl(element: &web_sys::Element) -> Option<web_sys::Element> {
    let window = web_sys::window()?;
    let mut current = element.parent_element();

    while let Some(ancestor) = current {
        let style = window.get_computed_style(&ancestor).ok().flatten()?;
        let overflow_x = style.get_property_value("overflow-x").unwrap_or_default();
        let overflow_y = style.get_property_value("overflow-y").unwrap_or_default();

        if has_scrollable_overflow(
            &overflow_x,
            &overflow_y,
            ancestor.scroll_width(),
            ancestor.client_width(),
            ancestor.scroll_height(),
            ancestor.client_height(),
        ) {
            return Some(ancestor);
        }

        current = ancestor.parent_element();
    }

    None
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
#[must_use]
fn nearest_scrollable_ancestor_impl(_element: &web_sys::Element) -> Option<web_sys::Element> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
fn scrollable_ancestors_impl(element: &web_sys::Element) -> Vec<web_sys::Element> {
    let mut ancestors = Vec::new();
    let mut current = nearest_scrollable_ancestor_impl(element);

    while let Some(ancestor) = current {
        ancestors.push(ancestor.clone());
        current = nearest_scrollable_ancestor_impl(&ancestor);
    }

    if let Some(document_root) = element
        .owner_document()
        .and_then(|document| document.document_element())
        && ancestors
            .last()
            .is_none_or(|last| !last.is_same_node(Some(&document_root)))
    {
        ancestors.push(document_root);
    }

    ancestors
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
#[must_use]
fn scrollable_ancestors_impl(_element: &web_sys::Element) -> Vec<web_sys::Element> {
    Vec::new()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn scroll_into_view_if_needed_impl(element: &web_sys::Element, options: ScrollIntoViewOptions) {
    if supports_scroll_into_view_options_impl() {
        let web_options = web_scroll_into_view_options(options);
        element.scroll_into_view_with_scroll_into_view_options(&web_options);
        return;
    }

    let ancestors = scrollable_ancestors_impl(element);
    if ancestors.is_empty() {
        element.scroll_into_view_with_bool(true);
        return;
    }

    let block = options.block.unwrap_or(ScrollLogicalPosition::Nearest);
    let inline = options.inline.unwrap_or(ScrollLogicalPosition::Nearest);

    for ancestor in ancestors {
        let element_rect = element.get_bounding_client_rect();
        let ancestor_rect = ancestor.get_bounding_client_rect();

        if let Some(target_top) = target_scroll_offset(
            block,
            AxisBounds {
                start: element_rect.top(),
                end: element_rect.bottom(),
            },
            AxisBounds {
                start: ancestor_rect.top(),
                end: ancestor_rect.bottom(),
            },
            f64::from(ancestor.scroll_top()),
        ) {
            ancestor.set_scroll_top(target_top.round() as i32);
        }

        if let Some(target_left) = target_scroll_offset(
            inline,
            AxisBounds {
                start: element_rect.left(),
                end: element_rect.right(),
            },
            AxisBounds {
                start: ancestor_rect.left(),
                end: ancestor_rect.right(),
            },
            f64::from(ancestor.scroll_left()),
        ) {
            ancestor.set_scroll_left(target_left.round() as i32);
        }
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn scroll_into_view_if_needed_impl(_element: &web_sys::Element, _options: ScrollIntoViewOptions) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
fn web_scroll_into_view_options(options: ScrollIntoViewOptions) -> web_sys::ScrollIntoViewOptions {
    let web_options = web_sys::ScrollIntoViewOptions::new();
    web_options.set_behavior(if options.smooth {
        web_sys::ScrollBehavior::Smooth
    } else {
        web_sys::ScrollBehavior::Auto
    });
    web_options.set_block(
        match options.block.unwrap_or(ScrollLogicalPosition::Nearest) {
            ScrollLogicalPosition::Start => web_sys::ScrollLogicalPosition::Start,
            ScrollLogicalPosition::Center => web_sys::ScrollLogicalPosition::Center,
            ScrollLogicalPosition::End => web_sys::ScrollLogicalPosition::End,
            ScrollLogicalPosition::Nearest => web_sys::ScrollLogicalPosition::Nearest,
        },
    );
    web_options.set_inline(
        match options.inline.unwrap_or(ScrollLogicalPosition::Nearest) {
            ScrollLogicalPosition::Start => web_sys::ScrollLogicalPosition::Start,
            ScrollLogicalPosition::Center => web_sys::ScrollLogicalPosition::Center,
            ScrollLogicalPosition::End => web_sys::ScrollLogicalPosition::End,
            ScrollLogicalPosition::Nearest => web_sys::ScrollLogicalPosition::Nearest,
        },
    );
    web_options
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "web", not(target_arch = "wasm32")))]
    use wasm_bindgen::{JsCast, JsValue};

    use super::*;

    #[test]
    fn supports_scroll_into_view_options_is_false_without_browser_dom() {
        assert!(!supports_scroll_into_view_options());
    }

    #[test]
    fn scroll_into_view_options_default_matches_spec() {
        assert_eq!(
            ScrollIntoViewOptions::default(),
            ScrollIntoViewOptions {
                block: Some(ScrollLogicalPosition::Nearest),
                inline: Some(ScrollLogicalPosition::Nearest),
                smooth: false,
            }
        );
    }

    #[test]
    fn nearest_ancestor_finds_first_scrollable_parent() {
        let parents = [None, Some(0), Some(1), Some(2), Some(2)];
        let matches = [false, false, true, false, true];

        assert_eq!(
            find_nearest_matching_ancestor(3, &parents, &matches),
            Some(2)
        );
        assert_eq!(
            find_nearest_matching_ancestor(4, &parents, &matches),
            Some(2)
        );
    }

    #[test]
    fn nearest_ancestor_returns_none_without_scrollable_parent() {
        let parents = [None, Some(0), Some(1), Some(2)];
        let matches = [false, false, false, false];

        assert_eq!(find_nearest_matching_ancestor(3, &parents, &matches), None);
    }

    #[test]
    fn scrollability_requires_overflow_style_and_actual_extra_content() {
        assert!(has_scrollable_overflow("auto", "visible", 200, 100, 80, 80));
        assert!(has_scrollable_overflow(
            "visible", "scroll", 80, 80, 200, 100
        ));
        assert!(!has_scrollable_overflow(
            "visible", "visible", 200, 100, 200, 100
        ));
        assert!(!has_scrollable_overflow(
            "auto", "scroll", 100, 100, 100, 100
        ));
    }

    #[test]
    fn nearest_alignment_scrolls_up_when_item_is_above_viewport() {
        let result = target_scroll_offset(
            ScrollLogicalPosition::Nearest,
            AxisBounds {
                start: 50.0,
                end: 80.0,
            },
            AxisBounds {
                start: 100.0,
                end: 200.0,
            },
            150.0,
        );

        assert_eq!(result, Some(100.0));
    }

    #[test]
    fn nearest_alignment_scrolls_down_when_item_is_below_viewport() {
        let result = target_scroll_offset(
            ScrollLogicalPosition::Nearest,
            AxisBounds {
                start: 240.0,
                end: 320.0,
            },
            AxisBounds {
                start: 100.0,
                end: 200.0,
            },
            150.0,
        );

        assert_eq!(result, Some(270.0));
    }

    #[test]
    fn nearest_alignment_scrolls_horizontally_when_item_is_outside_viewport() {
        let result = target_scroll_offset(
            ScrollLogicalPosition::Nearest,
            AxisBounds {
                start: 260.0,
                end: 340.0,
            },
            AxisBounds {
                start: 100.0,
                end: 250.0,
            },
            30.0,
        );

        assert_eq!(result, Some(120.0));
    }

    #[test]
    fn nearest_alignment_keeps_scroll_position_when_item_is_fully_visible() {
        let result = target_scroll_offset(
            ScrollLogicalPosition::Nearest,
            AxisBounds {
                start: 120.0,
                end: 180.0,
            },
            AxisBounds {
                start: 100.0,
                end: 200.0,
            },
            75.0,
        );

        assert_eq!(result, None);
    }

    #[test]
    fn explicit_alignment_modes_compute_expected_offsets() {
        let element = AxisBounds {
            start: 120.0,
            end: 180.0,
        };
        let container = AxisBounds {
            start: 100.0,
            end: 300.0,
        };

        assert_eq!(
            target_scroll_offset(ScrollLogicalPosition::Start, element, container, 50.0),
            Some(70.0)
        );
        assert_eq!(
            target_scroll_offset(ScrollLogicalPosition::Center, element, container, 50.0),
            Some(0.0)
        );
        assert_eq!(
            target_scroll_offset(ScrollLogicalPosition::End, element, container, 50.0),
            Some(-70.0)
        );
    }

    #[test]
    fn nested_ancestor_collection_preserves_nearest_to_furthest_order_and_appends_root_once() {
        let parents = [None, Some(0), Some(1), Some(2), Some(3)];
        let matches = [false, true, false, true, false];

        assert_eq!(
            collect_matching_ancestors(4, &parents, &matches, Some(0)),
            vec![3, 1, 0]
        );
        assert_eq!(
            collect_matching_ancestors(4, &parents, &matches, Some(1)),
            vec![3, 1]
        );
    }

    #[test]
    fn axis_bounds_debug_includes_named_fields() {
        let debug = format!(
            "{:?}",
            AxisBounds {
                start: 1.0,
                end: 2.0,
            }
        );
        assert!(debug.contains("AxisBounds"));
        assert!(debug.contains("start"));
        assert!(debug.contains("end"));
    }

    #[cfg(all(feature = "web", not(target_arch = "wasm32")))]
    fn dummy_element() -> web_sys::Element {
        JsValue::NULL.unchecked_into()
    }

    #[cfg(all(feature = "web", not(target_arch = "wasm32")))]
    #[test]
    fn non_wasm_web_stubs_are_safe_to_call() {
        let element = dummy_element();

        assert_eq!(nearest_scrollable_ancestor(&element), None);
        assert!(scrollable_ancestors(&element).is_empty());
        scroll_into_view_if_needed(&element, ScrollIntoViewOptions::default());
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> web_sys::HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &web_sys::Element, style: &str) -> web_sys::HtmlElement {
        let element = document()
            .create_element("div")
            .expect("element creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("div must be HtmlElement");
        element
            .set_attribute("style", style)
            .expect("style assignment must succeed");
        parent
            .append_child(&element)
            .expect("append_child must succeed");
        element
    }

    fn cleanup(node: &web_sys::HtmlElement) {
        node.remove();
    }

    #[wasm_bindgen_test]
    fn supports_scroll_into_view_options_reports_browser_support() {
        assert!(supports_scroll_into_view_options());
    }

    #[wasm_bindgen_test]
    fn nearest_scrollable_ancestor_finds_the_immediate_scroll_container() {
        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:300px;height:300px;",
        );
        let outer = append_div(
            root.as_ref(),
            "width:220px;height:220px;overflow:auto;border:0;padding:0;margin:0;",
        );
        let inner = append_div(outer.as_ref(), "width:200px;height:500px;");
        let target = append_div(inner.as_ref(), "width:100px;height:20px;margin-top:260px;");

        let found = nearest_scrollable_ancestor(target.as_ref()).expect("must find ancestor");
        assert!(found.is_same_node(Some(outer.as_ref())));

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn scrollable_ancestors_collects_nearest_first_and_includes_document_root() {
        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:300px;height:300px;",
        );
        let outer = append_div(
            root.as_ref(),
            "width:220px;height:220px;overflow:auto;border:0;padding:0;margin:0;",
        );
        let inner_scroll = append_div(
            outer.as_ref(),
            "width:180px;height:180px;overflow:auto;border:0;padding:0;margin:0;",
        );
        let content = append_div(inner_scroll.as_ref(), "width:160px;height:500px;");
        let target = append_div(
            content.as_ref(),
            "width:100px;height:20px;margin-top:260px;",
        );

        let ancestors = scrollable_ancestors(target.as_ref());
        assert!(
            ancestors
                .first()
                .is_some_and(|ancestor| ancestor.is_same_node(Some(inner_scroll.as_ref())))
        );
        assert!(
            ancestors
                .get(1)
                .is_some_and(|ancestor| ancestor.is_same_node(Some(outer.as_ref())))
        );
        let document_root = document()
            .document_element()
            .expect("document element must exist");
        assert!(
            ancestors
                .last()
                .is_some_and(|ancestor| ancestor.is_same_node(Some(document_root.as_ref())))
        );

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn scroll_into_view_if_needed_moves_scroll_position() {
        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:300px;height:300px;",
        );
        let outer = append_div(
            root.as_ref(),
            "width:220px;height:120px;overflow:auto;border:0;padding:0;margin:0;",
        );
        let content = append_div(outer.as_ref(), "width:200px;height:500px;");
        let target = append_div(
            content.as_ref(),
            "width:100px;height:20px;margin-top:320px;",
        );

        outer.set_scroll_top(0);
        scroll_into_view_if_needed(target.as_ref(), ScrollIntoViewOptions::default());
        assert!(outer.scroll_top() > 0);

        cleanup(&root);
    }
}
