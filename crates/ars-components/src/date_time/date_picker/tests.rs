//! Unit and snapshot tests for the `DatePicker` component.
//!
//! Test names that begin with `snapshot_` use `insta::assert_snapshot!` and
//! commit golden output under `snapshots/`. Every other test is a pure
//! state-machine or connect-API assertion that does not depend on `.snap`
//! files.

use alloc::{format, string::String, sync::Arc, vec, vec::Vec};
use core::cell::RefCell;

use ars_core::{AriaAttr, AttrMap, Callback, ComponentPart, Env, HtmlAttr, SendResult, Service};
use ars_i18n::{
    CalendarDate, Locale, StubIntlBackend,
    locales::{de_de, en_gb, en_us, ja_jp},
};
use ars_interactions::{KeyboardEventData, KeyboardKey};
use insta::assert_snapshot;

use super::{calendar, *};

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn props() -> Props {
    Props {
        id: String::from("date-picker"),
        label: String::from("Date"),
        ..Props::default()
    }
}

fn env(locale: Locale) -> Env {
    Env::new(locale, Arc::new(StubIntlBackend))
}

fn service() -> Service<Machine> {
    Service::<Machine>::new(props(), &env(en_us()), &Messages::default())
}

fn service_with(props: Props, locale: Locale) -> Service<Machine> {
    Service::<Machine>::new(props, &env(locale), &Messages::default())
}

fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

fn attr(attrs: &AttrMap, key: HtmlAttr) -> Option<String> {
    attrs.get(&key).map(ToString::to_string)
}

fn effects(result: SendResult<Machine>) -> Vec<Effect> {
    result
        .pending_effects
        .into_iter()
        .map(|effect| effect.name)
        .collect()
}

fn initial_effect_names(service: &mut Service<Machine>) -> Vec<Effect> {
    service
        .take_initial_effects()
        .iter()
        .map(|effect| effect.name)
        .collect()
}

fn keyboard(key: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key,
        character: None,
        code: String::new(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

// ────────────────────────────────────────────────────────────────────
// Initial state
// ────────────────────────────────────────────────────────────────────

#[test]
fn initial_state_is_closed() {
    let svc = service();

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn initial_value_defaults_to_none() {
    let svc = service();

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().input_text, "");
}

#[test]
fn default_value_seeds_input_text() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    assert_eq!(svc.context().input_text, "03/15/2024");
    assert_eq!(*svc.context().value.get(), Some(date(2024, 3, 15)));
}

#[test]
fn controlled_open_true_starts_open() {
    let svc = service_with(
        Props {
            open: Some(true),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn default_open_starts_open() {
    let svc = service_with(
        Props {
            default_open: true,
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn controlled_open_emits_initial_focus_calendar_only() {
    let mut svc = service_with(
        Props {
            open: Some(true),
            ..props()
        },
        en_us(),
    );

    // Boot-open focuses the calendar but does NOT fire `OpenChange` — the
    // initial open state is the parent's configuration, not a user interaction.
    assert_eq!(initial_effect_names(&mut svc), vec![Effect::FocusCalendar]);
}

#[test]
fn closed_picker_emits_no_initial_effects() {
    let mut svc = service();

    assert!(initial_effect_names(&mut svc).is_empty());
}

// ────────────────────────────────────────────────────────────────────
// Open / close lifecycle
// ────────────────────────────────────────────────────────────────────

#[test]
fn toggle_opens_popover() {
    let mut svc = service();

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn toggle_twice_closes_popover() {
    let mut svc = service();

    drop(svc.send(Event::Toggle));
    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn open_emits_focus_calendar_effect() {
    let mut svc = service();

    assert_eq!(
        effects(svc.send(Event::Open)),
        vec![Effect::OpenChange, Effect::FocusCalendar],
    );
}

#[test]
fn open_when_already_open_is_noop() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let result = svc.send(Event::Open);

    assert!(!result.state_changed);
}

#[test]
fn close_emits_restore_focus_to_trigger_effect() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(
        effects(svc.send(Event::Close)),
        vec![Effect::OpenChange, Effect::RestoreFocusToTrigger],
    );
    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn close_when_already_closed_is_noop() {
    let mut svc = service();

    let result = svc.send(Event::Close);

    assert!(!result.state_changed);
}

// ────────────────────────────────────────────────────────────────────
// Keyboard
// ────────────────────────────────────────────────────────────────────

#[test]
fn escape_closes_open_popover() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    }));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn escape_emits_restore_focus_to_trigger() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(
        effects(svc.send(Event::KeyDown {
            key: KeyboardKey::Escape,
        })),
        vec![Effect::OpenChange, Effect::RestoreFocusToTrigger],
    );
}

#[test]
fn arrow_down_opens_closed_popover() {
    let mut svc = service();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowDown,
    }));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn escape_when_closed_is_noop() {
    let mut svc = service();

    let result = svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    });

    assert!(!result.state_changed);
}

