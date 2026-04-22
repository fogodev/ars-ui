//! DOM element wrapper used by the shared test harness.

use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::Rect;

/// Wrapper around a DOM element for test assertions.
#[derive(Clone, Debug)]
pub struct ElementHandle {
    #[cfg(target_arch = "wasm32")]
    pub(crate) element: web_sys::Element,

    #[cfg(not(target_arch = "wasm32"))]
    stub: NativeElementStub,
}

impl ElementHandle {
    /// Reads an attribute value from the wrapped element.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            self.element.get_attribute(name)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.attrs.get(name).cloned()
        }
    }

    /// Returns the text content of the wrapped element.
    #[must_use]
    pub fn text_content(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            self.element.text_content().unwrap_or_default()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.text.clone()
        }
    }

    /// Returns the inner HTML of the wrapped element.
    #[must_use]
    pub fn inner_html(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            self.element.inner_html()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.inner_html.clone()
        }
    }

    /// Returns the current layout rectangle of the wrapped element.
    #[must_use]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::missing_const_for_fn,
            reason = "the wasm build reads runtime layout state"
        )
    )]
    pub fn bounding_rect(&self) -> Rect {
        #[cfg(target_arch = "wasm32")]
        {
            let rect = self.element.get_bounding_client_rect();

            Rect {
                x: rect.x(),
                y: rect.y(),
                width: rect.width(),
                height: rect.height(),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.rect
        }
    }

    /// Returns computed styles for the wrapped element.
    #[must_use]
    pub fn computed_styles(&self) -> HashMap<String, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut styles = HashMap::new();

            let window = web_sys::window().expect("window must exist for computed style lookup");

            let Some(computed) = window
                .get_computed_style(&self.element)
                .expect("computed style lookup must not throw")
            else {
                return styles;
            };

            let length = computed.length();

            for index in 0..length {
                let name = computed.item(index);

                let value = computed
                    .get_property_value(&name)
                    .unwrap_or_else(|_| String::new());
                styles.insert(name, value);
            }

            styles
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.styles.clone()
        }
    }

    /// Returns whether the wrapped element currently has focus.
    #[must_use]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::missing_const_for_fn,
            reason = "the wasm build reads runtime focus state"
        )
    )]
    pub fn is_focused(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.active_element())
                .is_some_and(|active| active == self.element)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.focused
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) const fn from_element(element: web_sys::Element) -> Self {
        Self { element }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "const construction is not important for native test stubs"
    )]
    pub(crate) fn from_stub(stub: NativeElementStub) -> Self {
        Self { stub }
    }

    #[must_use]
    pub(crate) fn query_selector(&self, selector: &str) -> Option<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            self.element
                .query_selector(selector)
                .expect("scoped query selector must not throw")
                .map(Self::from_element)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stub.children.get(selector).cloned()
        }
    }

    pub(crate) async fn click(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(html) = self.element.dyn_ref::<web_sys::HtmlElement>() {
                html.click();

                return;
            }

            let event = web_sys::MouseEvent::new("click").expect("click event must construct");

            let dispatched = self
                .element
                .dispatch_event(&event)
                .expect("click dispatch must succeed");

            let _ = dispatched;
        }
    }

    pub(crate) async fn focus(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(html) = self.element.dyn_ref::<web_sys::HtmlElement>() {
                html.focus().expect("focus should succeed");
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct NativeElementStub {
    attrs: HashMap<String, String>,
    text: String,
    inner_html: String,
    rect: Rect,
    styles: HashMap<String, String>,
    focused: bool,
    children: HashMap<String, ElementHandle>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeElementStub {
    #[cfg(test)]
    pub(crate) fn with_attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(name.into(), value.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn with_inner_html(mut self, inner_html: impl Into<String>) -> Self {
        self.inner_html = inner_html.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn with_style(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.styles.insert(name.into(), value.into());
        self
    }

    #[cfg(test)]
    pub(crate) const fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }

    #[cfg(test)]
    pub(crate) const fn with_focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_child(
        mut self,
        selector: impl Into<String>,
        element: ElementHandle,
    ) -> Self {
        self.children.insert(selector.into(), element);
        self
    }
}
