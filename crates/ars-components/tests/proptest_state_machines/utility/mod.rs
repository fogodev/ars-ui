use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use ars_a11y::AriaRelevant;
use ars_collections::{Key, selection};
use ars_components::utility as utility_core;
use ars_core::{ConnectApi, Env, HtmlAttr, Service, WeakSend, callback};
use ars_forms::{
    form::Mode,
    validation::{BoxedAsyncValidator, Error},
};
#[cfg(feature = "i18n")]
use ars_i18n::Locale;
use ars_i18n::Orientation;
use ars_interactions::{DragItem, FileHandle};
use proptest::prelude::*;

mod action_group;
mod ars_provider;
mod as_child;
mod button;
mod client_only;
mod dismissable;
mod download_trigger;
mod drop_zone;
mod error_boundary;
mod field;
mod fieldset;
mod focus_ring;
mod focus_scope;
mod form;
mod form_submit;
mod group;
mod heading;
#[cfg(feature = "i18n")]
mod highlight;
mod keyboard;
mod landmark;
mod live_region;
mod separator;
mod swap;
mod toggle;
mod toggle_button;
mod toggle_group;
mod visually_hidden;
mod z_index_allocator;

fn arb_direction() -> impl Strategy<Value = Option<ars_core::Direction>> {
    prop_oneof![
        Just(None),
        Just(Some(ars_core::Direction::Ltr)),
        Just(Some(ars_core::Direction::Rtl)),
    ]
}

fn arb_error() -> impl Strategy<Value = Error> {
    (
        "[a-z]{1,8}".prop_map(String::from),
        "[a-zA-Z0-9 _-]{1,16}".prop_map(String::from),
    )
        .prop_map(|(code, message)| Error::custom(code, message))
}

fn arb_field_props() -> impl Strategy<Value = utility_core::field::Props> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_direction(),
    )
        .prop_map(
            |(required, disabled, readonly, invalid, dir)| utility_core::field::Props {
                id: "field".to_string(),
                required,
                disabled,
                readonly,
                invalid,
                errors: Vec::new(),
                dir,
            },
        )
}

fn arb_button_props() -> impl Strategy<Value = utility_core::button::Props> {
    (any::<bool>(), any::<bool>()).prop_map(|(disabled, loading)| {
        utility_core::button::Props::new()
            .id("button")
            .disabled(disabled)
            .loading(loading)
    })
}

fn arb_button_event() -> impl Strategy<Value = utility_core::button::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| utility_core::button::Event::Focus { is_keyboard }),
        Just(utility_core::button::Event::Blur),
        Just(utility_core::button::Event::Press),
        Just(utility_core::button::Event::Release),
        Just(utility_core::button::Event::Click),
        any::<bool>().prop_map(utility_core::button::Event::SetLoading),
        any::<bool>().prop_map(utility_core::button::Event::SetDisabled),
    ]
}

fn arb_toggle_props() -> impl Strategy<Value = utility_core::toggle::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(pressed, default_pressed, disabled)| utility_core::toggle::Props {
                id: "toggle".to_string(),
                pressed,
                default_pressed,
                disabled,
                on_change: None,
            },
        )
}

fn arb_toggle_event() -> impl Strategy<Value = utility_core::toggle::Event> {
    prop_oneof![
        Just(utility_core::toggle::Event::Toggle),
        Just(utility_core::toggle::Event::TurnOn),
        Just(utility_core::toggle::Event::TurnOff),
        any::<bool>().prop_map(|is_keyboard| utility_core::toggle::Event::Focus { is_keyboard }),
        Just(utility_core::toggle::Event::Blur),
        any::<bool>().prop_map(utility_core::toggle::Event::SetDisabled),
        prop::option::of(any::<bool>()).prop_map(utility_core::toggle::Event::SetValue),
    ]
}

fn arb_drop_zone_accept() -> impl Strategy<Value = Vec<String>> {
    prop_oneof![
        Just(Vec::new()),
        Just(vec![".png".to_string()]),
        Just(vec!["image/*".to_string()]),
        Just(vec!["image/png".to_string()]),
        Just(vec!["text/plain".to_string()]),
    ]
}

