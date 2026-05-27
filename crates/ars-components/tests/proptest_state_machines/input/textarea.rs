use ars_components::input::textarea;
use ars_core::{AriaAttr, Direction, EffectMetadata, Env, HtmlAttr, InputMode, Service};
use proptest::prelude::*;

use super::arb_short_text;

fn arb_textarea_resize() -> impl Strategy<Value = textarea::ResizeMode> {
    prop_oneof![
        Just(textarea::ResizeMode::None),
        Just(textarea::ResizeMode::Both),
        Just(textarea::ResizeMode::Horizontal),
        Just(textarea::ResizeMode::Vertical),
    ]
}

fn arb_textarea_props() -> impl Strategy<Value = textarea::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        1_u32..12,
        prop::option::of(1_u32..80),
        arb_textarea_resize(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                rows,
                cols,
                resize,
                auto_resize,
                input_mode,
            )| textarea::Props {
                id: "textarea".to_string(),
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                placeholder: Some("placeholder".to_string()),
                max_length: Some(256),
                min_length: Some(1),
                name: Some("bio".to_string()),
                form: Some("form".to_string()),
                autocomplete: Some("off".to_string()),
                rows,
                cols,
                resize,
                auto_resize,
                max_height: Some("240px".to_string()),
                max_rows: Some(8),
                dir: Direction::Ltr,
                input_mode: input_mode.then_some(InputMode::Text),
                on_value_change: None,
            },
        )
}

fn arb_textarea_event() -> impl Strategy<Value = textarea::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| textarea::Event::Focus { is_keyboard }),
        Just(textarea::Event::Blur),
        arb_short_text().prop_map(textarea::Event::Change),
        Just(textarea::Event::Clear),
        any::<bool>().prop_map(textarea::Event::SetInvalid),
        Just(textarea::Event::CompositionStart),
        arb_short_text().prop_map(textarea::Event::CompositionEnd),
        prop::option::of(arb_short_text()).prop_map(textarea::Event::SetValue),
        Just(textarea::Event::SetProps),
        any::<bool>().prop_map(textarea::Event::SetHasDescription),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_textarea_event_sequences_preserve_invariants(
        props in arb_textarea_props(),
        events in prop::collection::vec(arb_textarea_event(), 0..128),
    ) {
        let mut service = Service::<textarea::Machine>::new(
            props,
            &Env::default(),
            &textarea::Messages,
        );

        for event in events {
            let result = service.send(event);

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "textarea");
            prop_assert_eq!(ctx.focused, service.state() == &textarea::State::Focused);

            if !ctx.focused {
                prop_assert!(!ctx.focus_visible);
            }

            let attrs = service.connect(&|_| {}).textarea_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Value), Some(ctx.value.get().as_str()));
            prop_assert_eq!(attrs.contains(&HtmlAttr::Disabled), ctx.disabled);
            prop_assert_eq!(attrs.contains(&HtmlAttr::ReadOnly), ctx.readonly);
            prop_assert_eq!(attrs.contains(&HtmlAttr::Required), ctx.required);

            let rows = ctx.rows.to_string();

            prop_assert_eq!(attrs.get(&HtmlAttr::Rows), Some(rows.as_str()));

            if ctx.invalid {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
            } else {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), None);
            }

            let described_by = match (ctx.has_description, ctx.invalid) {
                (false, false) => None,
                (true, false) => Some("textarea-description"),
                (false, true) => Some("textarea-error-message"),
                (true, true) => Some("textarea-description textarea-error-message"),
            };

            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
                described_by,
            );

            for effect in &result.pending_effects {
                if effect.name == textarea::Effect::AutoResize {
                    prop_assert_eq!(
                        effect.metadata.as_ref(),
                        Some(&EffectMetadata::ResizeToContent(ars_core::ResizeToContentEffect {
                            element_id: "textarea-textarea".to_string(),
                            max_height: ctx.max_height.clone(),
                            max_rows: ctx.max_rows,
                        })),
                    );
                }
            }
        }
    }
}
