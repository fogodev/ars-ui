//! Framework-agnostic adapter test harness for ars-ui components.
//!
//! This crate owns the shared, backend-agnostic contract for mounting rendered
//! adapter components into an isolated DOM container, querying anatomy parts,
//! simulating user input, and inspecting state through a type-erased service.

use std::{
    any::{Any, type_name},
    cell::RefCell,
    collections::BTreeMap,
    fmt::{self, Debug},
    str::FromStr,
    time::Duration,
};

use ars_core::{AttrMap, ComponentPart, ConnectApi, Machine, Service};
use ars_i18n::Locale;
#[cfg(all(test, not(target_arch = "wasm32")))]
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use {
    std::{cell::Cell, rc::Rc},
    wasm_bindgen::{JsCast, closure::Closure},
};

mod backend;
mod element;
mod item;
mod types;

pub use backend::HarnessBackend;
pub use element::ElementHandle;
#[cfg(all(test, not(target_arch = "wasm32")))]
use element::NativeElementStub;
pub use item::ItemHandle;
pub use types::{KeyboardKey, Point, Rect, point};

/// Marker trait for adapter component types mountable by the test harness.
pub trait Component: 'static {
    /// The framework-agnostic machine type wrapped by the adapter component.
    type Machine: Machine;
}

/// Type-erased wrapper around a component [`Service`].
pub trait AnyService {
    /// Returns the current machine state as a debug string.
    fn state_debug(&self) -> String;

    /// Returns the root attributes produced by the current connect snapshot.
    fn root_attrs(&self) -> AttrMap;

    /// Returns the attributes for the named component part.
    fn part_attrs(&self, part: &str) -> AttrMap;

    /// Parses and dispatches an event by name.
    fn send_named(&mut self, event_name: &str);

    /// Dispatches a boxed concrete event after downcasting it to the machine event type.
    fn send_boxed(&mut self, event: Box<dyn Any>);
}

fn noop_send<E>(_event: E) {}

impl<M> AnyService for Service<M>
where
    M: Machine,
    M::Event: FromStr + 'static,
{
    fn state_debug(&self) -> String {
        format!("{:?}", self.state())
    }

    fn root_attrs(&self) -> AttrMap {
        let api = M::connect(
            self.state(),
            self.context(),
            self.props(),
            &noop_send::<M::Event>,
        );

        api.part_attrs(<M::Api<'_> as ConnectApi>::Part::ROOT)
    }

    fn part_attrs(&self, part: &str) -> AttrMap {
        let api = M::connect(
            self.state(),
            self.context(),
            self.props(),
            &noop_send::<M::Event>,
        );

        for candidate in <M::Api<'_> as ConnectApi>::Part::all() {
            if candidate.name() == part {
                return api.part_attrs(candidate);
            }
        }

        panic!(
            "part_attrs: no part named '{part}' found for {}",
            type_name::<M>()
        );
    }

    fn send_named(&mut self, event_name: &str) {
        let event = event_name.parse::<M::Event>().unwrap_or_else(|_| {
            panic!("unknown event name for {}: {event_name}", type_name::<M>())
        });

        drop(self.send(event));
    }

    fn send_boxed(&mut self, event: Box<dyn Any>) {
        if let Ok(event) = event.downcast::<M::Event>() {
            drop(self.send(*event));
        } else {
            panic!(
                "boxed event type mismatch for {}; expected {}",
                type_name::<M>(),
                type_name::<M::Event>()
            );
        }
    }
}

/// Extension trait for typed part access on concrete services.
///
/// This is an extension trait rather than an inherent method because Rust does
/// not allow downstream crates to add inherent methods to external types.
pub trait ServiceHarnessExt<M: Machine> {
    /// Returns attributes for a concrete part value, preserving any data carried by the part.
    fn part_attrs_typed<'a>(&'a self, part: <M::Api<'a> as ConnectApi>::Part) -> AttrMap;
}

impl<M: Machine> ServiceHarnessExt<M> for Service<M> {
    fn part_attrs_typed<'a>(&'a self, part: <M::Api<'a> as ConnectApi>::Part) -> AttrMap {
        let api = M::connect(
            self.state(),
            self.context(),
            self.props(),
            &noop_send::<M::Event>,
        );

        api.part_attrs(part)
    }
}

enum ContainerHandle {
    #[cfg(target_arch = "wasm32")]
    Wasm(web_sys::HtmlElement),

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "constructed only by native unit-test helpers")
    )]
    Native {
        #[cfg(test)]
        element: web_sys::HtmlElement,
    },
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct WasmLayoutState {
    viewport: Option<ViewportOverride>,
    media_emulation: Option<MediaEmulation>,
    anchor: Option<ElementRectOverride>,
    last_root_pointer_point: Option<Point>,
    primary_pointer_button_down: bool,
    last_root_touch_point: Option<Point>,
    last_touch_target: Option<TouchTarget>,
    scroll_x: i32,
    scroll_y: i32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TouchTarget {
    Root,
    Trigger,
}

#[cfg(target_arch = "wasm32")]
struct ViewportOverride {
    width: Rc<Cell<f64>>,
    height: Rc<Cell<f64>>,
    container_rect: ElementRectOverride,

    _window_width_getter: Closure<dyn FnMut() -> JsValue>,
    _window_height_getter: Closure<dyn FnMut() -> JsValue>,
    _document_width_getter: Closure<dyn FnMut() -> JsValue>,
    _document_height_getter: Closure<dyn FnMut() -> JsValue>,

    original_window_width_descriptor: JsValue,
    original_window_height_descriptor: JsValue,
    original_document_width_descriptor: JsValue,
    original_document_height_descriptor: JsValue,
}

#[cfg(target_arch = "wasm32")]
impl ViewportOverride {
    fn install(container: &web_sys::HtmlElement, width: f64, height: f64) -> Self {
        let window = web_sys::window().expect("window must exist");

        let document = window.document().expect("document must exist");

        let document_element = document
            .document_element()
            .expect("document element must exist");

        let original_window_width_descriptor =
            own_property_descriptor(window.as_ref(), "innerWidth");
        let original_window_height_descriptor =
            own_property_descriptor(window.as_ref(), "innerHeight");
        let original_document_width_descriptor =
            own_property_descriptor(document_element.as_ref(), "clientWidth");
        let original_document_height_descriptor =
            own_property_descriptor(document_element.as_ref(), "clientHeight");

        let width_state = Rc::new(Cell::new(width));
        let height_state = Rc::new(Cell::new(height));

        let window_width_getter =
            define_numeric_getter(window.as_ref(), "innerWidth", Rc::clone(&width_state));
        let window_height_getter =
            define_numeric_getter(window.as_ref(), "innerHeight", Rc::clone(&height_state));

        let document_width_getter = define_numeric_getter(
            document_element.as_ref(),
            "clientWidth",
            Rc::clone(&width_state),
        );
        let document_height_getter = define_numeric_getter(
            document_element.as_ref(),
            "clientHeight",
            Rc::clone(&height_state),
        );

        let mut container_rect = install_rect_override(
            container.as_ref(),
            Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
        );

        container_rect.set_base_rect(
            Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
            0,
            0,
        );

        let viewport = Self {
            width: width_state,
            height: height_state,
            container_rect,
            _window_width_getter: window_width_getter,
            _window_height_getter: window_height_getter,
            _document_width_getter: document_width_getter,
            _document_height_getter: document_height_getter,
            original_window_width_descriptor,
            original_window_height_descriptor,
            original_document_width_descriptor,
            original_document_height_descriptor,
        };

        viewport.apply_container_styles(container);

        viewport
    }

    fn set_size(&mut self, container: &web_sys::HtmlElement, width: f64, height: f64) {
        self.width.set(width);

        self.height.set(height);

        self.container_rect.set_base_rect(
            Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
            0,
            0,
        );

        self.apply_container_styles(container);
    }

    fn apply_container_styles(&self, container: &web_sys::HtmlElement) {
        container
            .style()
            .set_property("position", "relative")
            .expect("setting harness container position should succeed");

        container
            .style()
            .set_property("overflow", "auto")
            .expect("setting harness container overflow should succeed");

        container
            .style()
            .set_property("width", &format!("{}px", self.width.get()))
            .expect("setting harness container width should succeed");

        container
            .style()
            .set_property("height", &format!("{}px", self.height.get()))
            .expect("setting harness container height should succeed");
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for ViewportOverride {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            restore_property_descriptor(
                window.as_ref(),
                "innerWidth",
                &self.original_window_width_descriptor,
            );

            restore_property_descriptor(
                window.as_ref(),
                "innerHeight",
                &self.original_window_height_descriptor,
            );

            if let Some(document_element) = window
                .document()
                .and_then(|document| document.document_element())
            {
                restore_property_descriptor(
                    document_element.as_ref(),
                    "clientWidth",
                    &self.original_document_width_descriptor,
                );

                restore_property_descriptor(
                    document_element.as_ref(),
                    "clientHeight",
                    &self.original_document_height_descriptor,
                );
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct ElementRectOverride {
    element: web_sys::Element,
    current_rect: Rc<RefCell<Rect>>,
    _getter: Closure<dyn FnMut() -> JsValue>,
    base_rect: Rect,
    original_descriptor: JsValue,
}

#[cfg(target_arch = "wasm32")]
impl ElementRectOverride {
    fn set_base_rect(&mut self, rect: Rect, scroll_x: i32, scroll_y: i32) {
        self.base_rect = rect;

        self.update_for_scroll(scroll_x, scroll_y);
    }

    fn update_for_scroll(&self, scroll_x: i32, scroll_y: i32) {
        *self.current_rect.borrow_mut() = Rect {
            x: self.base_rect.x - f64::from(scroll_x),
            y: self.base_rect.y - f64::from(scroll_y),
            width: self.base_rect.width,
            height: self.base_rect.height,
        };
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for ElementRectOverride {
    fn drop(&mut self) {
        restore_property_descriptor(
            self.element.as_ref(),
            "getBoundingClientRect",
            &self.original_descriptor,
        );
    }
}

#[cfg(target_arch = "wasm32")]
struct MediaEmulation {
    overrides: Rc<RefCell<BTreeMap<String, String>>>,
    original_match_media_descriptor: JsValue,
    _match_media: Closure<dyn FnMut(JsValue) -> JsValue>,
    _listener_noop: Closure<dyn FnMut()>,
    _dispatch_event: Closure<dyn FnMut(JsValue) -> bool>,
}

#[cfg(target_arch = "wasm32")]
impl MediaEmulation {
    fn install() -> Self {
        let window = web_sys::window().expect("window must exist");
        let overrides = Rc::new(RefCell::new(BTreeMap::new()));
        let listener_noop = Closure::<dyn FnMut()>::wrap(Box::new(|| {}));
        let dispatch_event = Closure::<dyn FnMut(JsValue) -> bool>::wrap(Box::new(|_| true));
        let original_match_media_descriptor =
            own_property_descriptor(window.as_ref(), "matchMedia");
        let original_match_media =
            js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
                .expect("window.matchMedia should be readable")
                .dyn_into::<js_sys::Function>()
                .expect("window.matchMedia should be a function");

        let overrides_for_match = Rc::clone(&overrides);
        let listener_noop_function = listener_noop
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .clone();
        let dispatch_event_function = dispatch_event
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .clone();
        let window_for_match = window.clone();

        let match_media = Closure::<dyn FnMut(JsValue) -> JsValue>::wrap(Box::new(
            move |query_value: JsValue| {
                let query = query_value
                    .as_string()
                    .expect("matchMedia queries should be strings");

                if let Some(matches) = emulated_media_match(&query, &overrides_for_match.borrow()) {
                    fake_media_query_list(
                        &query,
                        matches,
                        &listener_noop_function,
                        &dispatch_event_function,
                    )
                } else {
                    original_match_media
                        .call1(window_for_match.as_ref(), &JsValue::from_str(&query))
                        .expect("delegated matchMedia call should succeed")
                }
            },
        ));

        install_function_property(
            window.as_ref(),
            "matchMedia",
            match_media.as_ref().unchecked_ref(),
        );

        Self {
            overrides,
            original_match_media_descriptor,
            _match_media: match_media,
            _listener_noop: listener_noop,
            _dispatch_event: dispatch_event,
        }
    }

    fn set(&mut self, feature: &str, value: &str) {
        self.overrides
            .borrow_mut()
            .insert(normalize_media_text(feature), normalize_media_text(value));
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for MediaEmulation {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            restore_property_descriptor(
                window.as_ref(),
                "matchMedia",
                &self.original_match_media_descriptor,
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct MockClipboardState {
    last_written_text: Option<String>,
    read_text: String,
    denied: bool,
}

/// Installed clipboard mock used by browser-based harness tests.
///
/// The mock replaces `navigator.clipboard` with an object that supports
/// `writeText()` and `readText()`, records the last successful write, and can be
/// configured to reject future operations. Dropping the mock restores the
/// original `navigator.clipboard` property.
pub struct MockClipboard {
    #[cfg(target_arch = "wasm32")]
    state: Rc<RefCell<MockClipboardState>>,

    #[cfg(target_arch = "wasm32")]
    original_clipboard_descriptor: JsValue,

    #[cfg(target_arch = "wasm32")]
    _write_text: Closure<dyn FnMut(String) -> js_sys::Promise>,

    #[cfg(target_arch = "wasm32")]
    _read_text: Closure<dyn FnMut() -> js_sys::Promise>,
}

impl Debug for MockClipboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockClipboard").finish_non_exhaustive()
    }
}

impl MockClipboard {
    /// Returns the last successfully written clipboard text, if any.
    #[must_use]
    pub fn last_written_text(&self) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            self.state.borrow().last_written_text.clone()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("MockClipboard::last_written_text")
        }
    }

    /// Sets the text returned by future `readText()` calls.
    pub fn set_read_text(&self, value: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            self.state.borrow_mut().read_text = String::from(value);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = value;

            native_only("MockClipboard::set_read_text")
        }
    }

    /// Forces future clipboard operations to reject with a permission error.
    pub fn deny_permission(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.state.borrow_mut().denied = true;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("MockClipboard::deny_permission")
        }
    }
}

impl Drop for MockClipboard {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("window must exist");

            restore_property_descriptor(
                window.navigator().as_ref(),
                "clipboard",
                &self.original_clipboard_descriptor,
            );
        }
    }
}

/// Primary harness handle for adapter tests.
pub struct TestHarness {
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            dead_code,
            reason = "native builds keep the field for a uniform struct layout"
        )
    )]
    container: ContainerHandle,

    service: RefCell<Box<dyn AnyService>>,
    backend: Box<dyn HarnessBackend>,

    #[cfg(target_arch = "wasm32")]
    layout: RefCell<WasmLayoutState>,

    #[cfg(all(test, not(target_arch = "wasm32")))]
    query_results: RefCell<BTreeMap<String, Vec<ElementHandle>>>,

    #[cfg(all(test, not(target_arch = "wasm32")))]
    focused: RefCell<Option<ElementHandle>>,
}