// ────────────────────────────────────────────────────────────────────
// Date selection
// ────────────────────────────────────────────────────────────────────

#[test]
fn select_date_updates_value_and_input_text() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 3, 15)));
    assert_eq!(svc.context().input_text, "03/15/2024");
    assert_eq!(svc.context().parsed_date, Some(date(2024, 3, 15)));
}

#[test]
fn select_date_closes_popover_by_default() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn select_date_emits_value_change_open_change_and_restore_focus() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(
        effects(svc.send(Event::SelectDate {
            date: date(2024, 3, 15),
        })),
        vec![
            Effect::ValueChange,
            Effect::OpenChange,
            Effect::RestoreFocusToInput,
        ],
    );
}

#[test]
fn select_date_stays_open_when_close_on_select_false() {
    let mut svc = service_with(
        Props {
            close_on_select: false,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));

    let result = svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    });

    assert_eq!(*svc.state(), State::Open);
    // Value still changed (so ValueChange fires), but open state did not.
    assert_eq!(effects(result), vec![Effect::ValueChange]);
}

// ────────────────────────────────────────────────────────────────────
// Input text parsing
// ────────────────────────────────────────────────────────────────────

#[test]
fn input_change_parses_valid_date() {
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("06/20/2024"),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 20)));
}

#[test]
fn input_change_ignores_invalid_text() {
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("not-a-date"),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn input_change_empty_clears_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::new(),
    }));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().parsed_date, None);
}

