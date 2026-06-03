//! Unit and snapshot tests for the `DateRangePicker` component.
//!
//! Test names that begin with `snapshot_` use `insta::assert_snapshot!` and
//! commit golden output under `snapshots/`. Every other test is a pure
//! state-machine or connect-API assertion that does not depend on `.snap` files.

use alloc::{format, string::String, sync::Arc, vec, vec::Vec};

use ars_core::{AriaAttr, AttrMap, Env, HtmlAttr, SendResult, Service};
use ars_i18n::{CalendarDate, DateRange, Locale, StubIntlBackend, locales::en_us};
use ars_interactions::KeyboardKey;
use insta::assert_snapshot;

use super::*;

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn range(start: CalendarDate, end: CalendarDate) -> DateRange {
    DateRange::new(start, end).expect("ordered test range")
}

fn props() -> Props {
    Props {
        id: String::from("trip"),
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

fn sample_presets() -> Vec<Preset> {
    vec![
        Preset::new("Last 7 days", range(date(2025, 5, 26), date(2025, 6, 1))),
        Preset::new("Last 30 days", range(date(2025, 5, 3), date(2025, 6, 1))),
    ]
}

fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

fn attr(attrs: &AttrMap, key: HtmlAttr) -> Option<String> {
    attrs.get(&key).map(ToString::to_string)
}

/// Returns `true` when an event was a no-op: neither the state nor the context
/// changed (the machine's `transition` returned `None`).
fn unchanged(result: &SendResult<Machine>) -> bool {
    !result.state_changed && !result.context_changed
}

// ────────────────────────────────────────────────────────────────────
// Initial state
// ────────────────────────────────────────────────────────────────────

#[test]
fn initial_state_is_closed() {
    let svc = service();

    assert_eq!(*svc.state(), State::Closed);
    assert!(!*svc.context().open.get());
    assert!(svc.context().value.get().is_none());
    assert_eq!(svc.context().active_field, ActiveField::Start);
}

#[test]
fn default_value_seeds_range_and_fields() {
    let initial = range(date(2025, 6, 1), date(2025, 6, 15));

    let svc = service_with(
        Props {
            default_value: Some(initial.clone()),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.context().value.get(), Some(initial));
    assert_eq!(svc.context().start_date, Some(date(2025, 6, 1)));
    assert_eq!(svc.context().end_date, Some(date(2025, 6, 15)));
}

#[test]
fn controlled_value_overrides_default() {
    let controlled = range(date(2025, 1, 1), date(2025, 1, 31));

    let svc = service_with(
        Props {
            value: Some(Some(controlled.clone())),
            default_value: Some(range(date(2030, 1, 1), date(2030, 1, 2))),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.context().value.get(), Some(controlled));
}

// ────────────────────────────────────────────────────────────────────
// Popover open / close
// ────────────────────────────────────────────────────────────────────

#[test]
fn open_and_close() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Open);
    assert!(*svc.context().open.get());

    drop(svc.send(Event::Close));

    assert_eq!(*svc.state(), State::Closed);
    assert!(!*svc.context().open.get());
}

#[test]
fn toggle_flips_open_state() {
    let mut svc = service();

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Open);

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn arrow_down_opens_from_closed() {
    let mut svc = service();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowDown,
    }));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn escape_closes_when_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    }));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn escape_is_ignored_when_closed() {
    let mut svc = service();

    let result = svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    });

    assert!(unchanged(&result));
    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn focusout_closes_when_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::FocusOut));

    assert_eq!(*svc.state(), State::Closed);
}

#[test]
fn focusin_is_a_noop() {
    let mut svc = service();

    let result = svc.send(Event::FocusIn);

    assert!(unchanged(&result));
    assert_eq!(*svc.state(), State::Closed);
}

// ────────────────────────────────────────────────────────────────────
// Range selection from the calendar
// ────────────────────────────────────────────────────────────────────

