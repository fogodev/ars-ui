//! Controlled-prop synchronization helpers for Leptos components.

use leptos::prelude::*;

/// Watches a reactive signal and dispatches an event when the value changes.
///
/// The initial signal value is ignored because the machine already received the
/// correct props during initialization.
pub fn use_controlled_prop<T, E>(
    signal: Signal<T>,
    send: Callback<E>,
    event_fn: impl Fn(T) -> E + Send + Sync + 'static,
) where
    T: Clone + PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    let prev: StoredValue<Option<T>> = StoredValue::new(None);
    let effect = ImmediateEffect::new_isomorphic(move || {
        let new_value = signal.get();
        let should_send = prev.with_value(|current| current.as_ref() != Some(&new_value));
        if should_send {
            let is_initial = prev.with_value(Option::is_none);
            if !is_initial {
                send.run(event_fn(new_value.clone()));
            }
            prev.set_value(Some(new_value));
        }
    });
    on_cleanup(move || drop(effect));
}

#[cfg(test)]
mod tests {
    use leptos::{prelude::*, reactive::traits::GetUntracked};

    use super::use_controlled_prop;

    #[test]
    fn use_controlled_prop_skips_initial_value_and_dispatches_on_change() {
        let owner = Owner::new();
        owner.with(|| {
            let (value, set_value) = signal(1u32);
            let events: StoredValue<Vec<u32>> = StoredValue::new(Vec::new());
            let send = Callback::new(move |next: u32| {
                events.update_value(|captured| captured.push(next));
            });

            use_controlled_prop(value.into(), send, |next| next);
            assert!(events.with_value(Vec::is_empty));

            set_value.set(2);
            assert_eq!(events.with_value(Clone::clone), vec![2]);

            set_value.set(2);
            assert_eq!(events.with_value(Clone::clone), vec![2]);

            assert_eq!(value.get_untracked(), 2);
        });
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use leptos::{prelude::*, reactive::traits::GetUntracked};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::use_controlled_prop;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_controlled_prop_skips_initial_value_and_dispatches_on_change_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (value, set_value) = signal(1u32);
            let events: StoredValue<Vec<u32>> = StoredValue::new(Vec::new());
            let send = Callback::new(move |next: u32| {
                events.update_value(|captured| captured.push(next));
            });

            use_controlled_prop(value.into(), send, |next| next);
            assert!(events.with_value(Vec::is_empty));

            set_value.set(2);
            set_value.set(2);

            assert_eq!(events.with_value(Clone::clone), vec![2]);
            assert_eq!(value.get_untracked(), 2);
        });
    }
}