impl Debug for TestHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestHarness")
            .field("container", &self.is_mounted())
            .field("service", &"<type-erased>")
            .field("backend", &"<backend>")
            .finish()
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        let ContainerHandle::Wasm(container) = &self.container;

        #[cfg(target_arch = "wasm32")]
        container.remove();
    }
}

/// Mounts a component into an isolated DOM container using the provided backend.
pub async fn render_with_backend<C, B>(component: C, backend: B) -> TestHarness
where
    C: Component,
    B: HarnessBackend,
{
    let container = create_isolated_container();

    let backend: Box<dyn HarnessBackend> = Box::new(backend);

    let service = backend
        .mount(container_html(&container), Box::new(component))
        .await;

    backend.flush().await;

    TestHarness {
        container,
        service: RefCell::new(service),
        backend,
        #[cfg(target_arch = "wasm32")]
        layout: RefCell::new(WasmLayoutState::default()),
        #[cfg(all(test, not(target_arch = "wasm32")))]
        query_results: RefCell::new(BTreeMap::new()),
        #[cfg(all(test, not(target_arch = "wasm32")))]
        focused: RefCell::new(None),
    }
}

/// Mounts a component into an isolated DOM container using the provided backend and locale.
pub async fn render_with_locale_and_backend<C, B>(
    component: C,
    locale: Locale,
    backend: B,
) -> TestHarness
where
    C: Component,
    B: HarnessBackend,
{
    let container = create_isolated_container();

    let backend: Box<dyn HarnessBackend> = Box::new(backend);

    let service = backend
        .mount_with_locale(container_html(&container), Box::new(component), locale)
        .await;

    backend.flush().await;

    TestHarness {
        container,
        service: RefCell::new(service),
        backend,
        #[cfg(target_arch = "wasm32")]
        layout: RefCell::new(WasmLayoutState::default()),
        #[cfg(all(test, not(target_arch = "wasm32")))]
        query_results: RefCell::new(BTreeMap::new()),
        #[cfg(all(test, not(target_arch = "wasm32")))]
        focused: RefCell::new(None),
    }
}

/// Creates a mock browser [`web_sys::File`] for drag-and-drop and upload tests.
#[must_use]
pub fn mock_file(name: &str, content: &str, mime_type: &str) -> web_sys::File {
    #[cfg(target_arch = "wasm32")]
    {
        let bits = js_sys::Array::new();

        bits.push(&JsValue::from_str(content));

        let options = web_sys::FilePropertyBag::new();

        options.set_type(mime_type);

        web_sys::File::new_with_str_sequence_and_options(bits.as_ref(), name, &options)
            .expect("mock_file should construct a browser File")
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (name, content, mime_type);

        native_only("mock_file")
    }
}

/// Creates a [`web_sys::DataTransfer`] populated with the provided files.
#[must_use]
pub fn mock_data_transfer(files: &[web_sys::File]) -> web_sys::DataTransfer {
    #[cfg(target_arch = "wasm32")]
    {
        let data_transfer =
            web_sys::DataTransfer::new().expect("mock_data_transfer should create DataTransfer");

        let items = data_transfer.items();

        for file in files {
            items
                .add_with_file(file)
                .expect("adding file to DataTransfer should succeed");
        }

        data_transfer
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = files;

        native_only("mock_data_transfer")
    }
}

