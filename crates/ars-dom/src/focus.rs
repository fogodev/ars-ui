//! DOM-backed focus querying and focus-scope primitives.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use std::vec::Vec;
use std::{cell::RefCell, collections::HashMap, string::String};

use ars_a11y::{FocusScopeBehavior, FocusScopeOptions, FocusTarget};

/// CSS selector used to query focusable candidate elements before visibility and
/// inert-state filtering are applied.
///
/// This selector includes elements with an explicit `tabindex`, including `-1`,
/// which means the resulting candidate set may include elements that are
/// programmatically focusable but not tabbable.
pub const FOCUSABLE_SELECTOR: &str = concat!(
    "button:not([disabled]):not([aria-hidden='true']),",
    "input:not([disabled]):not([aria-hidden='true']),",
    "select:not([disabled]):not([aria-hidden='true']),",
    "textarea:not([disabled]):not([aria-hidden='true']),",
    "a[href]:not([aria-hidden='true']),",
    "area[href]:not([aria-hidden='true']),",
    "[tabindex]:not([disabled]):not([aria-hidden='true']),",
    "[contenteditable]:not([contenteditable='false']):not([aria-hidden='true'])",
);
/// CSS selector used to query tabbable candidate elements before visibility and
/// inert-state filtering are applied.
///
/// This selector excludes `tabindex="-1"` candidates so the result set maps to
/// the tab order rather than the broader focusable set.
pub const TABBABLE_SELECTOR: &str = concat!(
    "button:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "input:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "select:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "textarea:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "a[href]:not([tabindex='-1']):not([aria-hidden='true']),",
    "area[href]:not([tabindex='-1']):not([aria-hidden='true']),",
    "[tabindex]:not([tabindex='-1']):not([disabled]):not([aria-hidden='true']),",
    "[contenteditable]:not([contenteditable='false']):not([tabindex='-1']):not([aria-hidden='true'])",
);

/// Returns the raw selector string used to locate tabbable DOM candidates.
#[must_use]
pub fn get_tabbable_elements_selector() -> &'static str {
    TABBABLE_SELECTOR
}

/// Platform-agnostic reference to an element captured for later focus restoration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusedElement(pub String);

thread_local! {
    static PREVIOUSLY_ACTIVE_SCOPE_ELEMENTS: RefCell<HashMap<String, FocusedElement>> =
        RefCell::new(HashMap::new());
}

/// Manages focus within a bounded DOM region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusScope {
    /// Options controlling containment, restore, and auto-focus behavior.
    options: FocusScopeOptions,
    /// The previously focused element saved during activation.
    previously_focused: Option<FocusedElement>,
    /// Whether the scope is currently active.
    active: bool,
    /// The ID of the container element that bounds the scope.
    container_id: String,
}

impl FocusScopeBehavior for FocusScope {
    #[inline]
    fn activate(&mut self, focus_target: FocusTarget) {
        Self::activate(self, focus_target);
    }

    #[inline]
    fn deactivate(&mut self) {
        Self::deactivate(self);
    }

    #[inline]
    fn is_active(&self) -> bool {
        self.active
    }
}

impl FocusScope {
    /// Creates a new focus scope attached to the element with the given ID.
    #[must_use]
    pub fn new(container_id: impl Into<String>, options: FocusScopeOptions) -> Self {
        Self {
            options,
            previously_focused: None,
            active: false,
            container_id: container_id.into(),
        }
    }

    /// Activates the scope and optionally moves focus within it.
    pub fn activate(&mut self, focus_target: FocusTarget) {
        if self.active {
            return;
        }

        self.previously_focused = get_currently_focused();
        self.active = true;

        if self.options.auto_focus {
            self.focus_first(focus_target);
        }
    }

