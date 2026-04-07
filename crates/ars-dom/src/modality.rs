//! Web listener management for the shared modality context.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use std::cell::RefCell;
use std::{cell::Cell, fmt, rc::Rc};

use ars_a11y::FocusRing;
use ars_core::{KeyModifiers, KeyboardKey, ModalityContext, PointerType};

/// Adapter-facing coordinator that keeps shared modality and focus-visible state in sync.
pub struct ModalityManager {
    modality: Rc<dyn ModalityContext>,
    focus_ring: FocusRing,
    listeners_installed: Cell<bool>,
    listener_refcount: Cell<u32>,
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    listeners: RefCell<Option<WasmListenerHandles>>,
}

impl ModalityManager {
    /// Creates a new modality manager for a single provider root.
    #[must_use]
    pub fn new(modality: Rc<dyn ModalityContext>) -> Self {
        Self {
            modality,
            focus_ring: FocusRing::new(),
            listeners_installed: Cell::new(false),
            listener_refcount: Cell::new(0),
            #[cfg(all(feature = "web", target_arch = "wasm32"))]
            listeners: RefCell::new(None),
        }
    }

    /// Returns the shared modality context owned by this manager.
    #[must_use]
    pub fn modality(&self) -> Rc<dyn ModalityContext> {
        Rc::clone(&self.modality)
    }

    /// Returns the accessibility focus-ring tracker kept in sync with modality events.
    #[must_use]
    pub const fn focus_ring(&self) -> &FocusRing {
        &self.focus_ring
    }