#[test]
fn input_change_rejects_date_below_min() {
    let mut svc = service_with(
        Props {
            min: Some(date(2024, 1, 1)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("12/31/2023"),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn input_change_rejects_date_above_max() {
    let mut svc = service_with(
        Props {
            max: Some(date(2024, 12, 31)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("01/01/2025"),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn input_change_accepts_date_within_range() {
    let mut svc = service_with(
        Props {
            min: Some(date(2024, 1, 1)),
            max: Some(date(2024, 12, 31)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("06/15/2024"),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 15)));
}

#[test]
fn input_change_parses_day_month_year_format() {
    let mut svc = service_with(props(), en_gb());

    drop(svc.send(Event::InputChange {
        value: String::from("20/06/2024"),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 20)));
}

#[test]
fn input_change_parses_year_month_day_format() {
    let mut svc = service_with(props(), ja_jp());

    drop(svc.send(Event::InputChange {
        value: String::from("2024/06/20"),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 20)));
}

#[test]
fn default_format_for_german_is_day_month_year_with_dot() {
    // German uses day-first order with a `.` separator (consistent with
    // `date_field`'s locale heuristic).
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        de_de(),
    );

    assert_eq!(svc.context().input_text, "15.03.2024");
}

#[test]
fn typing_updates_calendar_props_value() {
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("03/15/2024"),
    }));

    let api = svc.connect(&|_| {});

    assert_eq!(api.calendar_props().value, Some(Some(date(2024, 3, 15))));
}

// ────────────────────────────────────────────────────────────────────
// Focus management
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_in_opens_when_open_on_click() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn focus_in_does_not_open_when_open_on_click_disabled() {
    let mut svc = service_with(
        Props {
            open_on_click: false,
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::FocusIn);

    assert_eq!(*svc.state(), State::Closed);
    assert!(!result.state_changed);
}

#[test]
fn focus_out_closes_open_popover() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::FocusOut));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn focus_out_emits_open_change_but_no_focus_effect() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    // Closing on focus-out notifies the consumer of the open change but does
    // not move focus (focus has already left the component).
    assert_eq!(effects(svc.send(Event::FocusOut)), vec![Effect::OpenChange],);
}

#[test]
fn focus_out_when_closed_is_noop() {
    let mut svc = service();

    let result = svc.send(Event::FocusOut);

    assert!(!result.state_changed);
}

// ────────────────────────────────────────────────────────────────────
// Disabled / read-only guards
// ────────────────────────────────────────────────────────────────────

#[test]
fn disabled_ignores_open() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn disabled_ignores_select_date() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn readonly_blocks_open() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn readonly_blocks_input_change() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("06/20/2024"),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

// ────────────────────────────────────────────────────────────────────
// Controlled-prop synchronization
// ────────────────────────────────────────────────────────────────────

#[test]
fn sync_props_applies_controlled_value() {
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    drop(svc.set_props(Props {
        value: Some(Some(date(2024, 3, 15))),
        ..props()
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 3, 15)));
    assert_eq!(svc.context().input_text, "03/15/2024");
}

#[test]
fn sync_props_controlled_open_opens_and_focuses() {
    let mut svc = service_with(
        Props {
            open: Some(false),
            ..props()
        },
        en_us(),
    );

    let result = svc.set_props(Props {
        open: Some(true),
        ..props()
    });

    assert_eq!(*svc.state(), State::Open);
    // A parent-driven (controlled) open change does not re-fire `OpenChange`;
    // focus still follows the open that lands at sync time.
    assert_eq!(effects(result), vec![Effect::FocusCalendar]);
}

#[test]
fn sync_props_controlled_open_closes_and_restores_focus() {
    let mut svc = service_with(
        Props {
            open: Some(true),
            ..props()
        },
        en_us(),
    );

    let result = svc.set_props(Props {
        open: Some(false),
        ..props()
    });

    assert_eq!(*svc.state(), State::Closed);
    // Parent-driven close: no `OpenChange`, focus returns to the trigger.
    assert_eq!(effects(result), vec![Effect::RestoreFocusToTrigger]);
}

#[test]
fn sync_props_clears_disabled_so_open_lands() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.set_props(props()));
    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn sync_props_open_controlled_to_uncontrolled_keeps_state() {
    let mut svc = service_with(
        Props {
            open: Some(true),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.state(), State::Open);

    // Dropping to uncontrolled open must preserve the open state (State is the
    // single source of truth; there is no parallel bindable to diverge).
    drop(svc.set_props(props()));

    assert_eq!(*svc.state(), State::Open);
}

// ────────────────────────────────────────────────────────────────────
// Connect API — ARIA / data attributes
// ────────────────────────────────────────────────────────────────────

#[test]
fn input_has_haspopup_dialog() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::HasPopup)).as_deref(),
        Some("dialog"),
    );
}

#[test]
fn trigger_has_haspopup_dialog() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Aria(AriaAttr::HasPopup)).as_deref(),
        Some("dialog"),
    );
}

#[test]
fn input_aria_expanded_reflects_closed_state() {
    let svc = service();
    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Expanded)).as_deref(),
        Some("false"),
    );
}

#[test]
fn trigger_aria_expanded_reflects_open_state() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Aria(AriaAttr::Expanded)).as_deref(),
        Some("true"),
    );
}

#[test]
fn input_aria_controls_points_to_content() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Controls)).as_deref(),
        Some("date-picker-content"),
    );
}

#[test]
fn label_for_points_to_input() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.label_attrs(), HtmlAttr::For).as_deref(),
        Some("date-picker-input"),
    );
}

