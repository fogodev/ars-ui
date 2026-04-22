//! Item-scoped convenience wrapper for repeated component anatomy.

use crate::{ElementHandle, TestHarness};

/// Scoped handle for a collection item rendered within a [`TestHarness`].
#[derive(Debug)]
pub struct ItemHandle<'a> {
    harness: &'a TestHarness,
    element: ElementHandle,
}

impl<'a> ItemHandle<'a> {
    /// Queries a descendant of the item root with the given selector.
    #[must_use]
    pub fn query_selector(&self, selector: &str) -> Option<ElementHandle> {
        self.element.query_selector(selector)
    }

    /// Reads an attribute from the item's trigger element.
    #[must_use]
    pub fn trigger_attr(&self, attr: &str) -> Option<String> {
        self.query_selector("[data-ars-part='trigger']")
            .and_then(|trigger| trigger.attr(attr))
    }

    /// Returns the item's trigger element.
    #[must_use]
    pub fn trigger(&self) -> ElementHandle {
        self.query_selector("[data-ars-part='trigger']")
            .expect("item trigger must exist")
    }

    /// Clicks the item's trigger element.
    pub async fn click_trigger(&self) {
        let trigger = self.trigger();

        trigger.click().await;

        self.harness.flush().await;
    }

    /// Returns the text content of the item root.
    #[must_use]
    pub fn text_content(&self) -> String {
        self.element.text_content()
    }

    /// Returns whether the item root currently has focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.element.is_focused()
    }

    /// Focuses the item root element.
    pub async fn focus(&self) {
        self.element.focus().await;

        self.harness.flush().await;
    }

    /// Reads an attribute from the item root element.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<String> {
        self.element.attr(name)
    }

    pub(crate) const fn new(harness: &'a TestHarness, element: ElementHandle) -> Self {
        Self { harness, element }
    }
}