    /// Deactivates the scope and restores focus according to the spec fallback chain.
    pub fn deactivate(&mut self) {
        if !self.active {
            return;
        }

        if let Some(active_within_scope) =
            get_currently_focused().filter(|focused| self.contains_element(&focused.0))
        {
            store_previously_active_element(&self.container_id, active_within_scope);
        }

        self.active = false;

        if self.options.restore_focus {
            let previous_is_valid = self
                .previously_focused
                .as_ref()
                .is_some_and(is_element_in_dom);
            match resolve_restore_target(
                previous_is_valid,
                container_parent_exists(&self.container_id),
            ) {
                RestoreTarget::PreviouslyFocused => {
                    if let Some(previous) = self.previously_focused.as_ref() {
                        focus_focused_element(previous);
                    }
                }
                RestoreTarget::ContainerParent => focus_container_parent(&self.container_id),
                RestoreTarget::Body => focus_body(),
            }
        }

        self.previously_focused = None;
    }

    /// Handles a Tab or Shift+Tab key press for containment.
    ///
    /// Returns `true` when the event should be prevented.
    #[must_use]
    pub fn handle_tab_key(&self, shift: bool) -> bool {
        let current_index = current_tabbable_index(&self.container_id);
        let tabbable_count = tabbable_count(&self.container_id);

        match resolve_tab_navigation(
            self.active,
            self.options.contain,
            tabbable_count,
            current_index,
            shift,
        ) {
            TabNavigationAction::AllowBrowserDefault => false,
            TabNavigationAction::FocusContainer => {
                focus_container(&self.container_id);
                true
            }
            TabNavigationAction::FocusFirst => {
                focus_first_tabbable(&self.container_id);
                true
            }
            TabNavigationAction::FocusLast => {
                focus_last_tabbable(&self.container_id);
                true
            }
        }
    }

    /// Moves focus to the requested initial target within the scope.
    pub fn focus_first(&self, target: FocusTarget) {
        match target {
            FocusTarget::Last => {
                self.focus_last();
                return;
            }
            FocusTarget::AutofocusMarked => {
                if focus_autofocus_marked(&self.container_id) {
                    return;
                }
            }
            FocusTarget::PreviouslyActive => {
                if let Some(previous) = get_previously_active_element(&self.container_id)
                    && self.contains_element(&previous.0)
                {
                    focus_focused_element(&previous);
                    return;
                }
            }
            FocusTarget::First => {}
        }

        if !focus_first_tabbable_impl(&self.container_id) {
            focus_container(&self.container_id);
        }
    }

    /// Moves focus to the last tabbable element in the scope.
    pub fn focus_last(&self) {
        if !focus_last_tabbable_impl(&self.container_id) {
            focus_container(&self.container_id);
        }
    }

    /// Returns whether the element with `element_id` is inside this scope's container.
    #[must_use]
    pub fn contains_element(&self, element_id: &str) -> bool {
        contains_element_by_id(&self.container_id, element_id)
    }
}

impl Drop for FocusScope {
    fn drop(&mut self) {
        self.deactivate();
    }
}

/// Returns the ID of the document's currently focused element.
#[must_use]
fn active_element_id() -> Option<String> {
    active_element_id_impl()
}

/// Returns the document's currently focused element as a restorable handle.
#[must_use]
fn get_currently_focused() -> Option<FocusedElement> {
    active_element_id().map(FocusedElement)
}

/// Returns whether the element referenced by `element` is still connected to the document.
#[must_use]
fn is_element_in_dom(element: &FocusedElement) -> bool {
    if element.0.is_empty() {
        return false;
    }
    document_contains_id_impl(&element.0)
}

/// Focuses the element with the given DOM ID.
pub fn focus_element_by_id(id: &str) {
    let _ = focus_element_by_id_impl(id);
}

/// Focuses the first tabbable element inside the container with `container_id`.
pub fn focus_first_tabbable(container_id: &str) {
    let _ = focus_first_tabbable_impl(container_id);
}

/// Focuses the last tabbable element inside the container with `container_id`.
fn focus_last_tabbable(container_id: &str) {
    let _ = focus_last_tabbable_impl(container_id);
}