#[test]
fn clear_trigger_hidden_when_no_value() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Hidden).as_deref(),
        Some("true"),
    );
}

#[test]
fn clear_trigger_visible_with_value() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert!(attr(&api.clear_trigger_attrs(), HtmlAttr::Hidden).is_none());
}

#[test]
fn positioner_hidden_when_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.positioner_attrs(), HtmlAttr::Hidden).as_deref(),
        Some("true"),
    );
}

#[test]
fn positioner_visible_when_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert!(attr(&api.positioner_attrs(), HtmlAttr::Hidden).is_none());
}

#[test]
fn content_has_dialog_role() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.content_attrs(), HtmlAttr::Role).as_deref(),
        Some("dialog"),
    );
}

#[test]
fn input_describedby_chains_description_and_error() {
    let svc = service_with(
        Props {
            description: Some(String::from("Pick a date")),
            invalid: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::DescribedBy)).as_deref(),
        Some("date-picker-description date-picker-error-message"),
    );
}

#[test]
fn input_marks_invalid() {
    let svc = service_with(
        Props {
            invalid: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Invalid)).as_deref(),
        Some("true"),
    );
}

#[test]
fn input_announces_selected_date() {
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("03/15/2024"),
    }));

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Description)).as_deref(),
        Some("Selected date: 03/15/2024"),
    );
}

#[test]
fn hidden_input_carries_iso_value() {
    let mut svc = service_with(
        Props {
            name: Some(String::from("date")),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 12, 25),
    }));

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("2024-12-25"),
    );
}

#[test]
fn hidden_input_empty_when_no_value() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some(""),
    );
}

#[test]
fn calendar_props_forward_constraints() {
    let svc = service_with(
        Props {
            min: Some(date(2024, 1, 1)),
            max: Some(date(2024, 12, 31)),
            visible_months: 2,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let calendar_props = api.calendar_props();

    assert_eq!(calendar_props.min, Some(date(2024, 1, 1)));
    assert_eq!(calendar_props.max, Some(date(2024, 12, 31)));
    assert_eq!(calendar_props.visible_months, 2);
    assert_eq!(calendar_props.id, "date-picker-calendar");
}

#[test]
fn part_attrs_dispatch_matches_getters() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        snapshot_attrs(&api.part_attrs(Part::Input)),
        snapshot_attrs(&api.input_attrs()),
    );
}

#[test]
fn part_attrs_dispatches_every_part() {
    let svc = service();

    let api = svc.connect(&|_| {});

    for part in Part::all() {
        let attrs = api.part_attrs(part.clone());

        assert_eq!(
            attr(&attrs, HtmlAttr::Data("ars-scope")).as_deref(),
            Some("date-picker"),
            "{part:?} attrs should carry the date-picker scope",
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// Connect API — typed handlers
// ────────────────────────────────────────────────────────────────────

#[test]
fn on_trigger_click_sends_toggle() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_trigger_click();

    assert_eq!(sent.borrow().as_slice(), &[Event::Toggle]);
}

#[test]
fn on_clear_trigger_click_sends_empty_input_change() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_clear_trigger_click();

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::InputChange {
            value: String::new(),
        }],
    );
}

#[test]
fn on_content_keydown_escape_sends_close() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_content_keydown(&keyboard(KeyboardKey::Escape));

    assert_eq!(sent.borrow().as_slice(), &[Event::Close]);
}

#[test]
fn on_trigger_keydown_arrow_down_sends_open() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    let handled = api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowDown));

    assert!(handled);
    assert_eq!(sent.borrow().as_slice(), &[Event::Open]);
}

#[test]
fn on_focusout_only_sends_when_leaving_component() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_focusout(false);

    assert!(sent.borrow().is_empty());

    api.on_focusout(true);

    assert_eq!(sent.borrow().as_slice(), &[Event::FocusOut]);
}

