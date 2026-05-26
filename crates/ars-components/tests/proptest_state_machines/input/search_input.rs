use ars_components::input::search_input;
use ars_core::{Env, HtmlAttr, Service};
use proptest::prelude::*;

use super::arb_short_text;

fn arb_search_input_props() -> impl Strategy<Value = search_input::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(1_u64..=500),
    )
        .prop_map(
            |(value, default_value, disabled, readonly, invalid, required, debounce_ms)| {
                search_input::Props {
                    id: "search-input".to_string(),
                    value,
                    default_value,
                    disabled,
                    readonly,
                    invalid,
                    required,
                    placeholder: Some("Search...".to_string()),
                    name: Some("q".to_string()),
                    form: Some("form".to_string()),
                    debounce: debounce_ms.map(core::time::Duration::from_millis),
                }
            },
        )
}

fn arb_search_input_event() -> impl Strategy<Value = search_input::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| search_input::Event::Focus { is_keyboard }),
        Just(search_input::Event::Blur),
        arb_short_text().prop_map(search_input::Event::Change),
        Just(search_input::Event::Clear),
        Just(search_input::Event::Submit),
        any::<bool>().prop_map(search_input::Event::SetSearching),
        Just(search_input::Event::DebounceExpired),
        Just(search_input::Event::CancelDebounce),
        Just(search_input::Event::CompositionStart),
        Just(search_input::Event::CompositionEnd),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_search_input_event_sequences_preserve_invariants(
        props in arb_search_input_props(),
        events in prop::collection::vec(arb_search_input_event(), 0..128),
    ) {
        let mut service = Service::<search_input::Machine>::new(
            props,
            &Env::default(),
            &search_input::Messages::default(),
        );

        for event in events {
            let result = service.send(event);

            let ctx = service.context();
            let state = *service.state();

            // Loading flag aligns with Searching state on entry, but Idle/Focused
            // can be returned to while loading=false. Just check the converse:
            // when state == Searching, loading must be true (set by Submit/SetSearching).
            if state == search_input::State::Searching {
                prop_assert!(ctx.loading);
            }

            // is_composing never co-occurs with a fresh debounce effect
            if ctx.is_composing {
                for effect in &result.pending_effects {
                    prop_assert!(effect.name != search_input::Effect::SearchDebounce);
                }
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Type), Some("search"));
        }
    }
}