#[test]
fn select_range_complete_sets_value_and_closes() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let selected = range(date(2025, 3, 1), date(2025, 3, 15));

    drop(svc.send(Event::SelectRangeComplete {
        range: selected.clone(),
    }));

    assert_eq!(*svc.state(), State::Closed);
    assert_eq!(*svc.context().value.get(), Some(selected));
    assert_eq!(svc.context().start_date, Some(date(2025, 3, 1)));
    assert_eq!(svc.context().end_date, Some(date(2025, 3, 15)));
}

#[test]
fn select_range_complete_stays_open_when_not_close_on_select() {
    let mut svc = service_with(
        Props {
            close_on_select: false,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));

    let selected = range(date(2025, 3, 1), date(2025, 3, 15));

    drop(svc.send(Event::SelectRangeComplete {
        range: selected.clone(),
    }));

    assert_eq!(*svc.state(), State::Open);
    assert_eq!(*svc.context().value.get(), Some(selected));
}

#[test]
fn select_range_complete_blocked_when_readonly() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));

    let result = svc.send(Event::SelectRangeComplete {
        range: range(date(2025, 3, 1), date(2025, 3, 15)),
    });

    assert!(unchanged(&result));
    assert!(svc.context().value.get().is_none());
}

#[test]
fn select_range_complete_commits_even_after_close() {
    // Browser ordering: clicking a calendar cell can fire FocusOut (closing the
    // popover) before the calendar reports the completed range. The selection
    // must still commit; only the close side-effect is gated on being open.
    let mut svc = service();
    drop(svc.send(Event::Open));
    drop(svc.send(Event::FocusOut));
    assert_eq!(*svc.state(), State::Closed);

    let selected = range(date(2025, 3, 1), date(2025, 3, 15));
    drop(svc.send(Event::SelectRangeComplete {
        range: selected.clone(),
    }));

    assert_eq!(*svc.state(), State::Closed);
    assert_eq!(*svc.context().value.get(), Some(selected));
    assert_eq!(svc.context().start_date, Some(date(2025, 3, 1)));
    assert_eq!(svc.context().end_date, Some(date(2025, 3, 15)));
}

// ────────────────────────────────────────────────────────────────────
// Field value coordination
// ────────────────────────────────────────────────────────────────────

#[test]
fn start_value_change_tracks_field_and_active() {
    let mut svc = service();

    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 1)))));

    assert_eq!(svc.context().start_date, Some(date(2025, 6, 1)));
    assert_eq!(svc.context().active_field, ActiveField::Start);
    // Range incomplete until both fields are set.
    assert!(svc.context().value.get().is_none());
}

#[test]
fn end_value_change_tracks_field_and_active() {
    let mut svc = service();

    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 15)))));

    assert_eq!(svc.context().end_date, Some(date(2025, 6, 15)));
    assert_eq!(svc.context().active_field, ActiveField::End);
    assert!(svc.context().value.get().is_none());
}

#[test]
fn field_edits_assemble_into_range() {
    let mut svc = service();

    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 1)))));
    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 15)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2025, 6, 1), date(2025, 6, 15)))
    );
}

#[test]
fn range_normalizes_when_start_after_end() {
    let mut svc = service();

    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 25)))));
    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 1)))));

    // The two field values are swapped so the canonical range stays ordered.
    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2025, 6, 1), date(2025, 6, 25)))
    );
    assert_eq!(svc.context().start_date, Some(date(2025, 6, 1)));
    assert_eq!(svc.context().end_date, Some(date(2025, 6, 25)));
}

#[test]
fn clearing_a_field_clears_the_range() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::StartValueChange(None)));

    assert!(svc.context().value.get().is_none());
    assert!(svc.context().start_date.is_none());
    assert_eq!(svc.context().end_date, Some(date(2025, 6, 15)));
}

#[test]
fn field_value_change_blocked_when_readonly() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    assert!(unchanged(
        &svc.send(Event::StartValueChange(Some(date(2025, 6, 1))))
    ));
    assert!(unchanged(
        &svc.send(Event::EndValueChange(Some(date(2025, 6, 15))))
    ));
    assert!(svc.context().start_date.is_none());
    assert!(svc.context().end_date.is_none());
}