#[test]
fn on_input_change_sends_input_change_event() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_input_change("06/20/2024");

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::InputChange {
            value: String::from("06/20/2024"),
        }],
    );
}

#[test]
fn on_input_keydown_sends_keydown_event() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_input_keydown(KeyboardKey::ArrowDown);

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }],
    );
}

#[test]
fn on_focusin_sends_focus_in_event() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_focusin();

    assert_eq!(sent.borrow().as_slice(), &[Event::FocusIn]);
}

#[test]
fn on_trigger_keydown_enter_sends_toggle_and_ignores_other_keys() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    // Enter is handled (returns true) so the adapter can suppress the native
    // activation click and avoid a double toggle.
    let enter_handled = api.on_trigger_keydown(&keyboard(KeyboardKey::Enter));
    // A key with no trigger binding is not handled (covers the `_` arm).
    let escape_handled = api.on_trigger_keydown(&keyboard(KeyboardKey::Escape));

    assert!(enter_handled);
    assert!(!escape_handled);
    assert_eq!(sent.borrow().as_slice(), &[Event::Toggle]);
}

#[test]
fn programmatic_open_close_toggle_send_events() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.open();
    api.close();
    api.toggle();

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::Open, Event::Close, Event::Toggle],
    );
}

// ────────────────────────────────────────────────────────────────────
// Guards, controlled-prop no-op, Context helpers, accessors
// ────────────────────────────────────────────────────────────────────

#[test]
fn readonly_blocks_select_date() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn set_props_with_unchanged_props_is_noop() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let result = svc.set_props(props());

    assert!(!result.state_changed);
    assert!(result.pending_effects.is_empty());
    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn clear_trigger_disabled_when_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Disabled).as_deref(),
        Some("true"),
    );
}

#[test]
fn format_without_three_fields_falls_back_to_month_day_year() {
    // A `format` pattern that does not split into three tokens keeps the default
    // month/day/year order with a `/` separator (defensive `parse_format` path).
    let svc = service_with(
        Props {
            format: Some(String::from("yyyyMMdd")),
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    assert_eq!(svc.context().formatted_value(), "03/15/2024");
}

#[test]
fn context_parse_input_and_formatted_value() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    assert_eq!(svc.context().formatted_value(), "03/15/2024");
    assert_eq!(
        svc.context().parse_input("06/20/2024"),
        Some(date(2024, 6, 20))
    );
    assert_eq!(svc.context().parse_input("nonsense"), None);
}

#[test]
fn selected_date_accessor_reflects_value() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(api.selected_date(), Some(&date(2024, 3, 15)));
    assert_eq!(api.formatted_value(), "03/15/2024");
}

#[test]
fn api_and_messages_debug_eq_impls() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert!(format!("{api:?}").contains("Api"));

    let messages = Messages::default();

    // Manual `PartialEq` (a clone shares the same `Arc`-backed `MessageFn`s).
    assert_eq!(messages.clone(), messages);
    // Manual `Debug`.
    assert!(format!("{messages:?}").contains("Messages"));
}

// ────────────────────────────────────────────────────────────────────
// Codex review #697 — controlled state, guards, predicate, form buttons
// ────────────────────────────────────────────────────────────────────

#[test]
fn readonly_blocks_focus_in_open() {
    // `FocusIn` must respect `readonly` exactly as the explicit `Open` event
    // does — a read-only field stays closed when focused.
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::FocusIn);

    assert_eq!(*svc.state(), State::Closed);
    assert!(!result.state_changed);
}

#[test]
fn select_date_does_not_change_display_when_value_is_controlled() {
    // Controlled-and-empty value: a calendar selection updates the internal
    // bindable but `get()` still returns the parent value (None), so the input
    // text, `selected_date()`, and hidden input must NOT show the new date.
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().input_text, "");
    let api = svc.connect(&|_| {});
    assert_eq!(api.selected_date(), None);
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some(""),
    );
}