/// Installs a mock `navigator.clipboard` implementation for browser-based tests.
#[must_use]
pub fn mock_clipboard() -> MockClipboard {
    #[cfg(target_arch = "wasm32")]
    {
        let state = Rc::new(RefCell::new(MockClipboardState::default()));

        let navigator = web_sys::window().expect("window must exist").navigator();

        let original_clipboard_descriptor =
            own_property_descriptor(navigator.as_ref(), "clipboard");

        let mock = js_sys::Object::new();

        let write_state = Rc::clone(&state);

        let write_text =
            Closure::<dyn FnMut(String) -> js_sys::Promise>::wrap(Box::new(move |text: String| {
                let mut state = write_state.borrow_mut();

                if state.denied {
                    js_sys::Promise::reject(&JsValue::from_str("MockClipboard permission denied"))
                } else {
                    state.last_written_text = Some(text.clone());
                    state.read_text = text;

                    js_sys::Promise::resolve(&JsValue::UNDEFINED)
                }
            }));

        let read_state = Rc::clone(&state);

        let read_text = Closure::<dyn FnMut() -> js_sys::Promise>::wrap(Box::new(move || {
            let state = read_state.borrow();

            if state.denied {
                js_sys::Promise::reject(&JsValue::from_str("MockClipboard permission denied"))
            } else {
                js_sys::Promise::resolve(&JsValue::from_str(&state.read_text))
            }
        }));

        define_value_property(mock.as_ref(), "writeText", write_text.as_ref());
        define_value_property(mock.as_ref(), "readText", read_text.as_ref());
        define_value_property(navigator.as_ref(), "clipboard", mock.as_ref());

        MockClipboard {
            state,
            original_clipboard_descriptor,
            _write_text: write_text,
            _read_text: read_text,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        native_only("mock_clipboard")
    }
}

impl TestHarness {
    /// Queries a single descendant of the isolated container.
    #[must_use]
    pub fn query_selector(&self, selector: &str) -> Option<ElementHandle> {
        #[cfg(target_arch = "wasm32")]
        {
            container_html(&self.container)
                .query_selector(selector)
                .expect("query selector must not throw")
                .map(ElementHandle::from_element)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[cfg(test)]
            {
                if let Some(elements) = self.query_results.borrow().get(selector) {
                    return elements.first().cloned();
                }
            }

            let _ = selector;

            native_only("query_selector")
        }
    }

    /// Queries all matching descendants of the isolated container.
    #[must_use]
    pub fn query_selector_all(&self, selector: &str) -> Vec<ElementHandle> {
        #[cfg(target_arch = "wasm32")]
        {
            let nodes = container_html(&self.container)
                .query_selector_all(selector)
                .expect("query selector all must not throw");

            let mut handles = Vec::new();

            for index in 0..nodes.length() {
                if let Some(node) = nodes.get(index)
                    && let Ok(element) = node.dyn_into::<web_sys::Element>()
                {
                    handles.push(ElementHandle::from_element(element));
                }
            }

            handles
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[cfg(test)]
            {
                if let Some(elements) = self.query_results.borrow().get(selector) {
                    return elements.clone();
                }
            }

            let _ = selector;

            native_only("query_selector_all")
        }
    }

    /// Queries a single matching element and panics if none is found.
    #[must_use]
    pub fn query(&self, selector: &str) -> ElementHandle {
        self.query_selector(selector)
            .unwrap_or_else(|| panic!("no element matched selector '{selector}'"))
    }

    /// Returns the focused descendant of this container, if any.
    #[must_use]
    pub fn focused_element(&self) -> Option<ElementHandle> {
        #[cfg(target_arch = "wasm32")]
        {
            let document = web_sys::window()
                .and_then(|window| window.document())
                .expect("document must exist");

            let active = document.active_element()?;

            let active_node: &web_sys::Node = active.as_ref();

            if container_html(&self.container).contains(Some(active_node)) {
                Some(ElementHandle::from_element(active))
            } else {
                None
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[cfg(test)]
            {
                if let Some(element) = self.focused.borrow().clone() {
                    return Some(element);
                }
            }

            native_only("focused_element")
        }
    }

    /// Queries a single `[data-ars-part='...']` element.
    #[must_use]
    pub fn query_part(&self, part_name: &str) -> Option<ElementHandle> {
        self.query_selector(&format!("[data-ars-part='{part_name}']"))
    }

    /// Reads an attribute from the `trigger` part.
    #[must_use]
    pub fn trigger_attr(&self, attr: &str) -> Option<String> {
        self.query_part("trigger")
            .and_then(|handle| handle.attr(attr))
    }

    /// Reads an attribute from the `input` part.
    #[must_use]
    pub fn input_attr(&self, attr: &str) -> Option<String> {
        self.query_part("input")
            .and_then(|handle| handle.attr(attr))
    }

    /// Reads an attribute from the `control` part.
    #[must_use]
    pub fn control_attr(&self, attr: &str) -> Option<String> {
        self.query_part("control")
            .and_then(|handle| handle.attr(attr))
    }

    /// Reads a `data-ars-*` attribute from the root element.
    #[must_use]
    pub fn data_attr(&self, name: &str) -> Option<String> {
        self.query_selector("[data-ars-scope]")
            .and_then(|handle| handle.attr(&format!("data-ars-{name}")))
    }

    /// Reads an attribute from the first element matching the selector.
    #[must_use]
    pub fn attr(&self, selector: &str, attr: &str) -> Option<String> {
        self.query_selector(selector)
            .and_then(|handle| handle.attr(attr))
    }

    /// Reads an attribute from the `button` part.
    #[must_use]
    pub fn button_attr(&self, attr: &str) -> Option<String> {
        self.query_part("button")
            .and_then(|handle| handle.attr(attr))
    }

    /// Clicks the component root element.
    pub async fn click(&self) {
        self.click_selector("[data-ars-scope]").await;
    }

    /// Clicks the first element matching the selector.
    pub async fn click_selector(&self, selector: &str) {
        let element = self.query(selector);

        element.click().await;

        self.flush().await;
    }

    /// Types text into the currently focused element and dispatches an input event.
    pub async fn type_text(&self, text: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("type_text requires a focused element");

            if let Some(input) = focused
                .clone()
                .element
                .dyn_ref::<web_sys::HtmlInputElement>()
            {
                let mut value = input.value();

                value.push_str(text);

                input.set_value(&value);
            } else if let Some(textarea) = focused
                .clone()
                .element
                .dyn_ref::<web_sys::HtmlTextAreaElement>()
            {
                let mut value = textarea.value();

                value.push_str(text);

                textarea.set_value(&value);
            } else {
                let current = focused.attr("value").unwrap_or_default();

                let next = format!("{current}{text}");

                drop(js_sys::Reflect::set(
                    focused.element.as_ref(),
                    &JsValue::from_str("value"),
                    &JsValue::from_str(&next),
                ));
            }

            let event = bubbling_input_event();

            let _ = focused
                .element
                .dispatch_event(&event)
                .expect("input event dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = text;

            native_only("type_text")
        }
    }

    /// Dispatches hover events on the first element matching the selector.
    pub async fn hover(&self, selector: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let element = self.query(selector).element;

            let pointer =
                web_sys::PointerEvent::new("pointerenter").expect("pointerenter must construct");

            let _ = element
                .dispatch_event(&pointer)
                .expect("pointerenter dispatch must succeed");

            let mouse = bubbling_mouse_over_event();

            let _ = element
                .dispatch_event(&mouse)
                .expect("mouseover dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = selector;

            native_only("hover")
        }
    }

    /// Dispatches hover events on the trigger element.
    pub async fn hover_trigger(&self) {
        self.hover("[data-ars-part='trigger']").await;
    }

    /// Blurs the currently focused element.
    pub async fn blur(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("blur requires a focused element");

            if let Some(html) = focused.element.dyn_ref::<web_sys::HtmlElement>() {
                html.blur().expect("blur should succeed");
            } else {
                let event = web_sys::FocusEvent::new("blur").expect("blur event must construct");

                let _ = focused
                    .element
                    .dispatch_event(&event)
                    .expect("blur dispatch must succeed");
            }

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("blur")
        }
    }

    /// Focuses the first element matching the selector.
    pub async fn focus(&self, selector: &str) {
        let handle = self.query(selector);

        handle.focus().await;

        self.flush().await;
    }

    /// Sets a JS `value` property on the currently focused element and dispatches an input event.
    pub async fn set_value<V: Into<JsValue>>(&self, value: V) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("set_value requires a focused element");

            let value = value.into();

            let _ = js_sys::Reflect::set(
                focused.element.as_ref(),
                &JsValue::from_str("value"),
                &value,
            )
            .expect("setting element value should succeed");

            let event = bubbling_input_event();

            let _ = focused
                .element
                .dispatch_event(&event)
                .expect("input event dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _unused_value = value;

            native_only("set_value")
        }
    }

    /// Dispatches a keydown event on the focused element.
    pub async fn press_key(&self, key: KeyboardKey) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("press_key requires a focused element");

            let event = keyboard_event("keydown", key);

            let _ = focused
                .element
                .dispatch_event(&event)
                .expect("keydown dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;

            native_only("press_key")
        }
    }

    /// Alias for [`press_key`](Self::press_key).
    pub async fn press(&self, key: KeyboardKey) {
        self.press_key(key).await;
    }

    /// Dispatches keydown, keypress, and keyup on the focused element.
    pub async fn key_sequence(&self, key: KeyboardKey) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("key_sequence requires a focused element");

            for event_name in ["keydown", "keypress", "keyup"] {
                let event = keyboard_event(event_name, key);

                let _ = focused
                    .element
                    .dispatch_event(&event)
                    .expect("keyboard event dispatch must succeed");
            }

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;

            native_only("key_sequence")
        }
    }

    /// Dispatches `touchstart` on the component root.
    pub async fn touch_start(&self, point: Point) {
        #[cfg(target_arch = "wasm32")]
        {
            {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_touch_point = Some(point);
                layout.last_touch_target = Some(TouchTarget::Root);
            }
            self.dispatch_touch_event("[data-ars-scope]", "touchstart", point)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = point;

            native_only("touch_start")
        }
    }

    /// Dispatches `touchmove` on the component root.
    pub async fn touch_move(&self, point: Point) {
        #[cfg(target_arch = "wasm32")]
        {
            {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_touch_point = Some(point);
                layout.last_touch_target = Some(TouchTarget::Root);
            }
            self.dispatch_touch_event("[data-ars-scope]", "touchmove", point)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = point;

            native_only("touch_move")
        }
    }

    /// Dispatches `touchend` on the component root.
    pub async fn touch_end(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let (selector, point) = {
                let layout = self.layout.borrow();
                let selector = match layout.last_touch_target.unwrap_or(TouchTarget::Root) {
                    TouchTarget::Root => "[data-ars-scope]",
                    TouchTarget::Trigger => "[data-ars-part='trigger']",
                };

                let point = layout.last_root_touch_point.unwrap_or_else(|| {
                    let rect = self.query(selector).bounding_rect();

                    point(rect.x + (rect.width / 2.0), rect.y + (rect.height / 2.0))
                });

                (selector, point)
            };

            self.dispatch_touch_event_with_state(selector, "touchend", point, false)
                .await;

            let mut layout = self.layout.borrow_mut();
            layout.last_root_touch_point = None;
            layout.last_touch_target = None;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.dispatch_simple_event("[data-ars-scope]", "touchend")
                .await;
        }
    }

    /// Dispatches `touchstart` on the trigger element.
    pub async fn touch_start_on_trigger(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let trigger = self.query("[data-ars-part='trigger']");

            let rect = trigger.bounding_rect();

            let center = Point {
                x: rect.x + (rect.width / 2.0),
                y: rect.y + (rect.height / 2.0),
            };

            {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_touch_point = Some(center);
                layout.last_touch_target = Some(TouchTarget::Trigger);
            }

            self.dispatch_touch_event("[data-ars-part='trigger']", "touchstart", center)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("touch_start_on_trigger")
        }
    }

    /// Dispatches `touchmove` on the trigger element.
    pub async fn touch_move_first(&self, point: Point) {
        #[cfg(target_arch = "wasm32")]
        {
            {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_touch_point = Some(point);
                layout.last_touch_target = Some(TouchTarget::Trigger);
            }
            self.dispatch_touch_event("[data-ars-part='trigger']", "touchmove", point)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = point;

            native_only("touch_move_first")
        }
    }

    /// Dispatches `pointerdown` on the component root.
    pub async fn pointer_down_at(&self, x: f64, y: f64) {
        #[cfg(target_arch = "wasm32")]
        {
            {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_pointer_point = Some(point(x, y));
                layout.primary_pointer_button_down = true;
            }

            self.dispatch_pointer_event("[data-ars-scope]", "pointerdown", x, y, true)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (x, y);

            native_only("pointer_down_at")
        }
    }

    /// Dispatches `pointermove` on the component root.
    pub async fn pointer_move_to(&self, x: f64, y: f64) {
        #[cfg(target_arch = "wasm32")]
        {
            let buttons_down = {
                let mut layout = self.layout.borrow_mut();
                layout.last_root_pointer_point = Some(point(x, y));
                layout.primary_pointer_button_down
            };

            self.dispatch_pointer_event("[data-ars-scope]", "pointermove", x, y, buttons_down)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (x, y);

            native_only("pointer_move_to")
        }
    }

    /// Dispatches `pointerup` on the component root.
    pub async fn pointer_up(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let point = self
                .layout
                .borrow()
                .last_root_pointer_point
                .unwrap_or_else(|| self.root_center("[data-ars-scope]"));

            self.dispatch_pointer_event("[data-ars-scope]", "pointerup", point.x, point.y, false)
                .await;

            let mut layout = self.layout.borrow_mut();
            layout.last_root_pointer_point = None;
            layout.primary_pointer_button_down = false;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("pointer_up")
        }
    }

    /// Dispatches composition start and update events.
    pub async fn ime_compose(&self, text: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("ime_compose requires a focused element");

            self.dispatch_composition_event(&focused.element, "compositionstart", "")
                .await;
            self.dispatch_composition_event(&focused.element, "compositionupdate", text)
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = text;

            native_only("ime_compose")
        }
    }

    /// Dispatches composition end and input events.
    pub async fn ime_commit(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let focused = self
                .focused_element()
                .expect("ime_commit requires a focused element");

            let end = web_sys::CompositionEvent::new("compositionend")
                .expect("compositionend must exist");

            let _ = focused
                .element
                .dispatch_event(&end)
                .expect("compositionend dispatch must succeed");

            let input = bubbling_input_event();

            let _ = focused
                .element
                .dispatch_event(&input)
                .expect("input dispatch must succeed");

            self.flush().await;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("ime_commit")
        }
    }

    /// Returns whether any element in the harness container currently has focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused_element().is_some()
    }

    /// Returns focusable descendants in DOM order.
    #[must_use]
    pub fn get_tab_order(&self) -> Vec<ElementHandle> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut elements =
                self.query_selector_all("button, [href], input, select, textarea, [tabindex]");
            elements.retain(|element| is_tabbable_element(&element.element));
            elements
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.query_selector_all("button, [href], input, select, textarea, [tabindex]")
        }
    }

    /// Returns `[data-ars-part]` descendants sorted by visual position.
    #[must_use]
    pub fn get_visual_order(&self) -> Vec<ElementHandle> {
        let mut elements = self.query_selector_all("[data-ars-part]");

        elements.sort_by(|left, right| {
            let left_rect = left.bounding_rect();
            let right_rect = right.bounding_rect();

            left_rect
                .y
                .partial_cmp(&right_rect.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left_rect
                        .x
                        .partial_cmp(&right_rect.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        elements
    }

    /// Advances backend-owned fake time.
    pub async fn advance_time(&self, duration: Duration) {
        self.backend.advance_time(duration).await;

        self.backend.flush().await;
    }

    /// Dispatches `animationend` on the root element.
    pub async fn fire_animation_end(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.dispatch_animation_event("[data-ars-scope]", "animationend")
                .await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("fire_animation_end")
        }
    }

    /// Returns whether the root element is still mounted.
    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.query_selector("[data-ars-scope]").is_some()
    }

    /// Sends a typed event directly to the erased service.
    pub async fn send<E: Any>(&self, event: E) {
        self.service.borrow_mut().send_boxed(Box::new(event));

        self.flush().await;
    }

    /// Returns the current machine state as a debug string.
    #[must_use]
    pub fn state(&self) -> String {
        self.service.borrow().state_debug()
    }

    /// Returns whether the component appears open based on root state attributes.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.data_attr("state")
            .is_some_and(|state| matches!(state.as_str(), "open" | "expanded"))
            || self
                .query_selector("[data-ars-scope]")
                .and_then(|handle| handle.attr("aria-expanded"))
                .is_some_and(|value| value == "true")
    }

    /// Returns `data-ars-*` attributes from the root element.
    #[must_use]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::missing_const_for_fn,
            reason = "the wasm build reads dynamic DOM attributes"
        )
    )]
    pub fn snapshot_attrs(&self) -> BTreeMap<String, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let root = self.query("[data-ars-scope]").element;

            let mut snapshot = BTreeMap::new();

            let attributes = root.attributes();

            for index in 0..attributes.length() {
                if let Some(attribute) = attributes.item(index) {
                    let name = attribute.name();

                    if name.starts_with("data-ars-") {
                        snapshot.insert(name, attribute.value());
                    }
                }
            }

            snapshot
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            BTreeMap::new()
        }
    }

    /// Returns all `data-ars-part` values in DOM order.
    #[must_use]
    pub fn snapshot_parts(&self) -> Vec<String> {
        self.query_selector_all("[data-ars-part]")
            .into_iter()
            .filter_map(|handle| handle.attr("data-ars-part"))
            .collect()
    }

    /// Returns whether the document body is scroll-locked.
    #[must_use]
    pub fn body_has_scroll_lock(&self) -> bool {
        self.body_style("overflow") == "hidden"
    }

    /// Reads a computed style from the document body.
    #[must_use]
    pub fn body_style(&self, property: &str) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            let document = web_sys::window()
                .and_then(|window| window.document())
                .expect("document must exist");

            let body = document.body().expect("body must exist");

            let computed = web_sys::window()
                .expect("window must exist")
                .get_computed_style(&body)
                .expect("computed style lookup must succeed")
                .expect("body must have computed style");

            computed
                .get_property_value(property)
                .unwrap_or_else(|_| String::new())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = property;

            native_only("body_style")
        }
    }

    /// Sets an inline style property on the document body.
    pub fn set_body_style(&self, property: &str, value: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let document = web_sys::window()
                .and_then(|window| window.document())
                .expect("document must exist");

            let body = document.body().expect("body must exist");

            body.style()
                .set_property(property, value)
                .expect("setting body style should succeed");
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (property, value);

            native_only("set_body_style")
        }
    }

    /// Runs a closure and returns the values it produced.
    #[must_use]
    pub fn record_values<F, T>(&self, f: F) -> Vec<T>
    where
        F: FnOnce() -> Vec<T>,
    {
        f()
    }

    /// Emulates a CSS media feature for the duration of the test.
    pub async fn emulate_media(&self, feature: &str, value: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let mut layout = self.layout.borrow_mut();

            let media_emulation = layout
                .media_emulation
                .get_or_insert_with(MediaEmulation::install);
            media_emulation.set(feature, value);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (feature, value);
        }

        self.flush().await;
    }

    /// Flushes pending reactive work.
    pub async fn tick(&self) {
        self.flush().await;
    }

    /// Flushes pending reactive work.
    pub async fn flush(&self) {
        self.backend.flush().await;
    }

    /// Returns an [`ItemHandle`] by DOM index.
    #[must_use]
    pub fn item(&self, index: usize) -> ItemHandle<'_> {
        let element = self
            .query_selector_all("[data-ars-part='item']")
            .into_iter()
            .nth(index)
            .unwrap_or_else(|| panic!("no item at index {index}"));

        ItemHandle::new(self, element)
    }

    /// Scrolls the browser window to a specific position.
    ///
    /// This targets the global window rather than the isolated harness container.
    pub fn scroll_to(&self, x: i32, y: i32) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                window.scroll_to_with_x_and_y(f64::from(x), f64::from(y));
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (x, y);

            native_only("scroll_to")
        }
    }

    /// Returns the current browser window scroll position.
    #[must_use]
    pub fn scroll_y(&self) -> i32 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|window| window.scroll_y().ok())
                .map_or(0, |value| value.round() as i32)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            native_only("scroll_y")
        }
    }

    /// Sets the isolated container viewport dimensions for positioning tests.
    pub fn set_viewport(&self, width: f64, height: f64) {
        #[cfg(target_arch = "wasm32")]
        {
            let container = container_html(&self.container);

            let mut layout = self.layout.borrow_mut();

            if let Some(viewport) = layout.viewport.as_mut() {
                viewport.set_size(container, width, height);
            } else {
                layout.viewport = Some(ViewportOverride::install(container, width, height));
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (width, height);

            native_only("set_viewport")
        }
    }

    /// Overrides the trigger element's layout rectangle for anchor-positioned overlays.
    pub fn set_anchor_position(&self, rect: Rect) {
        #[cfg(target_arch = "wasm32")]
        {
            let target = self
                .query_part("trigger")
                .or_else(|| self.query_selector("[data-ars-scope]"))
                .expect("set_anchor_position requires a trigger part or root element")
                .element;

            let mut layout = self.layout.borrow_mut();

            let scroll_x = layout.scroll_x;

            let scroll_y = layout.scroll_y;

            if let Some(anchor) = layout.anchor.as_mut()
                && anchor.element == target
            {
                anchor.set_base_rect(rect, scroll_x, scroll_y);
            } else {
                drop(layout.anchor.take());

                let mut anchor = install_rect_override(&target, rect);

                anchor.set_base_rect(rect, scroll_x, scroll_y);

                layout.anchor = Some(anchor);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = rect;

            native_only("set_anchor_position")
        }
    }

    /// Scrolls the isolated harness container by the provided delta.
    pub async fn scroll_container_by(&self, dx: i32, dy: i32) {
        #[cfg(target_arch = "wasm32")]
        {
            let container = container_html(&self.container);

            let (scroll_x, scroll_y) = {
                let mut layout = self.layout.borrow_mut();

                layout.scroll_x += dx;
                layout.scroll_y += dy;

                if let Some(anchor) = layout.anchor.as_ref() {
                    anchor.update_for_scroll(layout.scroll_x, layout.scroll_y);
                }

                (layout.scroll_x, layout.scroll_y)
            };

            define_value_property(
                container.as_ref(),
                "scrollLeft",
                &JsValue::from_f64(f64::from(scroll_x)),
            );

            define_value_property(
                container.as_ref(),
                "scrollTop",
                &JsValue::from_f64(f64::from(scroll_y)),
            );

            let container_scroll = web_sys::Event::new("scroll")
                .expect("scroll event for harness container must construct");

            let _ = container
                .dispatch_event(&container_scroll)
                .expect("container scroll dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (dx, dy);

            native_only("scroll_container_by")
        }
    }

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "generic event fallback is only used on non-wasm paths"
        )
    )]
    async fn dispatch_simple_event(&self, selector: &str, event_name: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let element = self.query(selector).element;

            let event = web_sys::Event::new(event_name).expect("event must construct");

            let _ = element
                .dispatch_event(&event)
                .expect("event dispatch must succeed");

            self.flush().await;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (selector, event_name);

            native_only("dispatch_simple_event")
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn root_center(&self, selector: &str) -> Point {
        let rect = self.query(selector).bounding_rect();

        point(rect.x + (rect.width / 2.0), rect.y + (rect.height / 2.0))
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_pointer_event(
        &self,
        selector: &str,
        event_name: &str,
        x: f64,
        y: f64,
        primary_button_down: bool,
    ) {
        let element = self.query(selector).element;

        let init = web_sys::PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_id(1);
        init.set_pointer_type("mouse");
        init.set_is_primary(true);
        init.set_client_x(dom_event_coordinate(x));
        init.set_client_y(dom_event_coordinate(y));

        match event_name {
            "pointerdown" => {
                init.set_button(0);
                init.set_buttons(1);
            }

            "pointermove" => {
                init.set_buttons(if primary_button_down { 1 } else { 0 });
            }

            "pointerup" => {
                init.set_button(0);
                init.set_buttons(0);
            }

            _ => {}
        }

        let event = web_sys::PointerEvent::new_with_event_init_dict(event_name, &init)
            .expect("pointer event must construct");

        let _ = element
            .dispatch_event(&event)
            .expect("pointer event dispatch must succeed");

        self.flush().await;
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_animation_event(&self, selector: &str, event_name: &str) {
        let element = self.query(selector).element;

        let init = web_sys::AnimationEventInit::new();

        init.set_bubbles(true);
        init.set_animation_name("");
        init.set_elapsed_time(0.0);
        init.set_pseudo_element("");

        let event = web_sys::AnimationEvent::new_with_event_init_dict(event_name, &init)
            .expect("animation event must construct");

        let _ = element
            .dispatch_event(&event)
            .expect("animation event dispatch must succeed");

        self.flush().await;
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_touch_event(&self, selector: &str, event_name: &str, point: Point) {
        self.dispatch_touch_event_with_state(selector, event_name, point, true)
            .await;
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_touch_event_with_state(
        &self,
        selector: &str,
        event_name: &str,
        point: Point,
        touch_active: bool,
    ) {
        let element = self.query(selector).element;

        let touch = touch_at_point(&element, point, 0);

        let changed_touches = js_sys::Array::new();
        let touches = js_sys::Array::new();

        changed_touches.push(&touch);
        if touch_active {
            touches.push(&touch);
        }

        let init = web_sys::TouchEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_changed_touches(changed_touches.as_ref());
        init.set_target_touches(touches.as_ref());
        init.set_touches(touches.as_ref());

        let event = web_sys::TouchEvent::new_with_event_init_dict(event_name, &init)
            .expect("touch event must construct");

        let _ = element
            .dispatch_event(&event)
            .expect("touch event dispatch must succeed");

        self.flush().await;
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_composition_event(
        &self,
        element: &web_sys::Element,
        event_name: &str,
        data: &str,
    ) {
        let init = web_sys::CompositionEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_data(data);

        let event = web_sys::CompositionEvent::new_with_event_init_dict(event_name, &init)
            .expect("composition event must construct");

        let _ = element
            .dispatch_event(&event)
            .expect("composition event dispatch must succeed");

        self.flush().await;
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    fn new_for_test(service: Box<dyn AnyService>, backend: Box<dyn HarnessBackend>) -> Self {
        Self {
            container: {
                ContainerHandle::Native {
                    element: JsValue::NULL.unchecked_into(),
                }
            },
            service: RefCell::new(service),
            backend,
            query_results: RefCell::new(BTreeMap::new()),
            focused: RefCell::new(None),
        }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    fn set_query_results(&self, selector: &str, elements: Vec<ElementHandle>) {
        self.query_results
            .borrow_mut()
            .insert(String::from(selector), elements);
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    fn set_focused_element(&self, element: Option<ElementHandle>) {
        *self.focused.borrow_mut() = element;
    }
}

#[cfg(target_arch = "wasm32")]
fn dom_event_coordinate(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }

    value
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[cfg(target_arch = "wasm32")]
fn bubbling_input_event() -> web_sys::InputEvent {
    let init = web_sys::InputEventInit::new();

    init.set_bubbles(true);
    init.set_composed(true);

    web_sys::InputEvent::new_with_event_init_dict("input", &init)
        .expect("input event must construct cleanly")
}

#[cfg(target_arch = "wasm32")]
fn bubbling_mouse_over_event() -> web_sys::MouseEvent {
    let init = web_sys::MouseEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    web_sys::MouseEvent::new_with_mouse_event_init_dict("mouseover", &init)
        .expect("mouseover must construct cleanly")
}

#[cfg(target_arch = "wasm32")]
fn touch_at_point(element: &web_sys::Element, point: Point, identifier: i32) -> web_sys::Touch {
    let event_target: &web_sys::EventTarget = element.unchecked_ref();

    let init = web_sys::TouchInit::new(identifier, event_target);

    let x = dom_event_coordinate(point.x);
    let y = dom_event_coordinate(point.y);

    init.set_client_x(x);
    init.set_client_y(y);
    init.set_page_x(x);
    init.set_page_y(y);
    init.set_screen_x(x);
    init.set_screen_y(y);

    web_sys::Touch::new(&init).expect("touch must construct")
}

#[cfg(target_arch = "wasm32")]
fn keyboard_event(event_name: &str, key: KeyboardKey) -> web_sys::KeyboardEvent {
    let init = web_sys::KeyboardEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(&key.as_key_value());

    web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(event_name, &init)
        .expect("keyboard event must construct")
}

fn create_isolated_container() -> ContainerHandle {
    #[cfg(target_arch = "wasm32")]
    {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("document must exist");

        let body = document.body().expect("body must exist");

        let element = document
            .create_element("div")
            .expect("isolated harness container must be creatable");

        element
            .set_attribute("data-ars-test-container", "")
            .expect("isolated harness container attribute must set cleanly");

        let container: web_sys::HtmlElement = element
            .dyn_into()
            .expect("isolated harness container must be an HtmlElement");

        body.append_child(&container)
            .expect("isolated harness container must append to body");

        ContainerHandle::Wasm(container)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(test)]
        {
            ContainerHandle::Native {
                element: JsValue::NULL.unchecked_into(),
            }
        }

        #[cfg(not(test))]
        {
            native_only("render_with_backend")
        }
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    expect(
        clippy::missing_const_for_fn,
        reason = "the native build retains a panic branch for non-wasm targets"
    )
)]
#[cfg_attr(
    all(test, not(target_arch = "wasm32")),
    expect(
        clippy::missing_const_for_fn,
        reason = "native test builds return a fake HtmlElement handle"
    )
)]
fn container_html(container: &ContainerHandle) -> &web_sys::HtmlElement {
    #[cfg(target_arch = "wasm32")]
    {
        match container {
            ContainerHandle::Wasm(container) => container,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(test)]
        {
            match container {
                ContainerHandle::Native { element } => element,
            }
        }

        #[cfg(not(test))]
        {
            let _ = container;

            native_only("container_html")
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cold]
fn native_only(method: &str) -> ! {
    panic!("{method} is only available on the wasm32 test runtime");
}

#[cfg(target_arch = "wasm32")]
fn install_rect_override(element: &web_sys::Element, rect: Rect) -> ElementRectOverride {
    let rect_state = Rc::new(RefCell::new(rect));
    let original_descriptor = own_property_descriptor(element.as_ref(), "getBoundingClientRect");

    let getter_state = Rc::clone(&rect_state);

    let getter = Closure::<dyn FnMut() -> JsValue>::wrap(Box::new(move || {
        rect_to_js_value(*getter_state.borrow())
    }));

    install_function_property(
        element.as_ref(),
        "getBoundingClientRect",
        getter.as_ref().unchecked_ref(),
    );

    ElementRectOverride {
        element: element.clone(),
        current_rect: rect_state,
        _getter: getter,
        base_rect: rect,
        original_descriptor,
    }
}

#[cfg(target_arch = "wasm32")]
fn normalize_media_text(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(target_arch = "wasm32")]
fn emulated_media_match(query: &str, overrides: &BTreeMap<String, String>) -> Option<bool> {
    let normalized_query = normalize_media_text(query);
    let body = normalized_query
        .strip_prefix('(')?
        .strip_suffix(')')
        .filter(|body| !body.contains('(') && !body.contains(')') && !body.contains(','))?;
    let (feature, actual_value) = body.split_once(':')?;
    let expected_value = overrides.get(feature.trim())?;

    Some(actual_value.trim() == expected_value)
}

#[cfg(target_arch = "wasm32")]
fn fake_media_query_list(
    query: &str,
    matches: bool,
    listener_noop: &js_sys::Function,
    dispatch_event: &js_sys::Function,
) -> JsValue {
    let media_query_list = js_sys::Object::new();

    define_value_property(
        media_query_list.as_ref(),
        "media",
        &JsValue::from_str(query),
    );
    define_value_property(
        media_query_list.as_ref(),
        "matches",
        &JsValue::from_bool(matches),
    );
    define_value_property(media_query_list.as_ref(), "onchange", &JsValue::NULL);

    for method in [
        "addListener",
        "removeListener",
        "addEventListener",
        "removeEventListener",
    ] {
        install_function_property(media_query_list.as_ref(), method, listener_noop);
    }

    install_function_property(media_query_list.as_ref(), "dispatchEvent", dispatch_event);

    media_query_list.into()
}

#[cfg(target_arch = "wasm32")]
fn rect_to_js_value(rect: Rect) -> JsValue {
    let object = js_sys::Object::new();

    set_object_number(&object, "x", rect.x);
    set_object_number(&object, "y", rect.y);
    set_object_number(&object, "width", rect.width);
    set_object_number(&object, "height", rect.height);
    set_object_number(&object, "left", rect.left());
    set_object_number(&object, "top", rect.top());
    set_object_number(&object, "right", rect.right());
    set_object_number(&object, "bottom", rect.bottom());

    object.into()
}

#[cfg(target_arch = "wasm32")]
fn set_object_number(object: &js_sys::Object, name: &str, value: f64) {
    js_sys::Reflect::set(
        object.as_ref(),
        &JsValue::from_str(name),
        &JsValue::from_f64(value),
    )
    .expect("setting object number property must succeed");
}

#[cfg(target_arch = "wasm32")]
fn define_numeric_getter(
    target: &JsValue,
    property: &str,
    value: Rc<Cell<f64>>,
) -> Closure<dyn FnMut() -> JsValue> {
    let getter =
        Closure::<dyn FnMut() -> JsValue>::wrap(Box::new(move || JsValue::from_f64(value.get())));

    install_getter_property(target, property, getter.as_ref().unchecked_ref());

    getter
}

#[cfg(target_arch = "wasm32")]
fn own_property_descriptor(target: &JsValue, property: &str) -> JsValue {
    let target: &js_sys::Object = target.unchecked_ref();

    js_sys::Object::get_own_property_descriptor(target, &JsValue::from_str(property))
}

#[cfg(target_arch = "wasm32")]
fn install_function_property(target: &JsValue, property: &str, function: &js_sys::Function) {
    let target: &js_sys::Object = target.unchecked_ref();

    let descriptor = js_sys::Object::new();

    js_sys::Reflect::set(
        descriptor.as_ref(),
        &JsValue::from_str("value"),
        function.as_ref(),
    )
    .expect("defining function property value must succeed");

    js_sys::Reflect::set(
        descriptor.as_ref(),
        &JsValue::from_str("configurable"),
        &JsValue::TRUE,
    )
    .expect("defining function property configurability must succeed");

    js_sys::Object::define_property(target, &JsValue::from_str(property), &descriptor);
}

#[cfg(target_arch = "wasm32")]
fn install_getter_property(target: &JsValue, property: &str, function: &js_sys::Function) {
    let target: &js_sys::Object = target.unchecked_ref();

    let descriptor = js_sys::Object::new();

    js_sys::Reflect::set(
        descriptor.as_ref(),
        &JsValue::from_str("get"),
        function.as_ref(),
    )
    .expect("defining getter property must succeed");

    js_sys::Reflect::set(
        descriptor.as_ref(),
        &JsValue::from_str("configurable"),
        &JsValue::TRUE,
    )
    .expect("defining getter configurability must succeed");

    js_sys::Object::define_property(target, &JsValue::from_str(property), &descriptor);
}

#[cfg(target_arch = "wasm32")]
fn define_value_property(target: &JsValue, property: &str, value: &JsValue) {
    let target: &js_sys::Object = target.unchecked_ref();

    let descriptor = js_sys::Object::new();

    js_sys::Reflect::set(descriptor.as_ref(), &JsValue::from_str("value"), value)
        .expect("defining value property must succeed");

    js_sys::Reflect::set(
        descriptor.as_ref(),
        &JsValue::from_str("configurable"),
        &JsValue::TRUE,
    )
    .expect("defining value property configurability must succeed");

    js_sys::Object::define_property(target, &JsValue::from_str(property), &descriptor);
}

#[cfg(target_arch = "wasm32")]
fn restore_property_descriptor(target: &JsValue, property: &str, descriptor: &JsValue) {
    if descriptor.is_undefined() {
        let target: &js_sys::Object = target.unchecked_ref();

        let deleted = js_sys::Reflect::delete_property(target, &JsValue::from_str(property))
            .expect("deleting restored property should not throw");

        assert!(deleted, "deleting restored property should succeed");
    } else {
        let target: &js_sys::Object = target.unchecked_ref();
        let descriptor: &js_sys::Object = descriptor.unchecked_ref();

        js_sys::Object::define_property(target, &JsValue::from_str(property), descriptor);
    }
}

#[cfg(target_arch = "wasm32")]
fn is_tabbable_element(element: &web_sys::Element) -> bool {
    if element.has_attribute("disabled") {
        return false;
    }

    js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("tabIndex"))
        .ok()
        .and_then(|value| value.as_f64())
        .is_some_and(|value| value >= 0.0)
}

#[cfg(test)]
mod tests {
    use std::{any::Any, future::Future, panic::AssertUnwindSafe, pin::Pin, str::FromStr};
    #[cfg(not(target_arch = "wasm32"))]
    use std::{
        cell::Cell,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    use ars_core::{AttrValue, Env, HasId, HtmlAttr};
    #[cfg(not(target_arch = "wasm32"))]
    use wasm_bindgen::JsCast;
    #[cfg(target_arch = "wasm32")]
    use {
        std::{cell::Cell, rc::Rc},
        wasm_bindgen_test::*,
    };

    use super::*;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum MockPart {
        Root,
        Trigger,
        Item,
    }

    impl ComponentPart for MockPart {
        const ROOT: Self = Self::Root;

        fn scope() -> &'static str {
            "mock"
        }

        fn name(&self) -> &'static str {
            match self {
                Self::Root => "root",
                Self::Trigger => "trigger",
                Self::Item => "item",
            }
        }

        fn all() -> Vec<Self> {
            vec![Self::Root, Self::Trigger, Self::Item]
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum MockState {
        Idle,
        Open,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum MockEvent {
        Open,
        Close,
    }

    impl FromStr for MockEvent {
        type Err = ();

        fn from_str(input: &str) -> Result<Self, Self::Err> {
            match input {
                "Open" => Ok(Self::Open),
                "Close" => Ok(Self::Close),
                _ => Err(()),
            }
        }
    }

    #[derive(Clone, Debug)]
    struct MockContext;

    #[derive(Clone, Debug, PartialEq)]
    struct MockProps {
        id: String,
    }

    impl HasId for MockProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(mut self, id: String) -> Self {
            self.id = id;
            self
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct MockApi<'a> {
        state: &'a MockState,
    }

    impl ConnectApi for MockApi<'_> {
        type Part = MockPart;

        fn part_attrs(&self, part: Self::Part) -> AttrMap {
            let mut attrs = AttrMap::new();

            attrs.set(HtmlAttr::Data("ars-scope"), MockPart::scope());
            attrs.set(HtmlAttr::Data("ars-part"), part.name());
            attrs.set(HtmlAttr::Role, "button");

            if matches!(self.state, MockState::Open) {
                attrs.set(HtmlAttr::Data("ars-state"), "open");
                attrs.set(HtmlAttr::Aria(ars_core::AriaAttr::Expanded), "true");
            } else {
                attrs.set(HtmlAttr::Data("ars-state"), "idle");
                attrs.set(HtmlAttr::Aria(ars_core::AriaAttr::Expanded), "false");
            }

            attrs
        }
    }

    struct MockMachine;

    impl Machine for MockMachine {
        type State = MockState;
        type Event = MockEvent;
        type Context = MockContext;
        type Props = MockProps;
        type Messages = ();
        type Api<'a> = MockApi<'a>;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (MockState::Idle, MockContext)
        }

        fn transition(
            state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match (state, event) {
                (MockState::Idle, MockEvent::Open) => {
                    Some(ars_core::TransitionPlan::to(MockState::Open))
                }

                (MockState::Open, MockEvent::Close) => {
                    Some(ars_core::TransitionPlan::to(MockState::Idle))
                }

                _ => None,
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            MockApi { state }
        }
    }

    struct MockComponent;

    impl Component for MockComponent {
        type Machine = MockMachine;
    }

    #[cfg(not(target_arch = "wasm32"))]
    struct NoopBackend;

    #[cfg(not(target_arch = "wasm32"))]
    impl HarnessBackend for NoopBackend {
        fn mount(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            Box::pin(async { panic!("test-only backend mount should not be called on native") })
        }

        fn mount_with_locale(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
            _locale: Locale,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            Box::pin(async {
                panic!("test-only backend mount_with_locale should not be called on native")
            })
        }

        fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async {})
        }

        fn advance_time(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async {})
        }
    }

    fn mock_service() -> Service<MockMachine> {
        Service::new(
            MockProps {
                id: String::from("mock"),
            },
            &Env::default(),
            &(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn panic_message(panic: &(dyn Any + Send)) -> String {
        if let Some(message) = panic.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = panic.downcast_ref::<&'static str>() {
            String::from(*message)
        } else {
            String::from("<non-string panic>")
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn panic_message(panic: &(dyn Any + Send)) -> String {
        if let Some(message) = panic.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = panic.downcast_ref::<&'static str>() {
            String::from(*message)
        } else {
            String::from("<non-string panic>")
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn assert_panics_with_message<F, R>(f: F, expected: &str)
    where
        F: FnOnce() -> R,
    {
        let Err(panic) = std::panic::catch_unwind(AssertUnwindSafe(f)) else {
            panic!("operation should panic on the native test runtime");
        };

        let message = panic_message(panic.as_ref());

        assert!(
            message.contains(expected),
            "expected panic message to contain {expected:?}, got {message:?}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    fn assert_panics_with_message<F, R>(f: F, expected: &str)
    where
        F: FnOnce() -> R,
    {
        let Err(panic) = std::panic::catch_unwind(AssertUnwindSafe(f)) else {
            panic!("operation should panic on the wasm test runtime");
        };

        let message = panic_message(panic.as_ref());

        assert!(
            message.contains(expected),
            "expected panic message to contain {expected:?}, got {message:?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    struct NoopWake;

    #[cfg(not(target_arch = "wasm32"))]
    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));

        let mut future = std::pin::pin!(future);

        let mut context = Context::from_waker(&waker);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test future unexpectedly returned Poll::Pending"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fake_html_element() -> web_sys::HtmlElement {
        JsValue::NULL.unchecked_into()
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[derive(Clone, Default)]
    struct RecordingBackend {
        flushes: Rc<Cell<u32>>,
        advanced: Rc<RefCell<Vec<Duration>>>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl RecordingBackend {
        fn flush_count(&self) -> u32 {
            self.flushes.get()
        }

        fn advanced_durations(&self) -> Vec<Duration> {
            self.advanced.borrow().clone()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[derive(Clone, Default)]
    struct MountedNativeBackend {
        flushes: Rc<Cell<u32>>,
        locales: Rc<RefCell<Vec<Locale>>>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl MountedNativeBackend {
        fn flush_count(&self) -> u32 {
            self.flushes.get()
        }

        fn mounted_locales(&self) -> Vec<Locale> {
            self.locales.borrow().clone()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl HarnessBackend for MountedNativeBackend {
        fn mount(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            Box::pin(async { Box::new(mock_service()) as Box<dyn AnyService> })
        }

        fn mount_with_locale(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
            locale: Locale,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            let locales = Rc::clone(&self.locales);
            Box::pin(async move {
                locales.borrow_mut().push(locale);
                Box::new(mock_service()) as Box<dyn AnyService>
            })
        }

        fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
            let flushes = Rc::clone(&self.flushes);
            Box::pin(async move {
                flushes.set(flushes.get() + 1);
            })
        }

        fn advance_time(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async {})
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl HarnessBackend for RecordingBackend {
        fn mount(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            Box::pin(async { panic!("test-only backend mount should not be called on native") })
        }

        fn mount_with_locale(
            &self,
            _container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
            _locale: Locale,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            Box::pin(async {
                panic!("test-only backend mount_with_locale should not be called on native")
            })
        }

        fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
            let flushes = Rc::clone(&self.flushes);
            Box::pin(async move {
                flushes.set(flushes.get() + 1);
            })
        }

        fn advance_time(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
            let advanced = Rc::clone(&self.advanced);
            Box::pin(async move {
                advanced.borrow_mut().push(duration);
            })
        }
    }

    #[test]
    fn component_trait_compiles_with_mock_machine() {
        fn accepts_component<C: Component>(_component: C) {}

        accepts_component(MockComponent);
        noop_send(MockEvent::Open);
    }

    #[test]
    fn send_named_dispatches_concrete_event() {
        let mut service = mock_service();

        AnyService::send_named(&mut service, "Open");

        assert_eq!(service.state(), &MockState::Open);
    }

    #[test]
    fn send_boxed_accepts_matching_event_type() {
        let mut service = mock_service();

        AnyService::send_boxed(&mut service, Box::new(MockEvent::Open));

        assert_eq!(service.state(), &MockState::Open);
    }

    #[test]
    fn send_boxed_panics_on_mismatched_event_type() {
        let mut service = mock_service();

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
            AnyService::send_boxed(&mut service, Box::new(String::from("Open")));
        }))
        .expect_err("mismatched boxed event should panic");

        #[cfg(not(target_arch = "wasm32"))]
        let message = panic_message(panic.as_ref());

        #[cfg(target_arch = "wasm32")]
        let message = if let Some(message) = panic.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = panic.downcast_ref::<&'static str>() {
            String::from(*message)
        } else {
            String::from("<non-string panic>")
        };

        assert!(message.contains("boxed event type mismatch"));
    }

    #[test]
    fn part_attrs_typed_returns_attrs_for_concrete_part() {
        let service = mock_service();

        let attrs = ServiceHarnessExt::<MockMachine>::part_attrs_typed(&service, MockPart::Trigger);

        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-part")),
            Some(&AttrValue::String(String::from("trigger")))
        );
    }

    #[test]
    fn keyboard_key_variants_report_dom_values() {
        assert_eq!(KeyboardKey::Enter.as_key_value(), "Enter");
        assert_eq!(KeyboardKey::Char('x').as_key_value(), "x");
    }

    #[test]
    fn keyboard_key_variants_cover_all_dom_values() {
        let cases = [
            (KeyboardKey::Space, " "),
            (KeyboardKey::Escape, "Escape"),
            (KeyboardKey::Tab, "Tab"),
            (KeyboardKey::ArrowUp, "ArrowUp"),
            (KeyboardKey::ArrowDown, "ArrowDown"),
            (KeyboardKey::ArrowLeft, "ArrowLeft"),
            (KeyboardKey::ArrowRight, "ArrowRight"),
            (KeyboardKey::Home, "Home"),
            (KeyboardKey::End, "End"),
            (KeyboardKey::PageUp, "PageUp"),
            (KeyboardKey::PageDown, "PageDown"),
            (KeyboardKey::Backspace, "Backspace"),
            (KeyboardKey::Delete, "Delete"),
        ];

        for (key, value) in cases {
            assert_eq!(key.as_key_value(), value);
        }
    }

    #[test]
    fn point_helper_builds_coordinates() {
        assert_eq!(point(2, 3.5), Point { x: 2.0, y: 3.5 });
    }

    #[test]
    fn rect_accessors_match_geometry() {
        let rect = Rect {
            x: 10.0,
            y: 5.0,
            width: 7.5,
            height: 2.5,
        };

        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.top(), 5.0);
        assert_eq!(rect.right(), 17.5);
        assert_eq!(rect.bottom(), 7.5);
    }

    #[test]
    fn any_service_helpers_cover_remaining_paths() {
        let mut service = mock_service();

        assert_eq!(AnyService::state_debug(&service), "Idle");
        assert_eq!(
            AnyService::root_attrs(&service).get_value(&HtmlAttr::Data("ars-part")),
            Some(&AttrValue::String(String::from("root")))
        );
        assert_eq!(
            AnyService::part_attrs(&service, "trigger").get_value(&HtmlAttr::Data("ars-part")),
            Some(&AttrValue::String(String::from("trigger")))
        );

        assert_panics_with_message(
            || AnyService::part_attrs(&service, "missing"),
            "no part named",
        );
        assert_panics_with_message(
            || AnyService::send_named(&mut service, "Missing"),
            "unknown event name",
        );
    }

    #[test]
    fn mock_props_and_machine_cover_remaining_paths() {
        let props = MockProps {
            id: String::from("initial"),
        }
        .with_id(String::from("next"));

        assert_eq!(props.id(), "next");

        let mut props = props;

        props.set_id(String::from("updated"));

        assert_eq!(props.id(), "updated");

        let mut service = mock_service();

        AnyService::send_named(&mut service, "Open");

        assert_eq!(
            AnyService::root_attrs(&service).get_value(&HtmlAttr::Data("ars-state")),
            Some(&AttrValue::String(String::from("open")))
        );

        AnyService::send_named(&mut service, "Close");

        assert_eq!(service.state(), &MockState::Idle);

        let props = MockProps {
            id: String::from("mock"),
        };

        assert!(
            MockMachine::transition(&MockState::Idle, &MockEvent::Close, &MockContext, &props)
                .is_none()
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_test_helpers_cover_remaining_fallback_paths() {
        let string_panic: Box<dyn Any + Send> = Box::new(String::from("owned"));

        let str_panic: Box<dyn Any + Send> = Box::new("borrowed");

        let other_panic: Box<dyn Any + Send> = Box::new(7usize);

        assert_eq!(panic_message(string_panic.as_ref()), "owned");
        assert_eq!(panic_message(str_panic.as_ref()), "borrowed");
        assert_eq!(panic_message(other_panic.as_ref()), "<non-string panic>");

        let helper_panic = std::panic::catch_unwind(|| assert_panics_with_message(|| (), "unused"))
            .expect_err("assert_panics_with_message should fail for non-panicking closures");

        let helper_message = panic_message(helper_panic.as_ref());

        assert!(helper_message.contains("operation should panic"));

        struct PendingFuture;

        impl Future for PendingFuture {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        let pending_panic = std::panic::catch_unwind(|| run_ready(PendingFuture))
            .expect_err("run_ready should panic when a future remains pending");

        let pending_message = panic_message(pending_panic.as_ref());

        assert!(pending_message.contains("Poll::Pending"));

        let waker = Waker::from(Arc::new(NoopWake));

        waker.wake_by_ref();
        waker.wake();

        assert!(format!("{:?}", MockClipboard {}).contains("MockClipboard"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn item_handle_delegates_to_underlying_element() {
        let trigger = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "trigger")
                .with_attr("aria-expanded", "true")
                .with_text("Trigger")
                .with_style("display", "block"),
        );

        let item = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "item")
                .with_text("Item")
                .with_inner_html("<button>Trigger</button>")
                .with_focused(true)
                .with_child("[data-ars-part='trigger']", trigger.clone()),
        );

        let harness = TestHarness::new_for_test(Box::new(mock_service()), Box::new(NoopBackend));

        let item = ItemHandle::new(&harness, item);

        assert_eq!(
            item.trigger_attr("aria-expanded"),
            Some(String::from("true"))
        );
        assert_eq!(item.trigger().text_content(), "Trigger");
        assert_eq!(item.text_content(), "Item");
        assert!(item.is_focused());
        assert_eq!(item.attr("data-ars-part"), Some(String::from("item")));

        let nested_trigger = item.query_selector("[data-ars-part='trigger']");

        assert!(nested_trigger.is_some());
        assert_eq!(
            nested_trigger
                .expect("nested trigger should exist")
                .computed_styles()
                .get("display"),
            Some(&String::from("block"))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn element_handle_stub_accessors_cover_remaining_native_paths() {
        let handle = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-test", "value")
                .with_text("Text")
                .with_inner_html("<span>Text</span>")
                .with_style("display", "grid")
                .with_focused(false)
                .with_child(
                    ".child",
                    ElementHandle::from_stub(NativeElementStub::default().with_text("nested")),
                ),
        );

        assert_eq!(handle.attr("data-test"), Some(String::from("value")));
        assert_eq!(handle.text_content(), "Text");
        assert_eq!(handle.inner_html(), "<span>Text</span>");
        assert_eq!(handle.bounding_rect(), Rect::default());
        assert!(!handle.is_focused());
        assert_eq!(
            handle
                .query_selector(".child")
                .map(|child| child.text_content()),
            Some(String::from("nested"))
        );
        assert!(handle.query_selector(".missing").is_none());
        assert_eq!(
            handle.computed_styles().get("display"),
            Some(&String::from("grid"))
        );

        run_ready(handle.click());
        run_ready(handle.focus());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn item_handle_click_and_focus_flush_backend() {
        let backend = RecordingBackend::default();

        let trigger = ElementHandle::from_stub(
            NativeElementStub::default().with_attr("data-ars-part", "trigger"),
        );

        let item = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "item")
                .with_child("[data-ars-part='trigger']", trigger),
        );

        let harness =
            TestHarness::new_for_test(Box::new(mock_service()), Box::new(backend.clone()));

        let item = ItemHandle::new(&harness, item);

        run_ready(item.click_trigger());
        run_ready(item.focus());

        assert_eq!(backend.flush_count(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn harness_helpers_cover_non_dom_native_paths() {
        let backend = RecordingBackend::default();

        let harness =
            TestHarness::new_for_test(Box::new(mock_service()), Box::new(backend.clone()));

        assert_eq!(harness.state(), "Idle");
        assert_eq!(harness.snapshot_attrs(), BTreeMap::new());
        assert_eq!(harness.record_values(|| vec![1, 2, 3]), vec![1, 2, 3]);

        run_ready(harness.flush());
        run_ready(harness.tick());
        run_ready(harness.emulate_media("prefers-reduced-motion", "reduce"));
        run_ready(harness.send(MockEvent::Open));
        run_ready(harness.advance_time(Duration::from_millis(25)));

        assert_eq!(harness.state(), "Open");
        assert_eq!(backend.flush_count(), 5);
        assert_eq!(
            backend.advanced_durations(),
            vec![Duration::from_millis(25)]
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_and_query_helpers_cover_remaining_native_paths() {
        let backend = MountedNativeBackend::default();

        let harness = run_ready(render_with_backend(MockComponent, backend.clone()));

        assert_eq!(harness.state(), "Idle");
        assert_eq!(backend.flush_count(), 1);

        let locale = Locale::parse("en-US").expect("locale should parse");

        let locale_harness = run_ready(render_with_locale_and_backend(
            MockComponent,
            locale.clone(),
            backend.clone(),
        ));

        assert_eq!(locale_harness.state(), "Idle");
        assert_eq!(backend.flush_count(), 2);
        assert_eq!(backend.mounted_locales(), vec![locale]);

        let root = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "root")
                .with_attr("data-ars-scope", "mock")
                .with_attr("data-ars-state", "open")
                .with_attr("data-ars-extra", "open")
                .with_attr("aria-expanded", "true")
                .with_rect(Rect {
                    x: 20.0,
                    y: 40.0,
                    width: 10.0,
                    height: 10.0,
                }),
        );

        let target = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "trigger")
                .with_attr("aria-expanded", "true")
                .with_rect(Rect {
                    x: 5.0,
                    y: 40.0,
                    width: 10.0,
                    height: 10.0,
                }),
        );

        let input = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "input")
                .with_attr("value", "typed"),
        );

        let control = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "control")
                .with_attr("id", "control-id"),
        );

        let button = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-part", "button")
                .with_attr("type", "button"),
        );

        let focused = ElementHandle::from_stub(
            NativeElementStub::default().with_attr("data-ars-part", "input"),
        );

        let item = ElementHandle::from_stub(
            NativeElementStub::default().with_attr("data-ars-part", "item"),
        );

        let harness =
            TestHarness::new_for_test(Box::new(mock_service()), Box::new(backend.clone()));

        harness.set_query_results(".root", vec![root.clone()]);
        harness.set_query_results(".missing", Vec::new());
        harness.set_query_results(".target", vec![target.clone()]);
        harness.set_query_results("[data-ars-part]", vec![root.clone(), target.clone()]);
        harness.set_query_results("[data-ars-part='trigger']", vec![target.clone()]);
        harness.set_query_results("[data-ars-part='input']", vec![input.clone()]);
        harness.set_query_results("[data-ars-part='control']", vec![control.clone()]);
        harness.set_query_results("[data-ars-part='button']", vec![button.clone()]);
        harness.set_query_results("[data-ars-scope]", vec![root.clone()]);
        harness.set_query_results("[data-ars-part='item']", vec![item.clone()]);
        harness.set_focused_element(Some(focused));

        assert_eq!(
            harness
                .focused_element()
                .and_then(|element| element.attr("data-ars-part")),
            Some(String::from("input"))
        );
        assert_eq!(
            harness.query(".root").attr("data-ars-part"),
            Some(String::from("root"))
        );
        assert_eq!(
            harness.trigger_attr("aria-expanded"),
            Some(String::from("true"))
        );
        assert_eq!(harness.input_attr("value"), Some(String::from("typed")));
        assert_eq!(harness.control_attr("id"), Some(String::from("control-id")));
        assert_eq!(harness.data_attr("state"), Some(String::from("open")));
        assert_eq!(harness.data_attr("extra"), Some(String::from("open")));
        assert_eq!(
            harness.attr(".target", "data-ars-part"),
            Some(String::from("trigger"))
        );
        assert_eq!(harness.button_attr("type"), Some(String::from("button")));
        assert!(harness.is_open());
        assert_eq!(
            harness.snapshot_parts(),
            vec![String::from("root"), String::from("trigger")]
        );
        assert_eq!(
            harness.item(0).attr("data-ars-part"),
            Some(String::from("item"))
        );

        assert_panics_with_message(|| harness.query(".missing"), "no element matched selector");
        assert_panics_with_message(|| harness.item(1), "no item at index 1");

        run_ready(harness.click_selector(".root"));
        run_ready(harness.focus(".target"));
        run_ready(backend.advance_time(Duration::from_millis(3)));

        let visual_order = harness.get_visual_order();

        assert_eq!(backend.flush_count(), 4);
        assert_eq!(
            visual_order
                .iter()
                .filter_map(|handle| handle.attr("data-ars-part"))
                .collect::<Vec<_>>(),
            vec![String::from("trigger"), String::from("root")]
        );

        let aria_only_root = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-scope", "mock")
                .with_attr("data-ars-state", "idle")
                .with_attr("aria-expanded", "true"),
        );

        let expanded_root = ElementHandle::from_stub(
            NativeElementStub::default()
                .with_attr("data-ars-scope", "mock")
                .with_attr("data-ars-state", "expanded")
                .with_attr("aria-expanded", "false"),
        );

        let harness = TestHarness::new_for_test(
            Box::new(mock_service()),
            Box::new(RecordingBackend::default()),
        );

        harness.set_query_results("[data-ars-scope]", vec![aria_only_root]);

        assert!(harness.is_open());

        let harness = TestHarness::new_for_test(
            Box::new(mock_service()),
            Box::new(RecordingBackend::default()),
        );

        harness.set_query_results("[data-ars-scope]", vec![expanded_root]);

        assert!(harness.is_open());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_backends_cover_remaining_test_backend_paths() {
        let container = fake_html_element();

        let noop = NoopBackend;

        assert_panics_with_message(
            || run_ready(noop.mount(&container, Box::new(()))),
            "test-only backend mount",
        );
        assert_panics_with_message(
            || {
                run_ready(noop.mount_with_locale(
                    &container,
                    Box::new(()),
                    Locale::parse("en-US").expect("locale should parse"),
                ))
            },
            "test-only backend mount_with_locale",
        );

        run_ready(noop.flush());
        run_ready(noop.advance_time(Duration::from_millis(1)));

        let backend = RecordingBackend::default();

        assert_panics_with_message(
            || run_ready(backend.mount(&container, Box::new(()))),
            "test-only backend mount",
        );
        assert_panics_with_message(
            || {
                run_ready(backend.mount_with_locale(
                    &container,
                    Box::new(()),
                    Locale::parse("en-US").expect("locale should parse"),
                ))
            },
            "test-only backend mount_with_locale",
        );

        run_ready(backend.flush());
        run_ready(backend.advance_time(Duration::from_millis(2)));

        assert_eq!(backend.flush_count(), 1);
        assert_eq!(backend.advanced_durations(), vec![Duration::from_millis(2)]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_only_public_surface_panics_on_native_runtime() {
        let harness = TestHarness::new_for_test(Box::new(mock_service()), Box::new(NoopBackend));
        let clipboard = MockClipboard {};

        assert_panics_with_message(|| mock_file("file.txt", "body", "text/plain"), "mock_file");
        assert_panics_with_message(|| mock_data_transfer(&[]), "mock_data_transfer");
        assert_panics_with_message(mock_clipboard, "mock_clipboard");
        assert_panics_with_message(
            || clipboard.last_written_text(),
            "MockClipboard::last_written_text",
        );
        assert_panics_with_message(
            || clipboard.set_read_text("seeded"),
            "MockClipboard::set_read_text",
        );
        assert_panics_with_message(
            || clipboard.deny_permission(),
            "MockClipboard::deny_permission",
        );

        let _ = container_html(&ContainerHandle::Native {
            element: fake_html_element(),
        });

        drop(create_isolated_container());

        assert_panics_with_message(
            || run_ready(render_with_backend(MockComponent, NoopBackend)),
            "test-only backend mount",
        );
        assert_panics_with_message(
            || {
                run_ready(render_with_locale_and_backend(
                    MockComponent,
                    Locale::parse("en-US").expect("locale should parse"),
                    NoopBackend,
                ))
            },
            "test-only backend mount_with_locale",
        );

        assert_panics_with_message(|| harness.query_selector(".root"), "query_selector");
        assert_panics_with_message(|| harness.query_selector_all(".root"), "query_selector_all");
        assert_panics_with_message(|| harness.query(".root"), "query_selector");
        assert_panics_with_message(|| harness.focused_element(), "focused_element");
        assert_panics_with_message(|| harness.query_part("trigger"), "query_selector");
        assert_panics_with_message(|| harness.trigger_attr("aria-expanded"), "query_selector");
        assert_panics_with_message(|| harness.input_attr("value"), "query_selector");
        assert_panics_with_message(|| harness.control_attr("id"), "query_selector");
        assert_panics_with_message(|| harness.data_attr("state"), "query_selector");
        assert_panics_with_message(|| harness.attr(".root", "id"), "query_selector");
        assert_panics_with_message(|| harness.button_attr("type"), "query_selector");
        assert_panics_with_message(|| harness.is_focused(), "focused_element");
        assert_panics_with_message(|| harness.get_tab_order(), "query_selector_all");
        assert_panics_with_message(|| harness.get_visual_order(), "query_selector_all");
        assert_panics_with_message(|| harness.is_mounted(), "query_selector");
        assert_panics_with_message(|| harness.is_open(), "query_selector");
        assert_panics_with_message(|| harness.body_has_scroll_lock(), "body_style");
        assert_panics_with_message(|| harness.body_style("overflow"), "body_style");
        assert_panics_with_message(
            || harness.set_body_style("overflow", "hidden"),
            "set_body_style",
        );
        assert_panics_with_message(|| format!("{harness:?}"), "query_selector");
        assert_panics_with_message(|| harness.snapshot_parts(), "query_selector_all");
        assert_panics_with_message(|| harness.item(0), "query_selector_all");
        assert_panics_with_message(|| harness.scroll_to(10, 20), "scroll_to");
        assert_panics_with_message(|| harness.scroll_y(), "scroll_y");
        assert_panics_with_message(|| harness.set_viewport(320.0, 240.0), "set_viewport");
        assert_panics_with_message(
            || {
                harness.set_anchor_position(Rect {
                    x: 1.0,
                    y: 2.0,
                    width: 3.0,
                    height: 4.0,
                });
            },
            "set_anchor_position",
        );
        assert_panics_with_message(
            || run_ready(harness.scroll_container_by(5, 6)),
            "scroll_container_by",
        );

        assert_panics_with_message(|| run_ready(harness.click()), "query_selector");
        assert_panics_with_message(
            || run_ready(harness.click_selector(".root")),
            "query_selector",
        );
        assert_panics_with_message(|| run_ready(harness.type_text("hello")), "type_text");
        assert_panics_with_message(|| run_ready(harness.hover(".root")), "hover");
        assert_panics_with_message(|| run_ready(harness.hover_trigger()), "hover");
        assert_panics_with_message(|| run_ready(harness.blur()), "blur");
        assert_panics_with_message(|| run_ready(harness.focus(".root")), "query_selector");
        assert_panics_with_message(|| run_ready(harness.set_value("next")), "set_value");
        assert_panics_with_message(
            || run_ready(harness.press_key(KeyboardKey::Enter)),
            "press_key",
        );
        assert_panics_with_message(|| run_ready(harness.press(KeyboardKey::Tab)), "press_key");
        assert_panics_with_message(
            || run_ready(harness.key_sequence(KeyboardKey::Space)),
            "key_sequence",
        );
        assert_panics_with_message(
            || run_ready(harness.touch_start(point(1.0, 2.0))),
            "touch_start",
        );
        assert_panics_with_message(
            || run_ready(harness.touch_move(point(3.0, 4.0))),
            "touch_move",
        );
        assert_panics_with_message(|| run_ready(harness.touch_end()), "dispatch_simple_event");
        assert_panics_with_message(
            || run_ready(harness.touch_start_on_trigger()),
            "touch_start_on_trigger",
        );
        assert_panics_with_message(
            || run_ready(harness.touch_move_first(point(5.0, 6.0))),
            "touch_move_first",
        );
        assert_panics_with_message(
            || run_ready(harness.pointer_down_at(7.0, 8.0)),
            "pointer_down_at",
        );
        assert_panics_with_message(
            || run_ready(harness.pointer_move_to(9.0, 10.0)),
            "pointer_move_to",
        );
        assert_panics_with_message(|| run_ready(harness.pointer_up()), "pointer_up");
        assert_panics_with_message(|| run_ready(harness.ime_compose("text")), "ime_compose");
        assert_panics_with_message(|| run_ready(harness.ime_commit()), "ime_commit");
        assert_panics_with_message(
            || run_ready(harness.fire_animation_end()),
            "fire_animation_end",
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[derive(Clone)]
    struct MountedDomBackend {
        flushes: Rc<Cell<u32>>,
    }

    #[cfg(target_arch = "wasm32")]
    impl MountedDomBackend {
        fn new(flushes: Rc<Cell<u32>>) -> Self {
            Self { flushes }
        }

        fn mount_markup(container: &web_sys::HtmlElement) {
            let document = web_sys::window()
                .and_then(|window| window.document())
                .expect("document must exist");

            let root = document
                .create_element("div")
                .expect("root element must be creatable");

            root.set_attribute("data-ars-scope", "mock")
                .expect("scope attribute should set");

            root.set_attribute("data-ars-state", "idle")
                .expect("state attribute should set");

            let trigger = document
                .create_element("button")
                .expect("trigger element must be creatable");

            trigger
                .set_attribute("data-ars-part", "trigger")
                .expect("trigger part attribute should set");

            trigger
                .set_attribute("type", "button")
                .expect("button type should set");

            trigger.set_inner_html("Trigger");

            root.append_child(&trigger)
                .expect("trigger should append to root");

            container
                .append_child(&root)
                .expect("root should append to harness container");
        }
    }

    #[cfg(target_arch = "wasm32")]
    impl HarnessBackend for MountedDomBackend {
        fn mount(
            &self,
            container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            let container = container.clone();
            Box::pin(async move {
                Self::mount_markup(&container);
                Box::new(mock_service()) as Box<dyn AnyService>
            })
        }

        fn mount_with_locale(
            &self,
            container: &web_sys::HtmlElement,
            _component: Box<dyn Any>,
            locale: Locale,
        ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
            let container = container.clone();
            Box::pin(async move {
                Self::mount_markup(&container);
                container
                    .set_attribute("lang", &locale.to_bcp47())
                    .expect("locale attribute should set");
                Box::new(mock_service()) as Box<dyn AnyService>
            })
        }

        fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
            let flushes = Rc::clone(&self.flushes);
            Box::pin(async move {
                flushes.set(flushes.get() + 1);
            })
        }

        fn advance_time(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async {})
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn js_number(value: &JsValue, property: &str) -> f64 {
        js_sys::Reflect::get(value, &JsValue::from_str(property))
            .unwrap_or_else(|_| panic!("property '{property}' should be readable"))
            .as_f64()
            .unwrap_or_else(|| panic!("property '{property}' should be numeric"))
    }

    #[cfg(target_arch = "wasm32")]
    fn js_string(value: &JsValue, property: &str) -> String {
        js_sys::Reflect::get(value, &JsValue::from_str(property))
            .unwrap_or_else(|_| panic!("property '{property}' should be readable"))
            .as_string()
            .unwrap_or_else(|| panic!("property '{property}' should be a string"))
    }

    #[cfg(target_arch = "wasm32")]
    fn js_bool(value: &JsValue, property: &str) -> bool {
        js_sys::Reflect::get(value, &JsValue::from_str(property))
            .unwrap_or_else(|_| panic!("property '{property}' should be readable"))
            .as_bool()
            .unwrap_or_else(|| panic!("property '{property}' should be a bool"))
    }

    #[cfg(target_arch = "wasm32")]
    fn first_changed_touch(event: &web_sys::TouchEvent) -> (f64, f64) {
        let changed_touches =
            js_sys::Reflect::get(event.as_ref(), &JsValue::from_str("changedTouches"))
                .expect("changedTouches should be readable");

        let touches = js_sys::Array::from(&changed_touches);

        let touch = touches.get(0);

        assert!(
            !touch.is_undefined(),
            "changedTouches should contain a primary touch"
        );

        (js_number(&touch, "clientX"), js_number(&touch, "clientY"))
    }

    #[cfg(target_arch = "wasm32")]
    fn touch_list_length(event: &web_sys::TouchEvent, property: &str) -> u32 {
        let touches = js_sys::Reflect::get(event.as_ref(), &JsValue::from_str(property))
            .unwrap_or_else(|_| panic!("property '{property}' should be readable"));

        js_sys::Array::from(&touches).length()
    }

    #[cfg(target_arch = "wasm32")]
    fn assert_same_descriptor(actual: &JsValue, expected: &JsValue, label: &str) {
        assert_eq!(
            actual.is_undefined(),
            expected.is_undefined(),
            "{label} should preserve descriptor presence"
        );

        if actual.is_undefined() {
            return;
        }

        for property in ["configurable", "enumerable", "writable"] {
            let actual_value = js_sys::Reflect::get(actual, &JsValue::from_str(property))
                .unwrap_or_else(|_| panic!("{label} {property} should be readable"));
            let expected_value = js_sys::Reflect::get(expected, &JsValue::from_str(property))
                .unwrap_or_else(|_| panic!("{label} {property} should be readable"));

            assert!(
                js_sys::Object::is(&actual_value, &expected_value),
                "{label} should preserve descriptor field {property}"
            );
        }

        for property in ["get", "set", "value"] {
            let actual_value = js_sys::Reflect::get(actual, &JsValue::from_str(property))
                .unwrap_or_else(|_| panic!("{label} {property} should be readable"));
            let expected_value = js_sys::Reflect::get(expected, &JsValue::from_str(property))
                .unwrap_or_else(|_| panic!("{label} {property} should be readable"));

            assert!(
                js_sys::Object::is(&actual_value, &expected_value),
                "{label} should preserve descriptor field {property}"
            );
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn match_media_matches(query: &str) -> bool {
        let window = web_sys::window().expect("window must exist");
        let match_media = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
            .expect("window.matchMedia should be readable")
            .dyn_into::<js_sys::Function>()
            .expect("window.matchMedia should be a function");

        let media_query_list = match_media
            .call1(window.as_ref(), &JsValue::from_str(query))
            .expect("window.matchMedia should succeed");

        js_bool(&media_query_list, "matches")
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn set_viewport_overrides_container_geometry_and_window_metrics() {
        let flushes = Rc::new(Cell::new(0));

        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::clone(&flushes))).await;

        harness.set_viewport(320.0, 480.0);

        let container = container_html(&harness.container);

        assert_eq!(
            container
                .style()
                .get_property_value("width")
                .ok()
                .as_deref(),
            Some("320px")
        );
        assert_eq!(
            container
                .style()
                .get_property_value("height")
                .ok()
                .as_deref(),
            Some("480px")
        );
        assert_eq!(container.get_bounding_client_rect().width(), 320.0);
        assert_eq!(container.get_bounding_client_rect().height(), 480.0);

        let window = web_sys::window().expect("window must exist");

        assert_eq!(
            window.inner_width().ok().and_then(|value| value.as_f64()),
            Some(320.0)
        );
        assert_eq!(
            window.inner_height().ok().and_then(|value| value.as_f64()),
            Some(480.0)
        );
        assert_eq!(flushes.get(), 1, "initial render should flush once");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn set_viewport_restores_window_metrics_on_drop() {
        let window = web_sys::window().expect("window must exist");
        let document = window.document().expect("document must exist");
        let document_element = document
            .document_element()
            .expect("document element must exist");
        let initial_window_width_descriptor =
            own_property_descriptor(window.as_ref(), "innerWidth");
        let initial_window_height_descriptor =
            own_property_descriptor(window.as_ref(), "innerHeight");
        let initial_document_width_descriptor =
            own_property_descriptor(document_element.as_ref(), "clientWidth");
        let initial_document_height_descriptor =
            own_property_descriptor(document_element.as_ref(), "clientHeight");

        let original_window_width = window
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .expect("window.innerWidth should be readable");
        let original_window_height = window
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .expect("window.innerHeight should be readable");
        let original_document_width = document_element.client_width();
        let original_document_height = document_element.client_height();

        {
            let harness =
                render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0))))
                    .await;

            harness.set_viewport(320.0, 480.0);

            assert_eq!(
                window.inner_width().ok().and_then(|value| value.as_f64()),
                Some(320.0)
            );
            assert_eq!(
                window.inner_height().ok().and_then(|value| value.as_f64()),
                Some(480.0)
            );
            assert_eq!(document_element.client_width(), 320);
            assert_eq!(document_element.client_height(), 480);
            assert!(
                !own_property_descriptor(window.as_ref(), "innerWidth").is_undefined(),
                "viewport override should install an own descriptor for window.innerWidth"
            );
            assert!(
                !own_property_descriptor(window.as_ref(), "innerHeight").is_undefined(),
                "viewport override should install an own descriptor for window.innerHeight"
            );
            assert!(
                !own_property_descriptor(document_element.as_ref(), "clientWidth").is_undefined(),
                "viewport override should install an own descriptor for documentElement.clientWidth"
            );
            assert!(
                !own_property_descriptor(document_element.as_ref(), "clientHeight").is_undefined(),
                "viewport override should install an own descriptor for documentElement.clientHeight"
            );
        }

        assert_eq!(
            window.inner_width().ok().and_then(|value| value.as_f64()),
            Some(original_window_width)
        );
        assert_eq!(
            window.inner_height().ok().and_then(|value| value.as_f64()),
            Some(original_window_height)
        );
        assert_eq!(document_element.client_width(), original_document_width);
        assert_eq!(document_element.client_height(), original_document_height);
        assert_same_descriptor(
            &own_property_descriptor(window.as_ref(), "innerWidth"),
            &initial_window_width_descriptor,
            "window.innerWidth",
        );
        assert_same_descriptor(
            &own_property_descriptor(window.as_ref(), "innerHeight"),
            &initial_window_height_descriptor,
            "window.innerHeight",
        );
        assert_same_descriptor(
            &own_property_descriptor(document_element.as_ref(), "clientWidth"),
            &initial_document_width_descriptor,
            "documentElement.clientWidth",
        );
        assert_same_descriptor(
            &own_property_descriptor(document_element.as_ref(), "clientHeight"),
            &initial_document_height_descriptor,
            "documentElement.clientHeight",
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn set_anchor_position_overrides_trigger_rect_and_tracks_container_scroll() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        harness.set_anchor_position(Rect {
            x: 100.0,
            y: 370.0,
            width: 100.0,
            height: 30.0,
        });

        let trigger = harness.query("[data-ars-part='trigger']");

        assert_eq!(
            trigger.bounding_rect(),
            Rect {
                x: 100.0,
                y: 370.0,
                width: 100.0,
                height: 30.0,
            }
        );

        harness.scroll_container_by(0, 50).await;

        let trigger = harness.query("[data-ars-part='trigger']");

        assert_eq!(trigger.bounding_rect().y, 320.0);
        assert_eq!(container_html(&harness.container).scroll_top(), 50);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn replacing_rect_override_restores_previous_element_descriptor() {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("document must exist");

        let first = document
            .create_element("div")
            .expect("first element must be creatable");
        let second = document
            .create_element("div")
            .expect("second element must be creatable");

        assert!(
            own_property_descriptor(first.as_ref(), "getBoundingClientRect").is_undefined(),
            "first element should not start with an own rect descriptor"
        );
        assert!(
            own_property_descriptor(second.as_ref(), "getBoundingClientRect").is_undefined(),
            "second element should not start with an own rect descriptor"
        );

        let mut layout = WasmLayoutState {
            anchor: Some(install_rect_override(
                &first,
                Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                },
            )),
            ..WasmLayoutState::default()
        };

        assert!(
            !own_property_descriptor(first.as_ref(), "getBoundingClientRect").is_undefined(),
            "first element should be overridden while active"
        );

        drop(layout.anchor.take());

        assert!(
            own_property_descriptor(first.as_ref(), "getBoundingClientRect").is_undefined(),
            "dropping an override should remove the temporary descriptor"
        );

        layout.anchor = Some(install_rect_override(
            &second,
            Rect {
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 60.0,
            },
        ));

        assert!(
            own_property_descriptor(first.as_ref(), "getBoundingClientRect").is_undefined(),
            "replacing the target should leave the previous element restored"
        );
        assert!(
            !own_property_descriptor(second.as_ref(), "getBoundingClientRect").is_undefined(),
            "new target should receive the override"
        );

        drop(layout.anchor.take());

        assert!(
            own_property_descriptor(second.as_ref(), "getBoundingClientRect").is_undefined(),
            "second element should also restore its original descriptor on drop"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn isolated_container_is_removed_on_drop_after_layout_customization() {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("document must exist");

        let flushes = Rc::new(Cell::new(0));

        let container_count_before = document
            .query_selector_all("[data-ars-test-container]")
            .expect("container query must succeed")
            .length();

        {
            let harness =
                render_with_backend(MockComponent, MountedDomBackend::new(Rc::clone(&flushes)))
                    .await;

            harness.set_viewport(400.0, 300.0);
            harness.set_anchor_position(Rect {
                x: 20.0,
                y: 40.0,
                width: 80.0,
                height: 20.0,
            });

            assert_eq!(
                document
                    .query_selector_all("[data-ars-test-container]")
                    .expect("container query must succeed")
                    .length(),
                container_count_before + 1
            );
        }

        assert_eq!(
            document
                .query_selector_all("[data-ars-test-container]")
                .expect("container query must succeed")
                .length(),
            container_count_before
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn keyboard_helpers_set_dom_key_values() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let trigger = harness.query("[data-ars-part='trigger']");
        let trigger_element = trigger.element.clone();

        let pressed_key = Rc::new(RefCell::new(None));

        let _keydown_listener = {
            let pressed_key = Rc::clone(&pressed_key);
            let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
                move |event: web_sys::KeyboardEvent| {
                    *pressed_key.borrow_mut() = Some(js_string(event.as_ref(), "key"));
                },
            ));

            trigger_element
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
                .expect("keydown listener should register");

            closure
        };

        let sequence = Rc::new(RefCell::new(Vec::new()));
        let _sequence_listeners = ["keydown", "keypress", "keyup"]
            .into_iter()
            .map(|event_name| {
                let sequence = Rc::clone(&sequence);
                let trigger_element = trigger_element.clone();
                let event_name = String::from(event_name);
                let listener_event_name = event_name.clone();
                let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
                    move |event: web_sys::KeyboardEvent| {
                        sequence.borrow_mut().push((
                            listener_event_name.clone(),
                            js_string(event.as_ref(), "key"),
                        ));
                    },
                ));

                trigger_element
                    .add_event_listener_with_callback(&event_name, closure.as_ref().unchecked_ref())
                    .expect("keyboard listener should register");

                closure
            })
            .collect::<Vec<_>>();

        harness.focus("[data-ars-part='trigger']").await;
        harness.press_key(KeyboardKey::Enter).await;

        assert_eq!(*pressed_key.borrow(), Some(String::from("Enter")));

        sequence.borrow_mut().clear();

        harness.key_sequence(KeyboardKey::Space).await;

        assert_eq!(
            sequence.borrow().as_slice(),
            &[
                (String::from("keydown"), String::from(" ")),
                (String::from("keypress"), String::from(" ")),
                (String::from("keyup"), String::from(" ")),
            ]
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn pointer_and_root_touch_helpers_preserve_coordinates() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let root = harness.query("[data-ars-scope]").element;

        let pointer_down = Rc::new(RefCell::new(None));

        let pointer_moves = Rc::new(RefCell::new(Vec::new()));

        let pointer_up = Rc::new(RefCell::new(None));

        let touch_start = Rc::new(RefCell::new(None));

        let touch_move = Rc::new(RefCell::new(None));

        let touch_end = Rc::new(RefCell::new(None));

        let _pointer_down_listener = {
            let pointer_down = Rc::clone(&pointer_down);
            let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::wrap(Box::new(
                move |event: web_sys::PointerEvent| {
                    *pointer_down.borrow_mut() = Some((
                        js_number(event.as_ref(), "clientX"),
                        js_number(event.as_ref(), "clientY"),
                        js_string(event.as_ref(), "pointerType"),
                        js_number(event.as_ref(), "buttons"),
                    ));
                },
            ));

            root.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())
                .expect("pointerdown listener should register");

            closure
        };

        let _pointer_move_listener = {
            let pointer_moves = Rc::clone(&pointer_moves);
            let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::wrap(Box::new(
                move |event: web_sys::PointerEvent| {
                    pointer_moves.borrow_mut().push((
                        js_number(event.as_ref(), "clientX"),
                        js_number(event.as_ref(), "clientY"),
                        js_string(event.as_ref(), "pointerType"),
                        js_number(event.as_ref(), "buttons"),
                    ));
                },
            ));

            root.add_event_listener_with_callback("pointermove", closure.as_ref().unchecked_ref())
                .expect("pointermove listener should register");

            closure
        };

        let _pointer_up_listener = {
            let pointer_up = Rc::clone(&pointer_up);
            let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::wrap(Box::new(
                move |event: web_sys::PointerEvent| {
                    *pointer_up.borrow_mut() = Some((
                        js_number(event.as_ref(), "clientX"),
                        js_number(event.as_ref(), "clientY"),
                        js_string(event.as_ref(), "pointerType"),
                        js_number(event.as_ref(), "pointerId"),
                        js_number(event.as_ref(), "buttons"),
                    ));
                },
            ));

            root.add_event_listener_with_callback("pointerup", closure.as_ref().unchecked_ref())
                .expect("pointerup listener should register");

            closure
        };

        let _touch_start_listener = {
            let touch_start = Rc::clone(&touch_start);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *touch_start.borrow_mut() = Some(first_changed_touch(&event));
                },
            ));

            root.add_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref())
                .expect("touchstart listener should register");

            closure
        };

        let _touch_move_listener = {
            let touch_move = Rc::clone(&touch_move);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *touch_move.borrow_mut() = Some(first_changed_touch(&event));
                },
            ));

            root.add_event_listener_with_callback("touchmove", closure.as_ref().unchecked_ref())
                .expect("touchmove listener should register");

            closure
        };

        let _touch_end_listener = {
            let touch_end = Rc::clone(&touch_end);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *touch_end.borrow_mut() = Some((
                        first_changed_touch(&event),
                        touch_list_length(&event, "touches"),
                        touch_list_length(&event, "targetTouches"),
                    ));
                },
            ));

            root.add_event_listener_with_callback("touchend", closure.as_ref().unchecked_ref())
                .expect("touchend listener should register");

            closure
        };

        harness.touch_start(point(12.0, 34.0)).await;
        harness.touch_move(point(56.0, 78.0)).await;
        harness.touch_end().await;
        harness.pointer_move_to(17.0, 29.0).await;
        harness.pointer_down_at(91.0, 123.0).await;
        harness.pointer_move_to(145.0, 167.0).await;
        harness.pointer_up().await;

        assert_eq!(*touch_start.borrow(), Some((12.0, 34.0)));
        assert_eq!(*touch_move.borrow(), Some((56.0, 78.0)));
        assert_eq!(*touch_end.borrow(), Some(((56.0, 78.0), 0, 0)));
        assert_eq!(
            *pointer_down.borrow(),
            Some((91.0, 123.0, String::from("mouse"), 1.0))
        );
        assert_eq!(
            *pointer_moves.borrow(),
            vec![
                (17.0, 29.0, String::from("mouse"), 0.0),
                (145.0, 167.0, String::from("mouse"), 1.0),
            ]
        );
        assert_eq!(
            *pointer_up.borrow(),
            Some((145.0, 167.0, String::from("mouse"), 1.0, 0.0))
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn trigger_touch_and_ime_helpers_preserve_payloads() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let trigger = harness.query("[data-ars-part='trigger']");

        let trigger_element = trigger.element.clone();

        let trigger_touch_start = Rc::new(RefCell::new(None));

        let trigger_touch_move = Rc::new(RefCell::new(None));

        let trigger_touch_end = Rc::new(RefCell::new(None));

        let composition_start = Rc::new(RefCell::new(None));

        let composition_update = Rc::new(RefCell::new(None));

        let _trigger_touch_start_listener = {
            let trigger_touch_start = Rc::clone(&trigger_touch_start);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *trigger_touch_start.borrow_mut() = Some(first_changed_touch(&event));
                },
            ));

            trigger_element
                .add_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref())
                .expect("trigger touchstart listener should register");

            closure
        };

        let _trigger_touch_move_listener = {
            let trigger_touch_move = Rc::clone(&trigger_touch_move);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *trigger_touch_move.borrow_mut() = Some(first_changed_touch(&event));
                },
            ));

            trigger_element
                .add_event_listener_with_callback("touchmove", closure.as_ref().unchecked_ref())
                .expect("trigger touchmove listener should register");

            closure
        };

        let _trigger_touch_end_listener = {
            let trigger_touch_end = Rc::clone(&trigger_touch_end);
            let closure = Closure::<dyn FnMut(web_sys::TouchEvent)>::wrap(Box::new(
                move |event: web_sys::TouchEvent| {
                    *trigger_touch_end.borrow_mut() = Some((
                        first_changed_touch(&event),
                        touch_list_length(&event, "touches"),
                        touch_list_length(&event, "targetTouches"),
                    ));
                },
            ));

            trigger_element
                .add_event_listener_with_callback("touchend", closure.as_ref().unchecked_ref())
                .expect("trigger touchend listener should register");

            closure
        };

        let _composition_start_listener = {
            let composition_start = Rc::clone(&composition_start);
            let closure = Closure::<dyn FnMut(web_sys::CompositionEvent)>::wrap(Box::new(
                move |event: web_sys::CompositionEvent| {
                    *composition_start.borrow_mut() = Some(js_string(event.as_ref(), "data"));
                },
            ));

            trigger_element
                .add_event_listener_with_callback(
                    "compositionstart",
                    closure.as_ref().unchecked_ref(),
                )
                .expect("compositionstart listener should register");

            closure
        };

        let _composition_update_listener = {
            let composition_update = Rc::clone(&composition_update);
            let closure = Closure::<dyn FnMut(web_sys::CompositionEvent)>::wrap(Box::new(
                move |event: web_sys::CompositionEvent| {
                    *composition_update.borrow_mut() = Some(js_string(event.as_ref(), "data"));
                },
            ));

            trigger_element
                .add_event_listener_with_callback(
                    "compositionupdate",
                    closure.as_ref().unchecked_ref(),
                )
                .expect("compositionupdate listener should register");

            closure
        };

        let rect = trigger.bounding_rect();

        let expected_center = (
            (rect.x + (rect.width / 2.0)).round(),
            (rect.y + (rect.height / 2.0)).round(),
        );

        harness.touch_start_on_trigger().await;
        harness.touch_move_first(point(201.0, 305.0)).await;
        harness.touch_end().await;
        harness.focus("[data-ars-part='trigger']").await;
        harness.ime_compose("漢字").await;

        assert_eq!(*trigger_touch_start.borrow(), Some(expected_center));
        assert_eq!(*trigger_touch_move.borrow(), Some((201.0, 305.0)));
        assert_eq!(*trigger_touch_end.borrow(), Some(((201.0, 305.0), 0, 0)));
        assert_eq!(*composition_start.borrow(), Some(String::new()));
        assert_eq!(*composition_update.borrow(), Some(String::from("漢字")));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn emulate_media_overrides_match_media_queries() {
        let window = web_sys::window().expect("window must exist");
        let initial_match_media_descriptor = own_property_descriptor(window.as_ref(), "matchMedia");

        {
            let harness =
                render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0))))
                    .await;

            harness
                .emulate_media("prefers-reduced-motion", "reduce")
                .await;

            assert!(match_media_matches("(prefers-reduced-motion: reduce)"));
            assert!(!match_media_matches(
                "(prefers-reduced-motion: no-preference)"
            ));
            assert!(!match_media_matches(
                "(prefers-reduced-motion: reduce) and (min-width: 100000px)"
            ));

            harness
                .emulate_media("prefers-reduced-motion", "no-preference")
                .await;

            assert!(!match_media_matches("(prefers-reduced-motion: reduce)"));
            assert!(match_media_matches(
                "(prefers-reduced-motion: no-preference)"
            ));
        }

        assert_same_descriptor(
            &own_property_descriptor(window.as_ref(), "matchMedia"),
            &initial_match_media_descriptor,
            "window.matchMedia",
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn get_tab_order_excludes_disabled_and_negative_tabindex_elements() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("document must exist");
        let root = harness.query("[data-ars-scope]").element;

        let enabled_input = document
            .create_element("input")
            .expect("input element must be creatable");
        enabled_input
            .set_attribute("id", "enabled-input")
            .expect("input id should set");

        let disabled_button = document
            .create_element("button")
            .expect("button element must be creatable");
        disabled_button
            .set_attribute("id", "disabled-button")
            .expect("button id should set");
        disabled_button
            .set_attribute("disabled", "")
            .expect("disabled attribute should set");

        let skipped_link = document
            .create_element("a")
            .expect("link element must be creatable");
        skipped_link
            .set_attribute("id", "skipped-link")
            .expect("link id should set");
        skipped_link
            .set_attribute("href", "#")
            .expect("link href should set");
        skipped_link
            .set_attribute("tabindex", "-1")
            .expect("link tabindex should set");

        let custom_tab_stop = document
            .create_element("div")
            .expect("div element must be creatable");
        custom_tab_stop
            .set_attribute("id", "custom-tab-stop")
            .expect("div id should set");
        custom_tab_stop
            .set_attribute("tabindex", "0")
            .expect("div tabindex should set");

        for element in [
            enabled_input,
            disabled_button,
            skipped_link,
            custom_tab_stop,
        ] {
            root.append_child(&element)
                .expect("tab-order fixture should append");
        }

        let tab_order = harness
            .get_tab_order()
            .into_iter()
            .map(|element| {
                element
                    .attr("id")
                    .or_else(|| element.attr("data-ars-part"))
                    .expect("fixture elements should expose a stable identifier")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            tab_order,
            vec![
                String::from("trigger"),
                String::from("enabled-input"),
                String::from("custom-tab-stop"),
            ]
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn input_helpers_dispatch_bubbling_input_events() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let root = harness.query("[data-ars-scope]").element;
        let trigger = harness.query("[data-ars-part='trigger']").element;

        let input_events = Rc::new(RefCell::new(Vec::new()));

        let _input_listener = {
            let input_events = Rc::clone(&input_events);
            let closure = Closure::<dyn FnMut(web_sys::InputEvent)>::wrap(Box::new(
                move |event: web_sys::InputEvent| {
                    input_events.borrow_mut().push((
                        event.input_type(),
                        js_bool(event.as_ref(), "bubbles"),
                        js_bool(event.as_ref(), "composed"),
                        event.is_composing(),
                    ));
                },
            ));

            root.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
                .expect("input listener should register");

            closure
        };

        harness.focus("[data-ars-part='trigger']").await;
        harness.type_text("hello").await;
        harness.set_value("next").await;
        harness.ime_commit().await;

        assert_eq!(
            *input_events.borrow(),
            vec![
                (String::new(), true, true, false),
                (String::new(), true, true, false),
                (String::new(), true, true, false),
            ]
        );

        let current_value = js_sys::Reflect::get(trigger.as_ref(), &JsValue::from_str("value"))
            .expect("value should be readable")
            .as_string()
            .expect("value should be a string");

        assert_eq!(current_value, "next");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn hover_dispatches_bubbling_mouseover_events() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let root = harness.query("[data-ars-scope]").element;

        let mouseover = Rc::new(RefCell::new(None));

        let _mouseover_listener = {
            let mouseover = Rc::clone(&mouseover);
            let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    *mouseover.borrow_mut() =
                        Some((event.bubbles(), event.cancelable(), event.type_()));
                },
            ));

            root.add_event_listener_with_callback("mouseover", closure.as_ref().unchecked_ref())
                .expect("mouseover listener should register");

            closure
        };

        harness.hover("[data-ars-part='trigger']").await;

        assert_eq!(
            *mouseover.borrow(),
            Some((true, true, String::from("mouseover")))
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn scroll_container_by_flushes_before_returning() {
        let flushes = Rc::new(Cell::new(0));

        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::clone(&flushes))).await;

        assert_eq!(flushes.get(), 1, "initial render should flush once");

        harness.scroll_container_by(0, 50).await;

        assert_eq!(flushes.get(), 2, "scroll should flush before returning");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn scroll_container_by_only_dispatches_container_scroll_events() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let container = container_html(&harness.container).clone();
        let window = web_sys::window().expect("window must exist");
        let container_scrolls = Rc::new(Cell::new(0));
        let window_scrolls = Rc::new(Cell::new(0));

        let _container_scroll_listener = {
            let container_scrolls = Rc::clone(&container_scrolls);
            let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                container_scrolls.set(container_scrolls.get() + 1);
            }));

            container
                .add_event_listener_with_callback("scroll", closure.as_ref().unchecked_ref())
                .expect("container scroll listener should register");

            closure
        };

        let _window_scroll_listener = {
            let window_scrolls = Rc::clone(&window_scrolls);
            let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                window_scrolls.set(window_scrolls.get() + 1);
            }));

            window
                .add_event_listener_with_callback("scroll", closure.as_ref().unchecked_ref())
                .expect("window scroll listener should register");

            closure
        };

        harness.scroll_container_by(0, 50).await;

        assert_eq!(container_scrolls.get(), 1);
        assert_eq!(window_scrolls.get(), 0);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn animation_helper_dispatches_typed_bubbling_animation_event() {
        let harness =
            render_with_backend(MockComponent, MountedDomBackend::new(Rc::new(Cell::new(0)))).await;

        let root = harness.query("[data-ars-scope]").element;
        let animation_end = Rc::new(RefCell::new(None));

        let _animation_listener = {
            let animation_end = Rc::clone(&animation_end);
            let closure = Closure::<dyn FnMut(web_sys::AnimationEvent)>::wrap(Box::new(
                move |event: web_sys::AnimationEvent| {
                    *animation_end.borrow_mut() = Some((
                        event.animation_name(),
                        event.elapsed_time(),
                        event.pseudo_element(),
                        event.bubbles(),
                    ));
                },
            ));

            root.add_event_listener_with_callback("animationend", closure.as_ref().unchecked_ref())
                .expect("animationend listener should register");

            closure
        };

        harness.fire_animation_end().await;

        assert_eq!(
            *animation_end.borrow(),
            Some((String::new(), 0.0, String::new(), true))
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn mock_file_and_data_transfer_create_real_browser_payloads() {
        let file = mock_file("hello.txt", "hello world", "text/plain");

        assert_eq!(file.name(), "hello.txt");
        assert_eq!(
            js_sys::Reflect::get(file.as_ref(), &JsValue::from_str("type"))
                .ok()
                .and_then(|value| value.as_string())
                .as_deref(),
            Some("text/plain")
        );

        let data_transfer = mock_data_transfer(std::slice::from_ref(&file));

        assert_eq!(data_transfer.items().length(), 1);
        assert_eq!(
            data_transfer
                .files()
                .expect("files list should exist")
                .length(),
            1
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn mock_clipboard_patches_navigator_and_restores_original_value() {
        let navigator = web_sys::window().expect("window must exist").navigator();
        let original_clipboard_descriptor =
            own_property_descriptor(navigator.as_ref(), "clipboard");

        let original_clipboard =
            js_sys::Reflect::get(navigator.as_ref(), &JsValue::from_str("clipboard"))
                .unwrap_or(JsValue::UNDEFINED);

        {
            let clipboard = mock_clipboard();

            assert_eq!(clipboard.last_written_text(), None);

            let installed_clipboard =
                js_sys::Reflect::get(navigator.as_ref(), &JsValue::from_str("clipboard"))
                    .expect("clipboard property should be readable");

            let write_text =
                js_sys::Reflect::get(&installed_clipboard, &JsValue::from_str("writeText"))
                    .expect("writeText should be installed")
                    .dyn_into::<js_sys::Function>()
                    .expect("writeText should be callable");

            let _promise = write_text
                .call1(&installed_clipboard, &JsValue::from_str("copied"))
                .expect("writeText call should succeed");

            assert_eq!(clipboard.last_written_text().as_deref(), Some("copied"));

            clipboard.set_read_text("seeded");

            assert_eq!(clipboard.state.borrow().read_text, "seeded");

            clipboard.deny_permission();

            let _promise = write_text
                .call1(&installed_clipboard, &JsValue::from_str("blocked"))
                .expect("denied writeText call still returns a Promise");

            assert_eq!(
                clipboard.last_written_text().as_deref(),
                Some("copied"),
                "denied writes must not update the recorded value"
            );
        }

        let restored_clipboard =
            js_sys::Reflect::get(navigator.as_ref(), &JsValue::from_str("clipboard"))
                .unwrap_or(JsValue::UNDEFINED);

        assert_same_descriptor(
            &own_property_descriptor(navigator.as_ref(), "clipboard"),
            &original_clipboard_descriptor,
            "navigator.clipboard",
        );
        assert!(
            js_sys::Object::is(&restored_clipboard, &original_clipboard),
            "dropping MockClipboard should restore the original navigator.clipboard value"
        );
    }
}