fn arb_drop_zone_props() -> impl Strategy<Value = utility_core::drop_zone::Props> {
    (
        arb_drop_zone_accept(),
        prop::option::of(0usize..=4),
        prop::option::of(0u64..=2_048),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(accept, max_files, max_file_size, disabled, read_only, invalid)| {
                utility_core::drop_zone::Props {
                    id: "drop-zone".to_string(),
                    accept,
                    max_files,
                    max_file_size,
                    disabled,
                    label: "Upload files".to_string(),
                    allowed_operations: vec![ars_interactions::DropOperation::Move],
                    name: Some("files".to_string()),
                    required: false,
                    invalid,
                    read_only,
                    activate_delay: Duration::from_millis(500),
                    reset_delay: Duration::from_millis(1_500),
                    get_drop_operation: None,
                    on_drop: None,
                    on_drop_rejected: None,
                    on_drop_enter: None,
                    on_drop_exit: None,
                    on_drop_move: None,
                    on_hover_start: None,
                    on_drop_activate: None,
                    on_hover_end: None,
                }
            },
        )
}

fn arb_drop_zone_item() -> impl Strategy<Value = DragItem> {
    prop_oneof![
        "[a-z]{0,16}".prop_map(DragItem::Text),
        ("[a-z]{1,8}", 0u64..=4_096).prop_map(|(name, size)| DragItem::File {
            name: format!("{name}.png"),
            mime_type: "image/png".to_string(),
            size,
            handle: FileHandle::opaque(),
        }),
        ("[a-z]{1,8}", 0u64..=4_096).prop_map(|(name, size)| DragItem::File {
            name: format!("{name}.txt"),
            mime_type: "text/plain".to_string(),
            size,
            handle: FileHandle::opaque(),
        }),
    ]
}

fn arb_drop_zone_data() -> impl Strategy<Value = utility_core::drop_zone::DragData> {
    (
        prop::collection::vec(arb_drop_zone_item(), 0..=4),
        prop::collection::vec(
            prop_oneof![
                Just("image/png".to_string()),
                Just("image/jpg".to_string()),
                Just("text/plain".to_string()),
                Just("application/json".to_string()),
                Just("Files".to_string()),
            ],
            0..=4,
        ),
    )
        .prop_map(|(items, types)| utility_core::drop_zone::DragData { items, types })
}

fn arb_drop_zone_event() -> impl Strategy<Value = utility_core::drop_zone::Event> {
    prop_oneof![
        arb_drop_zone_data().prop_map(utility_core::drop_zone::Event::DragEnter),
        arb_drop_zone_data().prop_map(utility_core::drop_zone::Event::DragOver),
        Just(utility_core::drop_zone::Event::DragLeave),
        arb_drop_zone_data().prop_map(utility_core::drop_zone::Event::Drop),
        Just(utility_core::drop_zone::Event::Reset),
        Just(utility_core::drop_zone::Event::AutoReset),
        Just(utility_core::drop_zone::Event::SetProps),
        Just(utility_core::drop_zone::Event::DropActivate),
        any::<bool>().prop_map(|is_keyboard| utility_core::drop_zone::Event::Focus { is_keyboard }),
        Just(utility_core::drop_zone::Event::Blur),
    ]
}

fn arb_toggle_group_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("bold")),
        Just(Key::str("italic")),
        Just(Key::str("strike")),
        Just(Key::str("code")),
    ]
}

fn arb_toggle_group_key_set() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::btree_set(arb_toggle_group_key(), 0..=4)
}

fn arb_toggle_group_mode() -> impl Strategy<Value = utility_core::toggle_group::SelectionMode> {
    prop_oneof![
        Just(utility_core::toggle_group::SelectionMode::None),
        Just(utility_core::toggle_group::SelectionMode::Single),
        Just(utility_core::toggle_group::SelectionMode::Multiple),
    ]
}