#[test]
fn input_announces_default_value_without_typing() {
    // The selected-date announcement must be present for `default_value` (and
    // controlled values), not only for dates typed/selected this session.
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Description)).as_deref(),
        Some("Selected date: 03/15/2024"),
    );
}

#[test]
fn sync_props_unrelated_change_preserves_typed_text() {
    // A partial/invalid in-progress entry must survive a prop change that does
    // not touch the value or format (here, flipping `invalid`).
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("03/15"),
    }));
    assert_eq!(svc.context().input_text, "03/15");
    assert_eq!(*svc.context().value.get(), None);

    drop(svc.set_props(Props {
        invalid: true,
        ..props()
    }));

    assert_eq!(svc.context().input_text, "03/15");
}

fn unavailable_after(cutoff: CalendarDate) -> calendar::IsDateUnavailableFn {
    Callback::new_ref(move |candidate: &CalendarDate| {
        candidate.compare(&cutoff) == Ordering::Greater
    })
}

#[test]
fn input_change_rejects_typed_unavailable_date() {
    // Typed input must honor `is_date_unavailable`, so a date the calendar would
    // refuse cannot be smuggled in by typing.
    let mut svc = service_with(
        Props {
            is_date_unavailable: Some(unavailable_after(date(2024, 6, 15))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("06/20/2024"),
    }));
    assert_eq!(*svc.context().value.get(), None);

    // An available date still commits.
    drop(svc.send(Event::InputChange {
        value: String::from("06/10/2024"),
    }));
    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 10)));
}

#[test]
fn trigger_and_clear_trigger_are_type_button() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button"),
    );
    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button"),
    );
}

#[test]
fn context_sync_input_text_reflects_value() {
    // `Context::sync_input_text` (public per spec §1.3) forces the input text to
    // match the current value, discarding any divergent in-progress text.
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    // Type over the displayed value without committing, then re-sync.
    drop(svc.send(Event::InputChange {
        value: String::from("nonsense"),
    }));
    assert_eq!(svc.context().input_text, "nonsense");

    svc.context_mut().sync_input_text();

    assert_eq!(svc.context().input_text, "03/15/2024");
}

#[test]
fn on_open_change_prop_round_trips() {
    let props = Props {
        on_open_change: Some(Callback::new(|_open: bool| {})),
        ..props()
    };

    assert!(props.on_open_change.is_some());
}

// ────────────────────────────────────────────────────────────────────
// Codex review #697 (pass 2) — focus, value notify, reject, readonly, ISO
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_in_open_keeps_input_focus() {
    // Opening via the focus path must NOT move focus into the calendar — the
    // user focused the input to type. Only `OpenChange` fires (no FocusCalendar).
    let mut svc = service();

    assert_eq!(effects(svc.send(Event::FocusIn)), vec![Effect::OpenChange]);
    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn select_date_records_requested_value_even_when_controlled() {
    // Controlled-and-empty value: the committed `value.get()` stays the parent's
    // (None), but `requested_value` carries the selected date so the adapter can
    // forward it to the parent, and `Effect::ValueChange` signals the change.
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    });

    assert!(effects(result).contains(&Effect::ValueChange));
    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().requested_value, Some(date(2024, 3, 15)));
}

#[test]
fn input_change_accepted_emits_value_change_and_records_request() {
    let mut svc = service();

    let result = svc.send(Event::InputChange {
        value: String::from("06/20/2024"),
    });

    assert_eq!(effects(result), vec![Effect::ValueChange]);
    assert_eq!(svc.context().requested_value, Some(date(2024, 6, 20)));
}

#[test]
fn input_change_partial_text_emits_no_value_change() {
    let mut svc = service();

    // Incomplete entry: no committed-value change, so no ValueChange effect.
    let result = svc.send(Event::InputChange {
        value: String::from("06/2"),
    });

    assert!(result.pending_effects.is_empty());
    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn typed_rejected_complete_date_clears_prior_value() {
    // A previously-selected value must be cleared when the user types a complete
    // date that is rejected, so the hidden input / calendar never submit a stale
    // date that contradicts the visible field.
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 6, 10)),
            max: Some(date(2024, 12, 31)),
            ..props()
        },
        en_us(),
    );
    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 10)));

    let result = svc.send(Event::InputChange {
        value: String::from("01/01/2025"),
    });

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().requested_value, None);
    assert!(effects(result).contains(&Effect::ValueChange));
    // Hidden input no longer submits the stale date.
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some(""),
    );
}