// ────────────────────────────────────────────────────────────────────
// Clear
// ────────────────────────────────────────────────────────────────────

#[test]
fn clear_resets_value_and_fields() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Clear));

    assert!(svc.context().value.get().is_none());
    assert!(svc.context().start_date.is_none());
    assert!(svc.context().end_date.is_none());
}

#[test]
fn clear_blocked_when_readonly() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::Clear);

    assert!(unchanged(&result));
    assert!(svc.context().value.get().is_some());
}

// ────────────────────────────────────────────────────────────────────
// Presets
// ────────────────────────────────────────────────────────────────────

#[test]
fn select_preset_applies_range_and_closes() {
    let mut svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::Open));
    drop(svc.send(Event::SelectPreset { index: 0 }));

    assert_eq!(*svc.state(), State::Closed);
    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2025, 5, 26), date(2025, 6, 1)))
    );
    assert_eq!(svc.context().start_date, Some(date(2025, 5, 26)));
    assert_eq!(svc.context().end_date, Some(date(2025, 6, 1)));
}

#[test]
fn select_preset_from_closed_stays_closed() {
    let mut svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SelectPreset { index: 1 }));

    assert_eq!(*svc.state(), State::Closed);
    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2025, 5, 3), date(2025, 6, 1)))
    );
}

#[test]
fn select_preset_out_of_range_is_ignored() {
    let mut svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::SelectPreset { index: 9 });

    assert!(unchanged(&result));
    assert!(svc.context().value.get().is_none());
}