fn arb_toggle_group_props() -> impl Strategy<Value = utility_core::toggle_group::Props> {
    (
        prop::option::of(arb_toggle_group_key_set()),
        arb_toggle_group_key_set(),
        arb_toggle_group_mode(),
        any::<bool>(),
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![
            Just(ars_core::Direction::Ltr),
            Just(ars_core::Direction::Rtl)
        ],
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_toggle_group_key_set(),
    )
        .prop_map(
            |(
                value,
                default_value,
                selection_mode,
                disabled,
                orientation,
                dir,
                loop_focus,
                roving_focus,
                disallow_empty_selection,
                read_only,
                disabled_items,
            )| utility_core::toggle_group::Props {
                id: "toggle-group".to_string(),
                value,
                default_value,
                selection_mode,
                disabled,
                orientation,
                dir,
                loop_focus,
                roving_focus,
                aria_label: Some("Format".to_string()),
                aria_labelledby: None,
                disallow_empty_selection,
                name: Some("format".to_string()),
                invalid: false,
                required: false,
                form: None,
                read_only,
                disabled_items,
                on_change: None,
            },
        )
}

fn arb_toggle_group_event() -> impl Strategy<Value = utility_core::toggle_group::Event> {
    prop_oneof![
        arb_toggle_group_key().prop_map(utility_core::toggle_group::Event::SelectItem),
        arb_toggle_group_key().prop_map(utility_core::toggle_group::Event::DeselectItem),
        arb_toggle_group_key().prop_map(utility_core::toggle_group::Event::ToggleItem),
        (arb_toggle_group_key(), any::<bool>()).prop_map(|(item, is_keyboard)| {
            utility_core::toggle_group::Event::Focus { item, is_keyboard }
        }),
        Just(utility_core::toggle_group::Event::Blur),
        Just(utility_core::toggle_group::Event::FocusNext),
        Just(utility_core::toggle_group::Event::FocusPrev),
        Just(utility_core::toggle_group::Event::FocusFirst),
        Just(utility_core::toggle_group::Event::FocusLast),
        arb_toggle_group_key().prop_map(utility_core::toggle_group::Event::RegisterItem),
        arb_toggle_group_key().prop_map(utility_core::toggle_group::Event::UnregisterItem),
        Just(utility_core::toggle_group::Event::Reset),
        prop::option::of(arb_toggle_group_key_set())
            .prop_map(utility_core::toggle_group::Event::SetValue),
        Just(utility_core::toggle_group::Event::SetProps),
    ]
}

fn arb_action_group_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("copy")),
        Just(Key::str("delete")),
        Just(Key::str("archive")),
        Just(Key::str("share")),
    ]
}

fn arb_action_group_key_set() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::btree_set(arb_action_group_key(), 0..=4)
}

fn arb_action_group_selection_mode() -> impl Strategy<Value = selection::Mode> {
    prop_oneof![
        Just(selection::Mode::None),
        Just(selection::Mode::Single),
        Just(selection::Mode::Multiple),
    ]
}

fn arb_action_group_overflow_mode()
-> impl Strategy<Value = utility_core::action_group::OverflowMode> {
    prop_oneof![
        Just(utility_core::action_group::OverflowMode::Wrap),
        Just(utility_core::action_group::OverflowMode::Collapse),
        Just(utility_core::action_group::OverflowMode::Menu),
    ]
}

fn arb_action_group_label_behavior()
-> impl Strategy<Value = utility_core::action_group::ButtonLabelBehavior> {
    prop_oneof![
        Just(utility_core::action_group::ButtonLabelBehavior::Show),
        Just(utility_core::action_group::ButtonLabelBehavior::Collapse),
        Just(utility_core::action_group::ButtonLabelBehavior::Hide),
    ]
}

fn arb_action_group_variant() -> impl Strategy<Value = utility_core::action_group::Variant> {
    prop_oneof![
        Just(utility_core::action_group::Variant::Toolbar),
        Just(utility_core::action_group::Variant::Outlined),
        Just(utility_core::action_group::Variant::Flat),
    ]
}

fn arb_action_group_props() -> impl Strategy<Value = utility_core::action_group::Props> {
    (
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![
            Just(ars_core::Direction::Ltr),
            Just(ars_core::Direction::Rtl)
        ],
        arb_action_group_overflow_mode(),
        arb_action_group_variant(),
        any::<bool>(),
        arb_action_group_key_set(),
        arb_action_group_selection_mode(),
        prop::option::of(0_usize..=4),
        arb_action_group_label_behavior(),
        prop::option::of("[a-z]{1,10}".prop_map(String::from)),
        any::<bool>(),
    )
        .prop_map(
            |(
                orientation,
                dir,
                overflow_mode,
                variant,
                disabled,
                disabled_items,
                selection_mode,
                max_visible_actions,
                button_label_behavior,
                density,
                justified,
            )| utility_core::action_group::Props {
                id: "action-group".to_string(),
                orientation,
                dir,
                overflow_mode,
                variant,
                disabled,
                disabled_items,
                selection_mode,
                max_visible_actions,
                button_label_behavior,
                density,
                justified,
                aria_label: Some("Actions".to_string()),
                aria_labelledby: None,
                on_action: None,
                on_selection_change: None,
            },
        )
}