#[test]
fn readonly_trigger_is_disabled() {
    let svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Disabled).as_deref(),
        Some("true"),
    );
    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
}

#[test]
fn hidden_input_uses_canonical_iso() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 12, 25)),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    // The hidden input must equal the date's canonical ISO 8601 string.
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value),
        Some(date(2024, 12, 25).to_iso8601()),
    );
}

// ────────────────────────────────────────────────────────────────────
// Codex review #697 (pass 3) — required, today, typed controlled display
// ────────────────────────────────────────────────────────────────────

#[test]
fn required_sets_native_required_on_input() {
    // ARIA alone does not drive browser constraint validation; the visible
    // input must carry the native `required` attribute too.
    let svc = service_with(
        Props {
            required: true,
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Required).as_deref(),
        Some("true"),
    );
    assert_eq!(
        attr(&api.input_attrs(), HtmlAttr::Aria(AriaAttr::Required)).as_deref(),
        Some("true"),
    );
}

#[test]
fn calendar_props_forward_injected_today() {
    // The adapter-injected `today` must reach the embedded calendar so an empty
    // picker opens on the current month rather than the calendar default.
    let svc = service_with(
        Props {
            today: date(2024, 6, 15),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert_eq!(api.calendar_props().today, date(2024, 6, 15));
}

#[test]
fn typed_accepted_date_does_not_diverge_when_value_controlled() {
    // Controlled-and-empty value: typing a complete, valid date records the
    // request but the visible field reflects the bindable (still the parent's
    // empty value), so the input never diverges from selected_date()/hidden
    // input/calendar props before the parent echoes the change.
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::InputChange {
        value: String::from("06/20/2024"),
    }));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().input_text, "");
    assert_eq!(svc.context().requested_value, Some(date(2024, 6, 20)));
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some(""),
    );
}

#[test]
fn typed_accepted_date_normalizes_display_when_uncontrolled() {
    // Uncontrolled: the bindable holds the new value, so the field reflects the
    // canonical formatting of the accepted date.
    let mut svc = service();

    drop(svc.send(Event::InputChange {
        value: String::from("6/20/2024"),
    }));

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 20)));
    assert_eq!(svc.context().input_text, "06/20/2024");
}

// ────────────────────────────────────────────────────────────────────
// Codex review #697 (pass 4) — controlled-open veto, invalid clear, no-op
// ────────────────────────────────────────────────────────────────────

#[test]
fn controlled_open_user_toggle_does_not_change_state() {
    // With controlled `open: Some(false)`, a user Toggle must NOT open locally;
    // it records the request and signals `OpenChange` for the parent to honor.
    let mut svc = service_with(
        Props {
            open: Some(false),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::Toggle);

    assert_eq!(*svc.state(), State::Closed);
    assert!(svc.context().requested_open);
    assert_eq!(effects(result), vec![Effect::OpenChange]);
}

#[test]
fn controlled_open_user_close_does_not_change_state() {
    // With controlled `open: Some(true)`, a user Close must NOT close locally.
    let mut svc = service_with(
        Props {
            open: Some(true),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::Close);

    assert_eq!(*svc.state(), State::Open);
    assert!(!svc.context().requested_open);
    assert_eq!(effects(result), vec![Effect::OpenChange]);
}

#[test]
fn uncontrolled_open_user_toggle_changes_state() {
    // Uncontrolled open still commits state on user events.
    let mut svc = service();

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Open);
    assert!(svc.context().requested_open);
}

#[test]
fn typed_complete_invalid_date_clears_prior_value() {
    // `02/30/2024` parses structurally but is not a real date — it must clear
    // the committed value (not fall through as in-progress partial text), so the
    // hidden input never submits the stale prior date.
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 6, 10)),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::InputChange {
        value: String::from("02/30/2024"),
    });

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().input_text, "02/30/2024");
    assert!(effects(result).contains(&Effect::ValueChange));
}