#[test]
fn select_preset_blocked_when_readonly() {
    let mut svc = service_with(
        Props {
            presets: sample_presets(),
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let result = svc.send(Event::SelectPreset { index: 0 });

    assert!(unchanged(&result));
    assert!(svc.context().value.get().is_none());
}

// ────────────────────────────────────────────────────────────────────
// Disabled
// ────────────────────────────────────────────────────────────────────

#[test]
fn disabled_ignores_events() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    assert!(unchanged(&svc.send(Event::Open)));
    assert!(unchanged(&svc.send(Event::Toggle)));
    assert!(unchanged(
        &svc.send(Event::StartValueChange(Some(date(2025, 6, 1))))
    ));
    assert_eq!(*svc.state(), State::Closed);
    assert!(svc.context().start_date.is_none());
}

// ────────────────────────────────────────────────────────────────────
// Controlled prop sync
// ────────────────────────────────────────────────────────────────────

#[test]
fn controlled_value_syncs_via_set_props() {
    let mut svc = service_with(
        Props {
            value: Some(Some(range(date(2025, 6, 1), date(2025, 6, 15)))),
            name: Some(String::from("trip-range")),
            ..props()
        },
        en_us(),
    );

    drop(svc.set_props(Props {
        value: Some(Some(range(date(2025, 9, 1), date(2025, 9, 30)))),
        name: Some(String::from("trip-range")),
        ..props()
    }));

    let ctx = svc.context();

    assert_eq!(ctx.start_date, Some(date(2025, 9, 1)));
    assert_eq!(ctx.end_date, Some(date(2025, 9, 30)));
    assert_eq!(
        *ctx.value.get(),
        Some(range(date(2025, 9, 1), date(2025, 9, 30)))
    );
}

#[test]
fn controlled_to_uncontrolled_reveals_consistent_value() {
    // A controlled instance whose parent later stops passing `value` (drops to
    // uncontrolled) must reveal an internal value that matches the per-field
    // values, not a stale earlier value.
    let mut svc = service_with(
        Props {
            value: Some(Some(range(date(2025, 6, 1), date(2025, 6, 15)))),
            ..props()
        },
        en_us(),
    );

    // The second update carries no `value` (uncontrolled), keeping the same id.
    drop(svc.set_props(props()));

    let ctx = svc.context();

    assert!(!ctx.value.is_controlled());
    assert_eq!(ctx.start_date, Some(date(2025, 6, 1)));
    assert_eq!(ctx.end_date, Some(date(2025, 6, 15)));
    assert_eq!(
        *ctx.value.get(),
        Some(range(date(2025, 6, 1), date(2025, 6, 15)))
    );
}

#[test]
fn identical_props_emit_no_sync() {
    let mut svc = service();

    // An unchanged props snapshot must not produce a SyncProps event.
    let result = svc.set_props(props());

    assert!(unchanged(&result));
}

#[test]
fn hidden_inputs_when_disabled_and_empty() {
    let svc = service_with(
        Props {
            disabled: true,
            name: Some(String::from("range")),
            start_name: Some(String::from("from")),
            end_name: Some(String::from("to")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    // Disabled controls are excluded from submission via `disabled`, and an
    // empty range submits an empty value.
    let combined = api.hidden_input_attrs();

    assert_eq!(attr(&combined, HtmlAttr::Disabled).as_deref(), Some("true"));
    assert_eq!(attr(&combined, HtmlAttr::Value).as_deref(), Some(""));

    let start = api.start_hidden_input_attrs();

    assert_eq!(attr(&start, HtmlAttr::Disabled).as_deref(), Some("true"));
    assert_eq!(attr(&start, HtmlAttr::Value).as_deref(), Some(""));

    let end = api.end_hidden_input_attrs();

    assert_eq!(attr(&end, HtmlAttr::Disabled).as_deref(), Some("true"));
    assert_eq!(attr(&end, HtmlAttr::Value).as_deref(), Some(""));
}

#[test]
fn prop_updates_sync_bounds_today_and_presets() {
    let mut svc = service();

    drop(svc.set_props(Props {
        min: Some(date(2025, 1, 1)),
        max: Some(date(2025, 12, 31)),
        today: date(2025, 7, 4),
        presets: sample_presets(),
        visible_months: 3,
        is_rtl: true,
        readonly: true,
        required: true,
        ..props()
    }));

    let ctx = svc.context();

    assert_eq!(ctx.min, Some(date(2025, 1, 1)));
    assert_eq!(ctx.max, Some(date(2025, 12, 31)));
    assert_eq!(ctx.today, date(2025, 7, 4));
    assert_eq!(ctx.presets.len(), 2);
    assert_eq!(ctx.visible_months, 3);
    assert!(ctx.is_rtl);
    assert!(ctx.readonly);
    assert!(ctx.required);
}

#[test]
fn disabled_component_still_processes_sync_props() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    // Re-enabling via props must reach the context even though the component is
    // currently disabled.
    drop(svc.set_props(Props {
        disabled: false,
        ..props()
    }));

    assert!(!svc.context().disabled);

    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn disabling_via_props_while_open_closes_popover() {
    // Disabling an open picker must dismiss it: otherwise the disabled guard
    // blocks Escape/FocusOut/Close and the dialog is stuck open.
    let mut svc = service();
    drop(svc.send(Event::Open));
    assert_eq!(*svc.state(), State::Open);

    drop(svc.set_props(Props {
        disabled: true,
        ..props()
    }));

    assert_eq!(*svc.state(), State::Closed);
    assert!(!*svc.context().open.get());
}

// ────────────────────────────────────────────────────────────────────
// Connect API: ARIA / attributes
// ────────────────────────────────────────────────────────────────────

#[test]
fn label_targets_start_input() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.label_attrs(), HtmlAttr::For).as_deref(),
        Some("trip-start-input")
    );
}

#[test]
fn control_is_a_labelled_group() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let attrs = api.control_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::LabelledBy)).as_deref(),
        Some("trip-label")
    );
}

#[test]
fn control_describedby_chains_description_and_error() {
    let svc = service_with(
        Props {
            has_description: true,
            has_error_message: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.control_attrs(), HtmlAttr::Aria(AriaAttr::DescribedBy)).as_deref(),
        Some("trip-description trip-error-message")
    );
}