    /// Records a keyboard interaction in both modality and focus-visible tracking.
    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        self.modality.on_key_down(key, modifiers);
        self.focus_ring.on_key_down(key, modifiers);
    }

    /// Records a pointer interaction in both modality and focus-visible tracking.
    pub fn on_pointer_down(&self, pointer_type: PointerType) {
        self.modality.on_pointer_down(pointer_type);
        self.focus_ring.on_pointer_down();
    }

    /// Records a virtual interaction in both modality and focus-visible tracking.
    pub fn on_virtual_input(&self) {
        self.modality.on_virtual_input();
        self.focus_ring.on_virtual_input();
    }

    /// Installs browser-level modality listeners when DOM access is available.
    pub fn ensure_listeners(&self) {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            let Some(document) = document() else {
                return;
            };

            if self.acquire_listener_consumer() {
                self.install_wasm_listeners(&document);
            }
        }
    }

    /// Removes browser-level modality listeners when the last consumer releases them.
    pub fn remove_listeners(&self) {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            let Some(document) = document() else {
                return;
            };

            if self.release_listener_consumer() {
                self.uninstall_wasm_listeners(&document);
            }
        }
    }

    #[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
    fn acquire_listener_consumer(&self) -> bool {
        self.listener_refcount
            .set(self.listener_refcount.get().saturating_add(1));

        let should_install = !self.listeners_installed.get();
        if should_install {
            self.listeners_installed.set(true);
        }
        should_install
    }

    #[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
    fn release_listener_consumer(&self) -> bool {
        let count = self.listener_refcount.get();
        if count == 0 {
            return false;
        }

        let next = count - 1;
        self.listener_refcount.set(next);

        if next == 0 {
            self.listeners_installed.set(false);
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    fn listener_state(&self) -> (bool, u32) {
        (self.listeners_installed.get(), self.listener_refcount.get())
    }
}

impl fmt::Debug for ModalityManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModalityManager")
            .field("modality", &"<dyn ModalityContext>")
            .field("focus_ring", &self.focus_ring)
            .field("listeners_installed", &self.listeners_installed.get())
            .field("listener_refcount", &self.listener_refcount.get())
            .finish()
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::{JsCast, closure::Closure};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use web_sys::{Document, FocusEvent, KeyboardEvent, MouseEvent, PointerEvent, TouchEvent};

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct WasmListenerHandles {
    keydown: Closure<dyn FnMut(KeyboardEvent)>,
    pointerdown: Closure<dyn FnMut(PointerEvent)>,
    mousedown: Closure<dyn FnMut(MouseEvent)>,
    touchstart: Closure<dyn FnMut(TouchEvent)>,
    focus: Closure<dyn FnMut(FocusEvent)>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<Document> {
    web_sys::window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl ModalityManager {
    fn install_wasm_listeners(&self, document: &Document) {
        let modality = self.modality();
        let focus_ring = self.focus_ring.clone();
        let keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let key = KeyboardKey::from_key_str(&event.key());
            let modifiers = KeyModifiers {
                shift: event.shift_key(),
                ctrl: event.ctrl_key(),
                alt: event.alt_key(),
                meta: event.meta_key(),
            };
            modality.on_key_down(key, modifiers);
            focus_ring.on_key_down(key, modifiers);
        }) as Box<dyn FnMut(KeyboardEvent)>);

        let modality = self.modality();
        let focus_ring = self.focus_ring.clone();
        let pointerdown = Closure::wrap(Box::new(move |event: PointerEvent| {
            let pointer_type = pointer_type_from_web(&event.pointer_type());
            modality.on_pointer_down(pointer_type);
            focus_ring.on_pointer_down();
        }) as Box<dyn FnMut(PointerEvent)>);

        let modality = self.modality();
        let focus_ring = self.focus_ring.clone();
        let mousedown = Closure::wrap(Box::new(move |_event: MouseEvent| {
            modality.on_pointer_down(PointerType::Mouse);
            focus_ring.on_pointer_down();
        }) as Box<dyn FnMut(MouseEvent)>);

        let modality = self.modality();
        let focus_ring = self.focus_ring.clone();
        let touchstart = Closure::wrap(Box::new(move |_event: TouchEvent| {
            modality.on_pointer_down(PointerType::Touch);
            focus_ring.on_pointer_down();
        }) as Box<dyn FnMut(TouchEvent)>);

        let focus =
            Closure::wrap(Box::new(move |_event: FocusEvent| {}) as Box<dyn FnMut(FocusEvent)>);

        let keydown_result =
            document.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
        debug_assert!(
            keydown_result.is_ok(),
            "failed to attach keydown modality listener"
        );

        let pointerdown_result = document
            .add_event_listener_with_callback("pointerdown", pointerdown.as_ref().unchecked_ref());
        debug_assert!(
            pointerdown_result.is_ok(),
            "failed to attach pointerdown modality listener"
        );

        let mousedown_result = document
            .add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref());
        debug_assert!(
            mousedown_result.is_ok(),
            "failed to attach mousedown modality listener"
        );

        let touchstart_result = document
            .add_event_listener_with_callback("touchstart", touchstart.as_ref().unchecked_ref());
        debug_assert!(
            touchstart_result.is_ok(),
            "failed to attach touchstart modality listener"
        );

        let focus_result = document.add_event_listener_with_callback_and_bool(
            "focus",
            focus.as_ref().unchecked_ref(),
            true,
        );
        debug_assert!(
            focus_result.is_ok(),
            "failed to attach focus modality listener"
        );

        self.listeners.replace(Some(WasmListenerHandles {
            keydown,
            pointerdown,
            mousedown,
            touchstart,
            focus,
        }));
    }

    fn uninstall_wasm_listeners(&self, document: &Document) {
        if let Some(handles) = self.listeners.borrow_mut().take() {
            let keydown_result = document.remove_event_listener_with_callback(
                "keydown",
                handles.keydown.as_ref().unchecked_ref(),
            );
            debug_assert!(
                keydown_result.is_ok(),
                "failed to detach keydown modality listener"
            );

            let pointerdown_result = document.remove_event_listener_with_callback(
                "pointerdown",
                handles.pointerdown.as_ref().unchecked_ref(),
            );
            debug_assert!(
                pointerdown_result.is_ok(),
                "failed to detach pointerdown modality listener"
            );

            let mousedown_result = document.remove_event_listener_with_callback(
                "mousedown",
                handles.mousedown.as_ref().unchecked_ref(),
            );
            debug_assert!(
                mousedown_result.is_ok(),
                "failed to detach mousedown modality listener"
            );

            let touchstart_result = document.remove_event_listener_with_callback(
                "touchstart",
                handles.touchstart.as_ref().unchecked_ref(),
            );
            debug_assert!(
                touchstart_result.is_ok(),
                "failed to detach touchstart modality listener"
            );

            let focus_result = document.remove_event_listener_with_callback_and_bool(
                "focus",
                handles.focus.as_ref().unchecked_ref(),
                true,
            );
            debug_assert!(
                focus_result.is_ok(),
                "failed to detach focus modality listener"
            );
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn pointer_type_from_web(pointer_type: &str) -> PointerType {
    match pointer_type {
        "touch" => PointerType::Touch,
        "pen" => PointerType::Pen,
        _ => PointerType::Mouse,
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{DefaultModalityContext, ModalitySnapshot};

    use super::*;

    #[test]
    fn keydown_updates_modality_and_focus_ring() {
        let modality: Rc<dyn ModalityContext> = Rc::new(DefaultModalityContext::new());
        let manager = ModalityManager::new(Rc::clone(&modality));

        manager.on_key_down(KeyboardKey::Tab, KeyModifiers::default());

        assert_eq!(
            manager.modality().snapshot(),
            ModalitySnapshot {
                last_pointer_type: Some(PointerType::Keyboard),
                global_press_active: false,
            }
        );
        assert!(manager.focus_ring().should_show_focus_ring());
    }

    #[test]
    fn modality_accessor_returns_shared_context() {
        let modality: Rc<dyn ModalityContext> = Rc::new(DefaultModalityContext::new());
        let manager = ModalityManager::new(Rc::clone(&modality));

        let owned = manager.modality();
        assert!(Rc::ptr_eq(&owned, &modality));
    }

    #[test]
    fn pointerdown_updates_modality_and_clears_focus_ring() {
        let modality: Rc<dyn ModalityContext> = Rc::new(DefaultModalityContext::new());
        let manager = ModalityManager::new(Rc::clone(&modality));

        manager.on_key_down(KeyboardKey::Enter, KeyModifiers::default());
        manager.on_pointer_down(PointerType::Touch);

        assert_eq!(modality.last_pointer_type(), Some(PointerType::Touch));
        assert!(modality.had_pointer_interaction());
        assert!(!manager.focus_ring().should_show_focus_ring());
    }

    #[test]
    fn pen_pointerdown_counts_as_pointer_interaction() {
        let modality: Rc<dyn ModalityContext> = Rc::new(DefaultModalityContext::new());
        let manager = ModalityManager::new(Rc::clone(&modality));

        manager.on_pointer_down(PointerType::Pen);

        assert_eq!(modality.last_pointer_type(), Some(PointerType::Pen));
        assert!(modality.had_pointer_interaction());
        assert!(!manager.focus_ring().should_show_focus_ring());
    }

    #[test]
    fn virtual_input_updates_both_trackers() {
        let modality: Rc<dyn ModalityContext> = Rc::new(DefaultModalityContext::new());
        let manager = ModalityManager::new(modality);

        manager.on_virtual_input();

        assert_eq!(
            manager.modality().snapshot(),
            ModalitySnapshot {
                last_pointer_type: Some(PointerType::Virtual),
                global_press_active: false,
            }
        );
        assert!(manager.focus_ring().should_show_focus_ring());
    }

    #[test]
    fn listener_refcount_transitions_are_stable() {
        let manager = ModalityManager::new(Rc::new(DefaultModalityContext::new()));

        assert_eq!(manager.listener_state(), (false, 0));

        assert!(manager.acquire_listener_consumer());
        assert_eq!(manager.listener_state(), (true, 1));

        assert!(!manager.acquire_listener_consumer());
        assert_eq!(manager.listener_state(), (true, 2));

        assert!(!manager.release_listener_consumer());
        assert_eq!(manager.listener_state(), (true, 1));

        assert!(manager.release_listener_consumer());
        assert_eq!(manager.listener_state(), (false, 0));

        assert!(!manager.release_listener_consumer());
        assert_eq!(manager.listener_state(), (false, 0));
    }

    #[test]
    fn host_listener_api_is_safe_to_call() {
        let manager = ModalityManager::new(Rc::new(DefaultModalityContext::new()));
        manager.ensure_listeners();
        manager.remove_listeners();
    }
}
