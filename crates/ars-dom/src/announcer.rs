//! DOM bootstrap for shared live-region announcer nodes.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use ars_a11y::LiveAnnouncer;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use ars_core::{AttrMap, AttrValue, HtmlAttr};

/// Ensure the shared live-region DOM nodes exist.
#[cfg(feature = "web")]
pub fn ensure_dom() {
    ensure_dom_impl();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn ensure_dom_impl() {
    let Some(document) = document() else {
        crate::debug::warn_skipped("ensure_dom()", "window.document");
        return;
    };
    let Some(body) = document.body() else {
        crate::debug::warn_skipped("ensure_dom()", "document.body");
        return;
    };

    ensure_region(
        &document,
        body.as_ref(),
        &LiveAnnouncer::polite_region_attrs(),
    );
    ensure_region(
        &document,
        body.as_ref(),
        &LiveAnnouncer::assertive_region_attrs(),
    );
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn ensure_dom_impl() {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn ensure_region(document: &web_sys::Document, parent: &web_sys::Element, attrs: &AttrMap) {
    let id = attrs
        .get(&HtmlAttr::Id)
        .expect("live region attrs must include an id");

    let element = if let Some(existing) = document.get_element_by_id(id) {
        existing
    } else {
        let created = document
            .create_element("div")
            .expect("live region creation must succeed");
        parent
            .append_child(&created)
            .expect("live region append must succeed");
        created
    };

    apply_attr_map(&element, attrs);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_attr_map(element: &web_sys::Element, attrs: &AttrMap) {
    for (attr, value) in attrs.attrs() {
        apply_attr_value(element, *attr, value);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_attr_value(element: &web_sys::Element, attr: HtmlAttr, value: &AttrValue) {
    match value {
        AttrValue::String(value) => {
            crate::debug::warn_dom_error(
                &format!("setting live region attribute {attr}"),
                element.set_attribute(&attr.to_string(), value),
            );
        }
        AttrValue::Bool(true) => {
            crate::debug::warn_dom_error(
                &format!("setting live region boolean attribute {attr}"),
                element.set_attribute(&attr.to_string(), ""),
            );
        }
        AttrValue::Bool(false) | AttrValue::None => {
            crate::debug::warn_dom_error(
                &format!("removing live region attribute {attr}"),
                element.remove_attribute(&attr.to_string()),
            );
        }
    }
}

#[cfg(all(test, feature = "web", not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;

    #[test]
    fn ensure_dom_is_host_safe_and_idempotent() {
        ensure_dom();
        ensure_dom();
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        super::document().expect("document must exist")
    }

    fn remove_live_regions() {
        if let Some(region) = document().get_element_by_id("ars-live-polite") {
            region.remove();
        }
        if let Some(region) = document().get_element_by_id("ars-live-assertive") {
            region.remove();
        }
    }

    fn region(id: &str) -> web_sys::Element {
        document()
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("missing region {id}"))
    }

    #[wasm_bindgen_test]
    fn ensure_dom_creates_both_live_regions_under_body() {
        remove_live_regions();

        ensure_dom();

        let polite = region("ars-live-polite");
        let assertive = region("ars-live-assertive");
        let body = document().body().expect("body must exist");

        assert!(body.contains(Some(polite.as_ref())));
        assert!(body.contains(Some(assertive.as_ref())));

        remove_live_regions();
    }

    #[wasm_bindgen_test]
    fn ensure_dom_creates_regions_with_expected_attributes() {
        remove_live_regions();

        ensure_dom();

        let polite = region("ars-live-polite");
        let assertive = region("ars-live-assertive");

        assert_eq!(polite.get_attribute("aria-live").as_deref(), Some("polite"));
        assert_eq!(polite.get_attribute("aria-atomic").as_deref(), Some("true"));
        assert_eq!(
            polite.get_attribute("data-ars-part").as_deref(),
            Some("live-region")
        );
        assert_eq!(
            polite.get_attribute("class").as_deref(),
            Some("ars-visually-hidden")
        );

        assert_eq!(
            assertive.get_attribute("aria-live").as_deref(),
            Some("assertive")
        );
        assert_eq!(
            assertive.get_attribute("aria-atomic").as_deref(),
            Some("true")
        );
        assert_eq!(
            assertive.get_attribute("aria-relevant").as_deref(),
            Some("additions text")
        );
        assert_eq!(
            assertive.get_attribute("data-ars-part").as_deref(),
            Some("live-region")
        );
        assert_eq!(
            assertive.get_attribute("class").as_deref(),
            Some("ars-visually-hidden")
        );

        remove_live_regions();
    }

    #[wasm_bindgen_test]
    fn ensure_dom_is_idempotent() {
        remove_live_regions();

        ensure_dom();
        ensure_dom();

        let regions = document()
            .query_selector_all("#ars-live-polite, #ars-live-assertive")
            .expect("selector must be valid");
        assert_eq!(regions.length(), 2);

        remove_live_regions();
    }
}