fn arb_action_group_event() -> impl Strategy<Value = utility_core::action_group::Event> {
    prop_oneof![
        arb_action_group_key().prop_map(utility_core::action_group::Event::FocusItem),
        Just(utility_core::action_group::Event::Blur),
        Just(utility_core::action_group::Event::FocusNext),
        Just(utility_core::action_group::Event::FocusPrev),
        Just(utility_core::action_group::Event::FocusFirst),
        Just(utility_core::action_group::Event::FocusLast),
        arb_action_group_key().prop_map(utility_core::action_group::Event::ActivateItem),
        arb_action_group_key().prop_map(utility_core::action_group::Event::SelectItem),
        (0_usize..=8).prop_map(utility_core::action_group::Event::OverflowChanged),
        arb_action_group_key().prop_map(utility_core::action_group::Event::RegisterItem),
        arb_action_group_key().prop_map(utility_core::action_group::Event::UnregisterItem),
        Just(utility_core::action_group::Event::SetProps),
    ]
}

fn arb_live_region_relevant() -> impl Strategy<Value = AriaRelevant> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(additions, removals, text)| {
        AriaRelevant {
            additions,
            removals,
            text,
        }
    })
}

fn arb_live_region_politeness() -> impl Strategy<Value = utility_core::live_region::AriaPoliteness>
{
    prop_oneof![
        Just(utility_core::live_region::AriaPoliteness::Off),
        Just(utility_core::live_region::AriaPoliteness::Polite),
        Just(utility_core::live_region::AriaPoliteness::Assertive),
    ]
}

fn arb_live_region_priority() -> impl Strategy<Value = utility_core::live_region::AnnouncePriority>
{
    prop_oneof![
        Just(utility_core::live_region::AnnouncePriority::Normal),
        Just(utility_core::live_region::AnnouncePriority::Urgent),
    ]
}

fn arb_live_region_props() -> impl Strategy<Value = utility_core::live_region::Props> {
    (
        arb_live_region_politeness(),
        any::<bool>(),
        arb_live_region_relevant(),
        (0_u64..=1_000).prop_map(Duration::from_millis),
    )
        .prop_map(
            |(politeness, atomic, relevant, delay)| utility_core::live_region::Props {
                id: "live-region".to_string(),
                politeness,
                atomic,
                relevant,
                delay,
            },
        )
}

fn arb_live_region_event() -> impl Strategy<Value = utility_core::live_region::Event> {
    prop_oneof![
        (
            "[a-zA-Z0-9 _-]{1,24}".prop_map(String::from),
            arb_live_region_priority(),
        )
            .prop_map(
                |(message, priority)| utility_core::live_region::Event::Announce {
                    message,
                    priority
                }
            ),
        Just(utility_core::live_region::Event::Clear),
        Just(utility_core::live_region::Event::Rendered),
        Just(utility_core::live_region::Event::SetProps),
    ]
}

fn arb_focus_scope_props() -> impl Strategy<Value = utility_core::focus_scope::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(trapped, contain, auto_focus, restore_focus)| utility_core::focus_scope::Props {
            id: "focus-scope".to_string(),
            trapped,
            contain,
            auto_focus,
            restore_focus,
        },
    )
}

fn arb_optional_focus_target() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[a-zA-Z0-9-]{1,16}".prop_map(String::from))
}