#[test]
fn trigger_exposes_popup_expanded_and_controls() {
    let mut svc = service();

    {
        let api = svc.connect(&|_| {});

        let attrs = api.trigger_attrs();

        assert_eq!(
            attr(&attrs, HtmlAttr::Aria(AriaAttr::HasPopup)).as_deref(),
            Some("dialog")
        );
        assert_eq!(
            attr(&attrs, HtmlAttr::Aria(AriaAttr::Expanded)).as_deref(),
            Some("false")
        );
        assert_eq!(
            attr(&attrs, HtmlAttr::Aria(AriaAttr::Controls)).as_deref(),
            Some("trip-content")
        );
    }

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Aria(AriaAttr::Expanded)).as_deref(),
        Some("true")
    );
}

#[test]
fn clear_trigger_disabled_when_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Disabled).as_deref(),
        Some("true")
    );
}

#[test]
fn clear_trigger_enabled_with_value() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(attr(&api.clear_trigger_attrs(), HtmlAttr::Disabled), None);
}

#[test]
fn clear_trigger_disabled_when_readonly() {
    // `Event::Clear` is rejected when readonly, so the rendered control must be
    // disabled rather than expose an actionable button that does nothing.
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Disabled).as_deref(),
        Some("true")
    );
}

#[test]
fn content_is_a_non_modal_dialog() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let attrs = api.content_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Role).as_deref(), Some("dialog"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::Modal)).as_deref(),
        Some("false")
    );
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::LabelledBy)).as_deref(),
        Some("trip-label")
    );
    assert_eq!(attr(&attrs, HtmlAttr::Id).as_deref(), Some("trip-content"));
}

#[test]
fn error_message_has_alert_role() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.error_message_attrs(), HtmlAttr::Role).as_deref(),
        Some("alert")
    );
}

#[test]
fn root_marks_invalid_when_out_of_bounds() {
    let svc = service_with(
        Props {
            max: Some(date(2025, 6, 30)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 7, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert!(api.is_invalid());
    assert_eq!(
        attr(&api.root_attrs(), HtmlAttr::Data("ars-invalid")).as_deref(),
        Some("true")
    );
}

// ────────────────────────────────────────────────────────────────────
// Connect API: form integration
// ────────────────────────────────────────────────────────────────────

#[test]
fn hidden_input_carries_iso_interval() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2024, 1, 10), date(2024, 1, 20))),
            name: Some(String::from("range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let attrs = api.hidden_input_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Name).as_deref(), Some("range"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Value).as_deref(),
        Some("2024-01-10/2024-01-20")
    );
}

#[test]
fn hidden_input_empty_without_range() {
    let svc = service_with(
        Props {
            name: Some(String::from("range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("")
    );
}

#[test]
fn split_hidden_inputs_carry_each_endpoint() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2024, 1, 10), date(2024, 1, 20))),
            start_name: Some(String::from("from")),
            end_name: Some(String::from("to")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_hidden_input_attrs();

    assert_eq!(attr(&start, HtmlAttr::Name).as_deref(), Some("from"));
    assert_eq!(attr(&start, HtmlAttr::Value).as_deref(), Some("2024-01-10"));

    let end = api.end_hidden_input_attrs();

    assert_eq!(attr(&end, HtmlAttr::Name).as_deref(), Some("to"));
    assert_eq!(attr(&end, HtmlAttr::Value).as_deref(), Some("2024-01-20"));
}

// ────────────────────────────────────────────────────────────────────
// Connect API: child props + descriptions
// ────────────────────────────────────────────────────────────────────

#[test]
fn start_field_props_carry_bounds_and_label() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            min: Some(date(2025, 1, 1)),
            max: Some(date(2025, 12, 31)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let field = api.start_field_props();

    assert_eq!(field.id, "trip-start-input");
    assert_eq!(field.value, Some(Some(date(2025, 6, 1))));
    assert_eq!(field.min_value, Some(date(2025, 1, 1)));
    assert_eq!(field.max_value, Some(date(2025, 12, 31)));
    assert_eq!(field.aria_label.as_deref(), Some("Start date"));
}

#[test]
fn end_field_props_carry_bounds_and_label() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let field = api.end_field_props();

    assert_eq!(field.id, "trip-end-input");
    assert_eq!(field.value, Some(Some(date(2025, 6, 15))));
    assert_eq!(field.aria_label.as_deref(), Some("End date"));
}

