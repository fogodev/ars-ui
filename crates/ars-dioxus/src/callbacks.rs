//! Optional event-callback helpers for Dioxus adapter components.

use dioxus::prelude::EventHandler;

/// Emits a value through an optional Dioxus event handler.
pub fn emit<T: 'static>(handler: Option<&EventHandler<T>>, value: T) {
    if let Some(handler) = handler {
        handler.call(value);
    }
}

/// Maps a value before emitting it through an optional Dioxus event handler.
pub fn emit_map<T, U: 'static>(handler: Option<&EventHandler<U>>, value: T, f: impl Fn(T) -> U) {
    if let Some(handler) = handler {
        handler.call(f(value));
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use super::{emit, emit_map};

    #[test]
    fn emit_dispatches_when_handler_is_present() {
        fn app() -> Element {
            let captured = use_signal(Vec::<u32>::new);

            let mut captured_for_handler = captured;

            let handler = EventHandler::new(move |value| {
                captured_for_handler.write().push(value);
            });

            emit(Some(&handler), 7);

            assert_eq!(captured.peek().as_slice(), &[7]);

            rsx! {
                div {}

            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn emit_is_noop_when_handler_is_absent() {
        emit::<u32>(None, 7);
    }

    #[test]
    fn emit_map_transforms_before_dispatch() {
        fn app() -> Element {
            let captured = use_signal(Vec::<String>::new);

            let mut captured_for_handler = captured;

            let handler = EventHandler::new(move |value| {
                captured_for_handler.write().push(value);
            });

            emit_map(Some(&handler), 7, |value| format!("value-{value}"));

            assert_eq!(captured.peek().as_slice(), &[String::from("value-7")]);

            rsx! {
                div {}

            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn emit_map_does_not_transform_when_handler_is_absent() {
        let transformed = std::cell::Cell::new(false);

        emit_map::<_, u32>(None, 7, |_| {
            transformed.set(true);
            8
        });

        assert!(!transformed.get());
    }
}