fn arb_focus_scope_event() -> impl Strategy<Value = utility_core::focus_scope::Event> {
    prop_oneof![
        (any::<bool>(), arb_optional_focus_target()).prop_map(|(trapped, saved_focus_id)| {
            utility_core::focus_scope::Event::Activate {
                trapped,
                saved_focus_id,
            }
        }),
        any::<bool>().prop_map(
            |restore_focus| utility_core::focus_scope::Event::Deactivate { restore_focus }
        ),
        Just(utility_core::focus_scope::Event::TrapFocus),
        Just(utility_core::focus_scope::Event::ReleaseTrap),
        Just(utility_core::focus_scope::Event::RestoreFocus),
        Just(utility_core::focus_scope::Event::FocusFirst),
        Just(utility_core::focus_scope::Event::FocusLast),
    ]
}

fn arb_swap_props() -> impl Strategy<Value = utility_core::swap::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(checked, default_checked, disabled)| utility_core::swap::Props {
                id: "swap".to_string(),
                checked,
                default_checked,
                disabled,
                label: None,
                animation: utility_core::swap::Animation::None,
                on_change: None,
            },
        )
}

fn arb_swap_event() -> impl Strategy<Value = utility_core::swap::Event> {
    prop_oneof![
        Just(utility_core::swap::Event::Toggle),
        Just(utility_core::swap::Event::SetOn),
        Just(utility_core::swap::Event::SetOff),
        any::<bool>().prop_map(utility_core::swap::Event::SetDisabled),
        prop::option::of(any::<bool>()).prop_map(utility_core::swap::Event::SetValue),
        any::<bool>().prop_map(|is_keyboard| utility_core::swap::Event::Focus { is_keyboard }),
        Just(utility_core::swap::Event::Blur),
    ]
}

fn arb_field_event() -> impl Strategy<Value = utility_core::field::Event> {
    prop_oneof![
        prop::collection::vec(arb_error(), 0..4).prop_map(utility_core::field::Event::SetErrors),
        Just(utility_core::field::Event::ClearErrors),
        any::<bool>().prop_map(utility_core::field::Event::SetHasDescription),
        any::<bool>().prop_map(utility_core::field::Event::SetDisabled),
        any::<bool>().prop_map(utility_core::field::Event::SetInvalid),
        any::<bool>().prop_map(utility_core::field::Event::SetReadonly),
        any::<bool>().prop_map(utility_core::field::Event::SetRequired),
        arb_direction().prop_map(utility_core::field::Event::SetDir),
        any::<bool>().prop_map(utility_core::field::Event::SetValidating),
    ]
}

fn arb_fieldset_props() -> impl Strategy<Value = utility_core::fieldset::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), arb_direction()).prop_map(
        |(disabled, invalid, readonly, dir)| utility_core::fieldset::Props {
            id: "fieldset".to_string(),
            disabled,
            invalid,
            readonly,
            dir,
        },
    )
}

fn arb_fieldset_event() -> impl Strategy<Value = utility_core::fieldset::Event> {
    prop_oneof![
        prop::collection::vec(arb_error(), 0..4).prop_map(utility_core::fieldset::Event::SetErrors),
        Just(utility_core::fieldset::Event::ClearErrors),
        any::<bool>().prop_map(utility_core::fieldset::Event::SetDisabled),
        any::<bool>().prop_map(utility_core::fieldset::Event::SetInvalid),
        any::<bool>().prop_map(utility_core::fieldset::Event::SetReadonly),
        arb_direction().prop_map(utility_core::fieldset::Event::SetDir),
        any::<bool>().prop_map(utility_core::fieldset::Event::SetHasDescription),
    ]
}

fn arb_validation_behavior() -> impl Strategy<Value = utility_core::form::ValidationBehavior> {
    prop_oneof![
        Just(utility_core::form::ValidationBehavior::Aria),
        Just(utility_core::form::ValidationBehavior::Native),
    ]
}

fn arb_error_map() -> impl Strategy<Value = BTreeMap<String, Vec<Error>>> {
    prop::collection::btree_map(
        "[a-z]{1,8}".prop_map(String::from),
        prop::collection::vec(arb_error(), 1..4),
        0..4,
    )
}

fn arb_form_props() -> impl Strategy<Value = utility_core::form::Props> {
    (
        arb_validation_behavior(),
        arb_error_map(),
        prop::option::of("[a-zA-Z0-9:/._?#=-]{1,24}".prop_map(String::from)),
        prop::option::of("[a-z-]{1,12}".prop_map(String::from)),
    )
        .prop_map(|(validation_behavior, validation_errors, action, role)| {
            utility_core::form::Props {
                id: "form".to_string(),
                validation_behavior,
                validation_errors,
                action,
                role,
            }
        })
}