#[test]
fn range_calendar_props_forward_today_and_visible_months() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            today: date(2025, 7, 4),
            visible_months: 3,
            is_rtl: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let calendar = api.range_calendar_props();

    assert_eq!(calendar.id, "trip-calendar");
    assert_eq!(calendar.today, date(2025, 7, 4));
    assert_eq!(calendar.visible_months, 3);
    assert!(calendar.is_rtl);
    assert_eq!(
        calendar.value,
        Some(Some(range(date(2025, 6, 1), date(2025, 6, 15))))
    );
}

#[test]
fn range_description_present_only_when_complete() {
    let empty = service();

    assert!(empty.connect(&|_| {}).range_description().is_none());

    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 3, 1), date(2025, 3, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        api.range_description().as_deref(),
        Some("March 1, 2025 to March 15, 2025")
    );
}

#[test]
fn preset_helpers_expose_labels_and_attrs() {
    let svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(api.presets().len(), 2);
    assert_eq!(api.preset_label(0), Some("Last 7 days"));
    assert_eq!(api.preset_label(9), None);

    let attrs = api.preset_trigger_attrs(0);

    assert_eq!(
        attr(&attrs, HtmlAttr::Data("ars-index")).as_deref(),
        Some("0")
    );
    assert_eq!(attr(&attrs, HtmlAttr::Disabled), None);

    // Out-of-range preset trigger is disabled.
    assert_eq!(
        attr(&api.preset_trigger_attrs(9), HtmlAttr::Disabled).as_deref(),
        Some("true")
    );
}

#[test]
fn preset_trigger_disabled_when_readonly() {
    // `Event::SelectPreset` is rejected when readonly, so a valid preset's
    // button must render disabled rather than do nothing when clicked.
    let svc = service_with(
        Props {
            presets: sample_presets(),
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.preset_trigger_attrs(0), HtmlAttr::Disabled).as_deref(),
        Some("true")
    );
}

#[test]
fn imperative_methods_dispatch_events() {
    use core::cell::RefCell;

    let events: RefCell<Vec<Event>> = RefCell::new(Vec::new());
    let record = |event| events.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&record);

    api.open();
    api.close();
    api.toggle();
    api.clear();
    api.select_range(range(date(2025, 3, 1), date(2025, 3, 15)));
    api.select_preset(0);
    api.set_start_value(Some(date(2025, 3, 1)));
    api.set_end_value(None);
    api.focus_in();
    api.focus_out();
    api.on_key_down(KeyboardKey::ArrowDown);

    let recorded = events.borrow();

    assert_eq!(recorded.len(), 11);
    assert_eq!(recorded[0], Event::Open);
    assert_eq!(recorded[5], Event::SelectPreset { index: 0 });
    assert_eq!(recorded[7], Event::EndValueChange(None));
}

#[test]
fn connect_api_dispatches_every_part() {
    use ars_core::{ComponentPart, ConnectApi};

    let svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    // Every anatomy part produces attributes without panicking, exercising the
    // `ConnectApi::part_attrs` dispatch (including the data-carrying
    // `PresetTrigger` arm).
    for part in Part::all() {
        drop(api.part_attrs(part));
    }
}

#[test]
fn convenience_getters_reflect_state() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        api.selected_range(),
        Some(&range(date(2025, 6, 1), date(2025, 6, 15)))
    );
    assert_eq!(api.active_field(), ActiveField::Start);
    assert!(!api.is_open());
}

#[test]
fn debug_impls_render() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert!(format!("{:?}", svc.context()).contains("Context"));
    assert!(format!("{api:?}").contains("Api"));
}

