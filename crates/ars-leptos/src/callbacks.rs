//! Optional event-callback helpers for Leptos adapter components.

use leptos::prelude::{Callable, Callback};

/// Emits a value through an optional Leptos callback.
pub fn emit<T>(callback: Option<&Callback<T>>, value: T) {
    if let Some(callback) = callback {
        callback.run(value);
    }
}

/// Maps a value before emitting it through an optional Leptos callback.
pub fn emit_map<T, U>(callback: Option<&Callback<U>>, value: T, f: impl Fn(T) -> U) {
    if let Some(callback) = callback {
        callback.run(f(value));
    }
}

#[cfg(test)]
mod tests {
    use leptos::{prelude::*, reactive::owner::Owner};

    use super::{emit, emit_map};

    #[test]
    fn emit_dispatches_when_callback_is_present() {
        let owner = Owner::new();

        owner.with(|| {
            let captured = StoredValue::new(Vec::<u32>::new());
            let callback = Callback::new(move |value| {
                captured.update_value(|values| values.push(value));
            });

            emit(Some(&callback), 7);

            assert_eq!(captured.with_value(Clone::clone), vec![7]);
        });
    }

    #[test]
    fn emit_is_noop_when_callback_is_absent() {
        emit::<u32>(None, 7);
    }

    #[test]
    fn emit_map_transforms_before_dispatch() {
        let owner = Owner::new();

        owner.with(|| {
            let captured = StoredValue::new(Vec::<String>::new());
            let callback = Callback::new(move |value| {
                captured.update_value(|values| values.push(value));
            });

            emit_map(Some(&callback), 7, |value| format!("value-{value}"));

            assert_eq!(
                captured.with_value(Clone::clone),
                vec![String::from("value-7")]
            );
        });
    }

    #[test]
    fn emit_map_does_not_transform_when_callback_is_absent() {
        let transformed = std::cell::Cell::new(false);

        emit_map::<_, u32>(None, 7, |_| {
            transformed.set(true);
            8
        });

        assert!(!transformed.get());
    }
}