fn arb_form_event() -> impl Strategy<Value = utility_core::form::Event> {
    prop_oneof![
        Just(utility_core::form::Event::Submit),
        any::<bool>().prop_map(|success| utility_core::form::Event::SubmitComplete { success }),
        Just(utility_core::form::Event::Reset),
        arb_error_map().prop_map(utility_core::form::Event::SetValidationErrors),
        Just(utility_core::form::Event::ClearValidationErrors),
        arb_validation_behavior().prop_map(utility_core::form::Event::SetValidationBehavior),
        prop::option::of("[a-zA-Z0-9 _-]{1,16}".prop_map(String::from))
            .prop_map(utility_core::form::Event::SetStatusMessage),
    ]
}

fn arb_mode() -> impl Strategy<Value = Mode> {
    prop_oneof![
        Just(Mode::on_submit()),
        Just(Mode::on_blur_revalidate()),
        Just(Mode::on_change()),
    ]
}

fn arb_server_errors() -> impl Strategy<Value = BTreeMap<String, Vec<String>>> {
    prop::collection::btree_map(
        "[a-z]{1,8}".prop_map(String::from),
        prop::collection::vec("[a-zA-Z0-9 _-]{1,16}".prop_map(String::from), 1..4),
        0..4,
    )
}

fn arb_form_submit_event() -> impl Strategy<Value = utility_core::form_submit::Event> {
    prop_oneof![
        Just(utility_core::form_submit::Event::Submit),
        Just(utility_core::form_submit::Event::ValidationPassed),
        Just(utility_core::form_submit::Event::ValidationFailed),
        Just(utility_core::form_submit::Event::SubmitComplete),
        "[a-zA-Z0-9 _-]{1,16}"
            .prop_map(String::from)
            .prop_map(utility_core::form_submit::Event::SubmitError),
        Just(utility_core::form_submit::Event::Reset),
        arb_server_errors().prop_map(utility_core::form_submit::Event::SetServerErrors),
        arb_mode().prop_map(utility_core::form_submit::Event::SetMode),
    ]
}

fn form_submit_props(initial_mode: Mode) -> utility_core::form_submit::Props {
    utility_core::form_submit::Props {
        id: "test-form".into(),
        validation_mode: initial_mode,
        spawn_async_validation: callback(
            |_: (
                Vec<(String, BoxedAsyncValidator)>,
                WeakSend<utility_core::form_submit::Event>,
            )|
             -> Box<dyn FnOnce()> { Box::new(|| {}) },
        ),
        schedule_microtask: callback(|_: Box<dyn FnOnce()>| {}),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Stateless utility components — invariant tests
//
// VisuallyHidden and Separator have no state machine; their `Api` is a
// pure function of `Props` and the only output surface is the `AttrMap`
// returned by `Api::root_attrs()` / `Api::part_attrs(Part::Root)`.
// Proptests here pin the invariants that distinguish output-affecting
// branches and guard against future refactors silently conflating them.
// ─────────────────────────────────────────────────────────────────────

fn arb_short_id() -> impl Strategy<Value = String> {
    // Permit empty, ASCII alnum/punctuation, and unicode-ish ids.
    prop_oneof![
        Just(String::new()),
        "[a-zA-Z0-9_-]{1,32}".prop_map(String::from),
        "[a-zA-Z0-9_-]{0,8} [a-zA-Z0-9_-]{0,8}".prop_map(String::from),
    ]
}

fn arb_orientation() -> impl Strategy<Value = Orientation> {
    prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)]
}

fn arb_visually_hidden_props() -> impl Strategy<Value = utility_core::visually_hidden::Props> {
    (arb_short_id(), any::<bool>(), any::<bool>()).prop_map(|(id, as_child, is_focusable)| {
        utility_core::visually_hidden::Props {
            id,
            as_child,
            is_focusable,
        }
    })
}

fn arb_separator_props() -> impl Strategy<Value = utility_core::separator::Props> {
    (arb_short_id(), arb_orientation(), any::<bool>()).prop_map(|(id, orientation, decorative)| {
        utility_core::separator::Props {
            id,
            orientation,
            decorative,
        }
    })
}