#[test]
fn partial_text_keeps_value_and_emits_no_value_change() {
    // Genuinely incomplete text leaves the committed value intact.
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 6, 10)),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::InputChange {
        value: String::from("02/3"),
    });

    assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 10)));
    assert!(result.pending_effects.is_empty());
}

#[test]
fn selecting_already_selected_date_emits_no_value_change() {
    // Re-selecting the current value must not fire `ValueChange` (avoids noisy
    // callbacks / redundant controlled reconciliation), though the popover still
    // closes.
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );
    drop(svc.send(Event::Open));

    let emitted = effects(svc.send(Event::SelectDate {
        date: date(2024, 3, 15),
    }));

    assert!(!emitted.contains(&Effect::ValueChange));
    assert_eq!(*svc.state(), State::Closed);
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — root
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_root_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_closed", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_open", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_disabled", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_readonly() {
    let svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_readonly", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_rtl() {
    let svc = service_with(
        Props {
            is_rtl: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_rtl", snapshot_attrs(&api.root_attrs()));
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — label / control
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_label() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("label", snapshot_attrs(&api.label_attrs()));
}

#[test]
fn snapshot_control() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("control", snapshot_attrs(&api.control_attrs()));
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — input
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_input_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_closed", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_open", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_with_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    // Surface the parsed_date-driven aria-description by re-parsing the value.
    drop(svc.send(Event::InputChange {
        value: String::from("03/15/2024"),
    }));

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_with_value", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_invalid() {
    let svc = service_with(
        Props {
            invalid: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_invalid", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_disabled", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_readonly() {
    let svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_readonly", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_required() {
    let svc = service_with(
        Props {
            required: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_required", snapshot_attrs(&api.input_attrs()));
}

#[test]
fn snapshot_input_with_placeholder() {
    let svc = service_with(
        Props {
            placeholder: Some(String::from("MM/DD/YYYY")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("input_with_placeholder", snapshot_attrs(&api.input_attrs()));
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — trigger / clear-trigger
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_trigger_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_closed", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_trigger_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_open", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_trigger_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_disabled", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_clear_trigger_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "clear_trigger_empty",
        snapshot_attrs(&api.clear_trigger_attrs())
    );
}

#[test]
fn snapshot_clear_trigger_with_value() {
    let svc = service_with(
        Props {
            default_value: Some(date(2024, 3, 15)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "clear_trigger_with_value",
        snapshot_attrs(&api.clear_trigger_attrs()),
    );
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — positioner / content
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_positioner_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("positioner_closed", snapshot_attrs(&api.positioner_attrs()));
}

#[test]
fn snapshot_positioner_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("positioner_open", snapshot_attrs(&api.positioner_attrs()));
}

#[test]
fn snapshot_content_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("content_open", snapshot_attrs(&api.content_attrs()));
}

// ────────────────────────────────────────────────────────────────────
// Snapshots — description / error / hidden input
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_description() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("description", snapshot_attrs(&api.description_attrs()));
}

#[test]
fn snapshot_error_message() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("error_message", snapshot_attrs(&api.error_message_attrs()));
}

#[test]
fn snapshot_hidden_input_empty() {
    let svc = service_with(
        Props {
            name: Some(String::from("date")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "hidden_input_empty",
        snapshot_attrs(&api.hidden_input_attrs())
    );
}

#[test]
fn snapshot_hidden_input_with_value() {
    let mut svc = service_with(
        Props {
            name: Some(String::from("date")),
            default_value: Some(date(2024, 12, 25)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 12, 25),
    }));

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "hidden_input_with_value",
        snapshot_attrs(&api.hidden_input_attrs()),
    );
}