/// Focuses `document.body` as a last-resort fallback.
pub fn focus_body() {
    focus_body_impl();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsCast;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use web_sys::{Document, Element, HtmlElement, Window};

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn window() -> Option<Window> {
    web_sys::window()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<Document> {
    window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn active_html_element() -> Option<HtmlElement> {
    document()
        .and_then(|document| document.active_element())
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn active_element_id_impl() -> Option<String> {
    active_html_element()
        .and_then(|element| element.get_attribute("id"))
        .filter(|id| !id.is_empty())
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn active_element_id_impl() -> Option<String> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document_contains_id_impl(id: &str) -> bool {
    get_element_by_id(id).is_some_and(|element| element.is_connected())
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn document_contains_id_impl(_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_focused_element(element: &FocusedElement) {
    let _ = focus_element_by_id_impl(&element.0);
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_focused_element(_element: &FocusedElement) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn container_parent_exists(container_id: &str) -> bool {
    get_html_element_by_id(container_id)
        .and_then(|element| element.parent_element())
        .is_some()
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn container_parent_exists(_container_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_container_parent(container_id: &str) {
    if let Some(parent) =
        get_html_element_by_id(container_id).and_then(|element| element.parent_element())
        && let Ok(parent) = parent.dyn_into::<HtmlElement>()
    {
        focus_element(&parent, false);
        return;
    }

    focus_body();
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_container_parent(_container_id: &str) {
    focus_body();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_container(container_id: &str) {
    if let Some(container) = get_html_element_by_id(container_id) {
        focus_element(&container, false);
    }
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_container(_container_id: &str) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn current_tabbable_index(container_id: &str) -> Option<usize> {
    let container = get_element_by_id(container_id)?;
    let tabbables = get_tabbable_elements(&container);
    let active = active_html_element()?;

    tabbables.iter().position(|candidate| candidate == &active)
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn current_tabbable_index(_container_id: &str) -> Option<usize> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn tabbable_count(container_id: &str) -> usize {
    get_element_by_id(container_id).map_or(0, |container| get_tabbable_elements(&container).len())
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn tabbable_count(_container_id: &str) -> usize {
    0
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_autofocus_marked(container_id: &str) -> bool {
    let Some(container) = get_element_by_id(container_id) else {
        return false;
    };
    let Some(marked) = container
        .query_selector("[data-ars-autofocus]")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    else {
        return false;
    };

    if is_focusable_element(&marked) {
        focus_element(&marked, false);
        return true;
    }

    false
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_autofocus_marked(_container_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_element_by_id_impl(id: &str) -> bool {
    get_html_element_by_id(id)
        .filter(is_focusable_element)
        .map(|element| {
            focus_element(&element, false);
        })
        .is_some()
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_element_by_id_impl(_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_first_tabbable_impl(container_id: &str) -> bool {
    let Some(container) = get_element_by_id(container_id) else {
        return false;
    };
    let Some(first) = get_tabbable_elements(&container).into_iter().next() else {
        return false;
    };

    focus_element(&first, false);
    true
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_first_tabbable_impl(_container_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_last_tabbable_impl(container_id: &str) -> bool {
    let Some(container) = get_element_by_id(container_id) else {
        return false;
    };
    let Some(last) = get_tabbable_elements(&container).into_iter().last() else {
        return false;
    };

    focus_element(&last, false);
    true
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_last_tabbable_impl(_container_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_body_impl() {
    if let Some(body) = document().and_then(|document| document.body()) {
        focus_element(&body, false);
    }
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn focus_body_impl() {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn contains_element_by_id(container_id: &str, element_id: &str) -> bool {
    let Some(container) = get_element_by_id(container_id) else {
        return false;
    };
    let Some(element) = get_element_by_id(element_id) else {
        return false;
    };

    container.contains(Some(element.as_ref()))
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn contains_element_by_id(_container_id: &str, _element_id: &str) -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn collect_candidates(container: &Element, selector: &str) -> Vec<HtmlElement> {
    let Ok(nodes) = container.query_selector_all(selector) else {
        return Vec::new();
    };

    let mut elements = Vec::new();
    for index in 0..nodes.length() {
        if let Some(node) = nodes.item(index)
            && let Ok(element) = node.dyn_into::<HtmlElement>()
        {
            elements.push(element);
        }
    }

    elements
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_focusable_element(element: &HtmlElement) -> bool {
    if !element.is_connected()
        || is_hidden(element)
        || has_inert_ancestor(element)
        || has_aria_hidden_ancestor(element)
        || is_inside_closed_details(element)
    {
        return false;
    }

    if is_disabled(element) {
        return false;
    }

    if element.has_attribute("tabindex") {
        return true;
    }

    match element.tag_name().as_str() {
        "BUTTON" | "INPUT" | "SELECT" | "TEXTAREA" => true,
        "A" | "AREA" => element.has_attribute("href"),
        _ => element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false"),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_tabbable_element(element: &HtmlElement) -> bool {
    is_focusable_element(element) && element.tab_index() >= 0
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_disabled(element: &HtmlElement) -> bool {
    element.has_attribute("disabled")
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_hidden(element: &HtmlElement) -> bool {
    let Some(window) = window() else {
        return true;
    };
    let Ok(Some(style)) = window.get_computed_style(element) else {
        return true;
    };

    let display = style.get_property_value("display").unwrap_or_default();
    let visibility = style.get_property_value("visibility").unwrap_or_default();

    display == "none" || visibility == "hidden"
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn has_inert_ancestor(element: &HtmlElement) -> bool {
    let mut current = Some(element.clone().unchecked_into::<Element>());
    while let Some(node) = current {
        if node.has_attribute("inert") {
            return true;
        }
        current = node.parent_element();
    }
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn has_aria_hidden_ancestor(element: &HtmlElement) -> bool {
    let mut current = Some(element.clone().unchecked_into::<Element>());
    while let Some(node) = current {
        if node
            .get_attribute("aria-hidden")
            .is_some_and(|value| value == "true")
        {
            return true;
        }
        current = node.parent_element();
    }
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_inside_closed_details(element: &HtmlElement) -> bool {
    let mut current = element.parent_element();
    while let Some(node) = current {
        if node.tag_name() == "DETAILS" && !node.has_attribute("open") {
            return true;
        }
        current = node.parent_element();
    }
    false
}

/// Looks up a DOM element by ID.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub(crate) fn get_element_by_id(id: &str) -> Option<Element> {
    document().and_then(|document| document.get_element_by_id(id))
}

/// Looks up a DOM element by ID and downcasts it to [`HtmlElement`].
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn get_html_element_by_id(id: &str) -> Option<HtmlElement> {
    get_element_by_id(id).and_then(|element| element.dyn_into::<HtmlElement>().ok())
}

/// Queries focusable elements within `container` in DOM order.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn get_focusable_elements(container: &Element) -> Vec<HtmlElement> {
    collect_candidates(container, FOCUSABLE_SELECTOR)
        .into_iter()
        .filter(is_focusable_element)
        .collect()
}

/// Queries tabbable elements within `container`, ordered by tabindex and DOM position.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub(crate) fn get_tabbable_elements(container: &Element) -> Vec<HtmlElement> {
    let mut tabbables = collect_candidates(container, TABBABLE_SELECTOR)
        .into_iter()
        .filter(is_tabbable_element)
        .enumerate()
        .collect::<Vec<_>>();

    tabbables.sort_by(|(left_index, left), (right_index, right)| {
        let left_tabindex = left.tab_index();
        let right_tabindex = right.tab_index();
        let left_priority = if left_tabindex > 0 { 0 } else { 1 };
        let right_priority = if right_tabindex > 0 { 0 } else { 1 };

        left_priority
            .cmp(&right_priority)
            .then_with(|| left_tabindex.cmp(&right_tabindex))
            .then_with(|| left_index.cmp(right_index))
    });

    tabbables.into_iter().map(|(_, element)| element).collect()
}

/// Returns the first focusable element inside `container`.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn get_first_focusable(container: &Element) -> Option<HtmlElement> {
    get_focusable_elements(container).into_iter().next()
}

/// Returns the last focusable element inside `container`.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn get_last_focusable(container: &Element) -> Option<HtmlElement> {
    get_focusable_elements(container).into_iter().last()
}

/// Focuses `element`, optionally preventing scroll.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn focus_element(element: &HtmlElement, prevent_scroll: bool) {
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(prevent_scroll);
    drop(element.focus_with_options(&options));
}

/// Returns whether `element` is still contained in the current document.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn document_contains(element: &HtmlElement) -> bool {
    document().is_some_and(|document| document.contains(Some(element.as_ref())))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreTarget {
    PreviouslyFocused,
    ContainerParent,
    Body,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabNavigationAction {
    AllowBrowserDefault,
    FocusContainer,
    FocusFirst,
    FocusLast,
}

fn resolve_restore_target(previous_is_valid: bool, has_container_parent: bool) -> RestoreTarget {
    if previous_is_valid {
        RestoreTarget::PreviouslyFocused
    } else if has_container_parent {
        RestoreTarget::ContainerParent
    } else {
        RestoreTarget::Body
    }
}

fn resolve_tab_navigation(
    active: bool,
    contain: bool,
    tabbable_count: usize,
    current_index: Option<usize>,
    shift: bool,
) -> TabNavigationAction {
    if !active || !contain {
        return TabNavigationAction::AllowBrowserDefault;
    }

    if tabbable_count == 0 {
        return TabNavigationAction::FocusContainer;
    }

    match current_index {
        Some(0) | None if shift => TabNavigationAction::FocusLast,
        Some(index) if !shift && index + 1 == tabbable_count => TabNavigationAction::FocusFirst,
        None => TabNavigationAction::FocusFirst,
        Some(_) => TabNavigationAction::AllowBrowserDefault,
    }
}

fn store_previously_active_element(container_id: &str, element: FocusedElement) {
    PREVIOUSLY_ACTIVE_SCOPE_ELEMENTS.with(|elements| {
        elements
            .borrow_mut()
            .insert(String::from(container_id), element);
    });
}

fn get_previously_active_element(container_id: &str) -> Option<FocusedElement> {
    PREVIOUSLY_ACTIVE_SCOPE_ELEMENTS.with(|elements| elements.borrow().get(container_id).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_scope_new_starts_inactive_with_expected_fields() {
        let scope = FocusScope::new("dialog-root", FocusScopeOptions::default());

        assert_eq!(scope.container_id, "dialog-root");
        assert_eq!(scope.options, FocusScopeOptions::default());
        assert_eq!(scope.previously_focused, None);
        assert!(!scope.active);
    }

    #[test]
    fn focus_scope_activate_and_deactivate_are_safe_on_host() {
        let mut scope = FocusScope::new("dialog-root", FocusScopeOptions::default());

        scope.activate(FocusTarget::First);
        assert!(scope.active);
        assert_eq!(scope.previously_focused, None);
        assert!(scope.is_active());

        scope.deactivate();
        assert!(!scope.active);
        assert_eq!(scope.previously_focused, None);
        assert!(!scope.is_active());
    }

    #[test]
    fn focus_scope_focus_helpers_are_host_safe() {
        let scope = FocusScope::new("dialog-root", FocusScopeOptions::default());

        scope.focus_first(FocusTarget::First);
        scope.focus_first(FocusTarget::Last);
        scope.focus_first(FocusTarget::AutofocusMarked);
        scope.focus_first(FocusTarget::PreviouslyActive);
        scope.focus_last();

        assert!(!scope.contains_element("child"));
    }

    #[test]
    fn host_focus_query_helpers_return_safe_defaults() {
        let focused = FocusedElement(String::from("field"));

        assert_eq!(active_element_id(), None);
        assert_eq!(get_currently_focused(), None);
        assert!(!is_element_in_dom(&focused));
        assert!(!document_contains_id_impl("field"));
        assert_eq!(active_element_id_impl(), None);
        assert_eq!(current_tabbable_index("dialog-root"), None);
        assert_eq!(tabbable_count("dialog-root"), 0);
        assert!(!focus_autofocus_marked("dialog-root"));
        assert!(!focus_element_by_id_impl("field"));
        assert!(!focus_first_tabbable_impl("dialog-root"));
        assert!(!focus_last_tabbable_impl("dialog-root"));
        assert!(!contains_element_by_id("dialog-root", "field"));
    }

    #[test]
    fn host_focus_side_effect_helpers_are_safe_to_call() {
        focus_focused_element(&FocusedElement(String::from("field")));
        focus_container_parent("dialog-root");
        focus_container("dialog-root");
        focus_body_impl();
        focus_element_by_id("field");
        focus_first_tabbable("dialog-root");
        focus_last_tabbable("dialog-root");
        focus_body();
    }

    #[test]
    fn restore_target_prefers_previous_then_parent_then_body() {
        assert_eq!(
            resolve_restore_target(true, true),
            RestoreTarget::PreviouslyFocused
        );
        assert_eq!(
            resolve_restore_target(false, true),
            RestoreTarget::ContainerParent
        );
        assert_eq!(resolve_restore_target(false, false), RestoreTarget::Body);
    }

    #[test]
    fn tab_navigation_wraps_at_scope_boundaries() {
        assert_eq!(
            resolve_tab_navigation(true, true, 3, Some(2), false),
            TabNavigationAction::FocusFirst
        );
        assert_eq!(
            resolve_tab_navigation(true, true, 3, Some(0), true),
            TabNavigationAction::FocusLast
        );
    }

    #[test]
    fn tab_navigation_focuses_container_when_no_tabbables_exist() {
        assert_eq!(
            resolve_tab_navigation(true, true, 0, None, false),
            TabNavigationAction::FocusContainer
        );
        assert_eq!(
            resolve_tab_navigation(true, true, 0, None, true),
            TabNavigationAction::FocusContainer
        );
    }

    #[test]
    fn tab_navigation_focuses_boundary_when_active_element_is_unknown() {
        assert_eq!(
            resolve_tab_navigation(true, true, 2, None, false),
            TabNavigationAction::FocusFirst
        );
        assert_eq!(
            resolve_tab_navigation(true, true, 2, None, true),
            TabNavigationAction::FocusLast
        );
    }

    #[test]
    fn tab_navigation_allows_browser_default_inside_scope() {
        assert_eq!(
            resolve_tab_navigation(true, true, 4, Some(1), false),
            TabNavigationAction::AllowBrowserDefault
        );
        assert_eq!(
            resolve_tab_navigation(false, true, 4, Some(3), false),
            TabNavigationAction::AllowBrowserDefault
        );
        assert_eq!(
            resolve_tab_navigation(true, false, 4, Some(0), true),
            TabNavigationAction::AllowBrowserDefault
        );
    }

    #[test]
    fn tab_navigation_shift_with_non_boundary_keeps_browser_default() {
        assert_eq!(
            resolve_tab_navigation(true, true, 4, Some(2), true),
            TabNavigationAction::AllowBrowserDefault
        );
    }

    #[test]
    fn tabbable_selector_keeps_focusable_ordering_contract() {
        assert!(TABBABLE_SELECTOR.contains("button:not([disabled])"));
        assert!(TABBABLE_SELECTOR.contains("[tabindex]:not([tabindex='-1'])"));
        assert!(FOCUSABLE_SELECTOR.contains("[tabindex]:not([disabled])"));
        assert!(!FOCUSABLE_SELECTOR.contains("[tabindex]:not([tabindex='-1'])"));
    }

    #[test]
    fn get_tabbable_elements_selector_returns_public_constant() {
        assert_eq!(get_tabbable_elements_selector(), TABBABLE_SELECTOR);
    }

    #[test]
    fn previously_active_elements_are_stored_per_scope() {
        store_previously_active_element("dialog-a", FocusedElement(String::from("input-a")));
        store_previously_active_element("dialog-b", FocusedElement(String::from("input-b")));

        assert_eq!(
            get_previously_active_element("dialog-a"),
            Some(FocusedElement(String::from("input-a")))
        );
        assert_eq!(
            get_previously_active_element("dialog-b"),
            Some(FocusedElement(String::from("input-b")))
        );
    }

    #[test]
    fn previously_active_lookup_returns_none_for_unknown_scope() {
        assert_eq!(get_previously_active_element("missing-scope"), None);
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, Element, HtmlElement};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &Element, id: &str, style: &str) -> HtmlElement {
        let element = document()
            .create_element("div")
            .expect("div creation must succeed")
            .dyn_into::<HtmlElement>()
            .expect("div must be HtmlElement");
        element.set_id(id);
        element
            .set_attribute("style", style)
            .expect("style assignment must succeed");
        parent
            .append_child(&element)
            .expect("append_child must succeed");
        element
    }

    fn append_button(
        parent: &Element,
        id: &str,
        tabindex: Option<&str>,
        disabled: bool,
    ) -> HtmlElement {
        let button = document()
            .create_element("button")
            .expect("button creation must succeed")
            .dyn_into::<HtmlElement>()
            .expect("button must be HtmlElement");
        button.set_id(id);
        if let Some(tabindex) = tabindex {
            button
                .set_attribute("tabindex", tabindex)
                .expect("tabindex assignment must succeed");
        }
        if disabled {
            button
                .set_attribute("disabled", "")
                .expect("disabled assignment must succeed");
        }
        parent
            .append_child(&button)
            .expect("append_child must succeed");
        button
    }

    fn append_focusable_div(parent: &Element, id: &str, tabindex: &str) -> HtmlElement {
        let div = document()
            .create_element("div")
            .expect("div creation must succeed")
            .dyn_into::<HtmlElement>()
            .expect("div must be HtmlElement");
        div.set_id(id);
        div.set_attribute("tabindex", tabindex)
            .expect("tabindex assignment must succeed");
        parent
            .append_child(&div)
            .expect("append_child must succeed");
        div
    }

    fn cleanup(node: &HtmlElement) {
        node.remove();
    }

    #[wasm_bindgen_test]
    fn focus_queries_distinguish_focusable_from_tabbable_candidates() {
        let root = append_div(
            body().as_ref(),
            "focus-query-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );
        let button = append_button(root.as_ref(), "native-button", None, false);
        let programmatic_only = append_focusable_div(root.as_ref(), "programmatic-only", "-1");
        let tabbable = append_focusable_div(root.as_ref(), "tabbable-div", "0");
        let _disabled = append_button(root.as_ref(), "disabled-button", None, true);

        let focusable_ids = get_focusable_elements(root.as_ref())
            .into_iter()
            .map(|element| element.id())
            .collect::<Vec<_>>();
        let tabbable_ids = get_tabbable_elements(root.as_ref())
            .into_iter()
            .map(|element| element.id())
            .collect::<Vec<_>>();

        assert_eq!(
            focusable_ids,
            vec![button.id(), programmatic_only.id(), tabbable.id(),]
        );
        assert_eq!(tabbable_ids, vec![button.id(), tabbable.id()]);
        assert_eq!(
            get_first_focusable(root.as_ref()).map(|element| element.id()),
            Some(button.id())
        );
        assert_eq!(
            get_last_focusable(root.as_ref()).map(|element| element.id()),
            Some(tabbable.id())
        );
        assert!(get_html_element_by_id("programmatic-only").is_some());

        cleanup(&root);
    }

    fn active_id() -> Option<String> {
        document()
            .active_element()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            .map(|element| element.id())
            .filter(|id| !id.is_empty())
    }

    #[wasm_bindgen_test]
    fn focus_scope_activate_auto_focuses_first_and_deactivate_restores_previous() {
        let root = append_div(
            body().as_ref(),
            "scope-activate-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let outside = append_button(root.as_ref(), "scope-activate-outside", None, false);

        let container = append_div(root.as_ref(), "scope-activate-container", "");

        let inner_first = append_button(container.as_ref(), "scope-activate-first", None, false);
        let _inner_second = append_button(container.as_ref(), "scope-activate-second", None, false);

        focus_element(&outside, false);

        assert_eq!(active_id(), Some(outside.id()));

        let mut scope = FocusScope::new(
            "scope-activate-container",
            FocusScopeOptions {
                contain: true,
                restore_focus: true,
                auto_focus: true,
            },
        );

        scope.activate(FocusTarget::First);

        assert!(scope.is_active());
        assert_eq!(active_id(), Some(inner_first.id()));

        scope.deactivate();

        assert!(!scope.is_active());
        assert_eq!(active_id(), Some(outside.id()));

        // Double-deactivate is a no-op.
        scope.deactivate();

        assert!(!scope.is_active());

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn focus_scope_handle_tab_key_contained_wraps_at_boundaries() {
        let root = append_div(
            body().as_ref(),
            "scope-tab-wrap-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let container = append_div(root.as_ref(), "scope-tab-wrap-container", "");

        let first = append_button(container.as_ref(), "scope-tab-wrap-first", None, false);
        let _middle = append_button(container.as_ref(), "scope-tab-wrap-middle", None, false);
        let last = append_button(container.as_ref(), "scope-tab-wrap-last", None, false);

        let mut scope = FocusScope::new(
            "scope-tab-wrap-container",
            FocusScopeOptions {
                contain: true,
                restore_focus: false,
                auto_focus: true,
            },
        );
        scope.activate(FocusTarget::First);

        assert_eq!(active_id(), Some(first.id()));

        // Shift+Tab at the first tabbable wraps to the last.
        focus_element(&first, false);

        assert!(scope.handle_tab_key(true));
        assert_eq!(active_id(), Some(last.id()));

        // Tab at the last tabbable wraps to the first.
        focus_element(&last, false);

        assert!(scope.handle_tab_key(false));
        assert_eq!(active_id(), Some(first.id()));

        scope.deactivate();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn focus_scope_handle_tab_key_uncontained_allows_browser_default() {
        let root = append_div(
            body().as_ref(),
            "scope-tab-free-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let container = append_div(root.as_ref(), "scope-tab-free-container", "");

        let first = append_button(container.as_ref(), "scope-tab-free-first", None, false);
        let last = append_button(container.as_ref(), "scope-tab-free-last", None, false);

        let mut scope = FocusScope::new(
            "scope-tab-free-container",
            FocusScopeOptions {
                contain: false,
                restore_focus: false,
                auto_focus: false,
            },
        );

        // Inactive scope short-circuits; verify first, then flip `active`
        // so the remaining assertions exercise the
        // `active && !contain` arm of `resolve_tab_navigation`.
        focus_element(&first, false);
        assert!(!scope.handle_tab_key(false));
        scope.activate(FocusTarget::First);
        assert!(scope.is_active());

        // Active but uncontained scope still allows browser default navigation.
        focus_element(&first, false);

        assert!(!scope.handle_tab_key(false));

        focus_element(&last, false);

        assert!(!scope.handle_tab_key(true));

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn focus_scope_focus_first_honors_autofocus_and_previously_active_targets() {
        let root = append_div(
            body().as_ref(),
            "scope-targets-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let container = append_div(root.as_ref(), "scope-targets-container", "");

        let plain = append_button(container.as_ref(), "scope-targets-plain", None, false);

        let autofocus = append_button(container.as_ref(), "scope-targets-autofocus", None, false);

        autofocus
            .set_attribute("data-ars-autofocus", "")
            .expect("autofocus marker must set");

        let scope = FocusScope::new(
            "scope-targets-container",
            FocusScopeOptions {
                contain: false,
                restore_focus: false,
                auto_focus: false,
            },
        );

        // AutofocusMarked prefers the marked candidate over the first tabbable.
        scope.focus_first(FocusTarget::AutofocusMarked);

        assert_eq!(active_id(), Some(autofocus.id()));

        // focus_last explicitly jumps to the last tabbable.
        scope.focus_last();

        assert_eq!(active_id(), Some(autofocus.id()));

        // PreviouslyActive restores a stored candidate inside the scope.
        store_previously_active_element("scope-targets-container", FocusedElement(plain.id()));

        scope.focus_first(FocusTarget::PreviouslyActive);

        assert_eq!(active_id(), Some(plain.id()));

        // Unknown target falls back to the first tabbable.
        scope.focus_first(FocusTarget::First);

        assert_eq!(active_id(), Some(plain.id()));

        cleanup(&root);
    }
}