fn arb_optional_short_filename() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-z]{1,12}\\.[a-z]{2,4}".prop_map(Some),]
}

fn arb_optional_mime_type() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-z]{1,12}/[a-z]{1,12}".prop_map(Some),]
}

fn arb_download_trigger_props() -> impl Strategy<Value = utility_core::download_trigger::Props> {
    (
        arb_short_id(),
        prop_oneof![
            "[a-z]{1,16}".prop_map(|segment| format!("/{segment}")),
            Just(String::from("https://example.com/file")),
        ],
        arb_optional_short_filename(),
        arb_optional_mime_type(),
        any::<bool>(),
        prop_oneof![Just(None), Just(Some(String::from("https://example.com"))),],
    )
        .prop_map(
            |(id, href, filename, mime_type, disabled, document_origin)| {
                utility_core::download_trigger::Props {
                    id,
                    href,
                    filename,
                    mime_type,
                    disabled,
                    document_origin,
                }
            },
        )
}

fn arb_download_trigger_cross_origin_props()
-> impl Strategy<Value = utility_core::download_trigger::Props> {
    (arb_short_id(), arb_optional_short_filename(), any::<bool>()).prop_map(
        |(id, filename, disabled)| utility_core::download_trigger::Props {
            id,
            href: String::from("https://cdn.example/asset.bin"),
            filename,
            mime_type: None,
            disabled,
            document_origin: Some(String::from("https://app.example")),
        },
    )
}

// ── Highlight (utility::highlight, gated on the `i18n` feature) ────

#[cfg(feature = "i18n")]
fn arb_match_strategy() -> impl Strategy<Value = utility_core::highlight::MatchStrategy> {
    prop_oneof![
        Just(utility_core::highlight::MatchStrategy::Contains),
        Just(utility_core::highlight::MatchStrategy::StartsWith),
        Just(utility_core::highlight::MatchStrategy::Fuzzy),
    ]
}

#[cfg(feature = "i18n")]
fn arb_highlight_text() -> impl Strategy<Value = String> {
    // Mix of ASCII, multi-byte UTF-8 (ß, İ, é), and whitespace to
    // exercise byte-boundary and case-folding paths together.
    "[a-zA-Z0-9 _\u{00DF}\u{0130}\u{00E9}]{0,32}".prop_map(String::from)
}

#[cfg(feature = "i18n")]
fn arb_highlight_query() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        "[a-zA-Z0-9 \u{00DF}\u{0130}]{0,8}".prop_map(String::from),
        0..4,
    )
}

#[cfg(feature = "i18n")]
fn arb_highlight_props() -> impl Strategy<Value = utility_core::highlight::Props> {
    (
        arb_highlight_query(),
        arb_highlight_text(),
        any::<bool>(),
        arb_match_strategy(),
    )
        .prop_map(|(query, text, ignore_case, match_strategy)| {
            utility_core::highlight::Props::new()
                .query(query)
                .text(text)
                .ignore_case(ignore_case)
                .match_strategy(match_strategy)
        })
}

#[cfg(feature = "i18n")]
fn arb_highlight_locale() -> impl Strategy<Value = Locale> {
    // Mix of locale families that exercise different case-folding paths:
    // - en-US: default Unicode fold.
    // - tr / az: Turkic dotted/dotless-I fold (CaseMapper::fold_turkic_string).
    // - de: German eszett expansion `ß → ss`.
    // - el: Greek final-sigma collapse `Σ/σ/ς → σ`.
    // - lt: Lithuanian combining-dot handling.
    // - hy: Armenian — exercises a script with case but no special tailoring.
    // - ar: a script with no case at all (the fold is the identity).
    prop_oneof![
        Just(Locale::parse("en-US").expect("en-US must parse")),
        Just(Locale::parse("tr").expect("tr must parse")),
        Just(Locale::parse("az").expect("az must parse")),
        Just(Locale::parse("de").expect("de must parse")),
        Just(Locale::parse("el").expect("el must parse")),
        Just(Locale::parse("lt").expect("lt must parse")),
        Just(Locale::parse("hy").expect("hy must parse")),
        Just(Locale::parse("ar").expect("ar must parse")),
    ]
}