#[test]
fn props_builders_match_struct_literal() {
    let built = Props::new()
        .id("trip")
        .value(Some(range(date(2025, 1, 1), date(2025, 1, 2))))
        .default_value(Some(range(date(2030, 1, 1), date(2030, 1, 2))))
        .min(Some(date(2024, 1, 1)))
        .max(Some(date(2026, 1, 1)))
        .today(date(2025, 7, 4))
        .presets(sample_presets())
        .visible_months(3)
        .is_rtl(true)
        .disabled(true)
        .readonly(true)
        .required(true)
        .force_leading_zeros(true)
        .has_description(true)
        .has_error_message(true)
        .name(Some(String::from("range")))
        .start_name(Some(String::from("from")))
        .end_name(Some(String::from("to")))
        .close_on_select(false);

    let literal = Props {
        id: String::from("trip"),
        value: Some(Some(range(date(2025, 1, 1), date(2025, 1, 2)))),
        default_value: Some(range(date(2030, 1, 1), date(2030, 1, 2))),
        min: Some(date(2024, 1, 1)),
        max: Some(date(2026, 1, 1)),
        today: date(2025, 7, 4),
        presets: sample_presets(),
        visible_months: 3,
        is_rtl: true,
        disabled: true,
        readonly: true,
        required: true,
        force_leading_zeros: true,
        has_description: true,
        has_error_message: true,
        name: Some(String::from("range")),
        start_name: Some(String::from("from")),
        end_name: Some(String::from("to")),
        close_on_select: false,
    };

    assert_eq!(built, literal);
}

// ────────────────────────────────────────────────────────────────────
// Snapshots
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_root_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
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

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
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

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_required() {
    let svc = service_with(
        Props {
            required: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_invalid() {
    let svc = service_with(
        Props {
            max: Some(date(2025, 6, 30)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 7, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_label() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.label_attrs()));
}

#[test]
fn snapshot_control() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.control_attrs()));
}

#[test]
fn snapshot_control_with_description_and_error() {
    let svc = service_with(
        Props {
            has_description: true,
            has_error_message: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.control_attrs()));
}

#[test]
fn snapshot_start_input_marker() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.start_input_attrs()));
}

#[test]
fn snapshot_end_input_marker() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.end_input_attrs()));
}

#[test]
fn snapshot_separator() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.separator_attrs()));
}

#[test]
fn snapshot_trigger_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_trigger_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.trigger_attrs()));
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

    assert_snapshot!(snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_clear_trigger_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.clear_trigger_attrs()));
}

#[test]
fn snapshot_clear_trigger_with_value() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.clear_trigger_attrs()));
}

#[test]
fn snapshot_preset_trigger() {
    let svc = service_with(
        Props {
            presets: sample_presets(),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.preset_trigger_attrs(1)));
}

#[test]
fn snapshot_positioner() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.positioner_attrs()));
}

#[test]
fn snapshot_content() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.content_attrs()));
}

#[test]
fn snapshot_description() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.description_attrs()));
}

#[test]
fn snapshot_error_message() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.error_message_attrs()));
}

#[test]
fn snapshot_hidden_input_empty() {
    let svc = service_with(
        Props {
            name: Some(String::from("range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.hidden_input_attrs()));
}

#[test]
fn snapshot_hidden_input_with_range() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2024, 1, 10), date(2024, 1, 20))),
            name: Some(String::from("range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.hidden_input_attrs()));
}

#[test]
fn snapshot_start_hidden_input() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2024, 1, 10), date(2024, 1, 20))),
            start_name: Some(String::from("from")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.start_hidden_input_attrs()));
}

#[test]
fn snapshot_end_hidden_input() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2024, 1, 10), date(2024, 1, 20))),
            end_name: Some(String::from("to")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.end_hidden_input_attrs()));
}

#[test]
fn snapshot_start_field_props() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let child = svc_child_field_group(api.start_field_props());

    assert_snapshot!(snapshot_attrs(&child));
}

#[test]
fn snapshot_end_field_props() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let child = svc_child_field_group(api.end_field_props());

    assert_snapshot!(snapshot_attrs(&child));
}

/// Drives a child `DateField` from the given props and returns its field-group
/// attributes, exercising the start/end child configuration end-to-end.
fn svc_child_field_group(child: date_field::Props) -> AttrMap {
    let service =
        Service::<date_field::Machine>::new(child, &env(en_us()), &date_field::Messages::default());

    service.connect(&|_| {}).field_group_attrs()
}
