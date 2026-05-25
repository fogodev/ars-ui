//! Unit-level contract tests for Meter, Stat, and Progress data-display cores.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use ars_components::data_display::{meter, progress, stat};
use ars_core::{AriaAttr, ConnectApi, CssProperty, Env, HtmlAttr, Machine as _, Service};
use ars_i18n::number;

fn meter_api(props: meter::Props) -> meter::Api {
    meter::Api::new(props, &Env::default(), &meter::Messages::default())
}

fn stat_api(props: stat::Props) -> stat::Api {
    stat::Api::new(props, &Env::default(), &stat::Messages::default())
}

fn progress_service(props: progress::Props) -> Service<progress::Machine> {
    Service::<progress::Machine>::new(props, &Env::default(), &progress::Messages::default())
}

fn hash_progress_part(part: &progress::Part) -> u64 {
    let mut hasher = DefaultHasher::new();

    part.hash(&mut hasher);

    hasher.finish()
}

#[test]
fn meter_props_builders_write_all_fields() {
    let options = number::FormatOptions {
        max_fraction_digits: 1,
        ..number::FormatOptions::default()
    };

    let props = meter::Props::new()
        .id("disk")
        .value(72.0)
        .min(10.0)
        .max(110.0)
        .low(30.0)
        .high(90.0)
        .optimum(60.0)
        .format_options(options.clone());

    assert_eq!(props.id, "disk");
    assert_eq!(props.value, 72.0);
    assert_eq!(props.min, 10.0);
    assert_eq!(props.max, 110.0);
    assert_eq!(props.low, Some(30.0));
    assert_eq!(props.high, Some(90.0));
    assert_eq!(props.optimum, Some(60.0));
    assert_eq!(props.format_options, Some(options));
}

#[test]
fn meter_value_text_uses_format_options() {
    let options = number::FormatOptions {
        max_fraction_digits: 1,
        ..number::FormatOptions::default()
    };

    let api = meter_api(
        meter::Props::new()
            .id("meter")
            .value(12.5)
            .format_options(options),
    );

    assert_eq!(api.value_text(), "12.5%");
}

#[test]
fn meter_value_text_uses_sanitized_display_bounds() {
    let api = meter_api(
        meter::Props::new()
            .id("meter")
            .value(50.0)
            .min(100.0)
            .max(0.0),
    );

    let root = api.root_attrs();

    assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
    assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
    assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("50"));
    assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::ValueText)), Some("50%"));
    assert_eq!(api.value_text(), "50%");
    assert!(
        api.range_attrs()
            .styles()
            .contains(&(CssProperty::Width, String::from("50%")))
    );
}

#[test]
fn meter_root_exposes_meter_aria_and_native_attrs() {
    let attrs = meter_api(
        meter::Props::new()
            .id("disk")
            .value(72.0)
            .min(0.0)
            .max(256.0)
            .low(64.0)
            .high(192.0)
            .optimum(32.0),
    )
    .root_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Role), Some("meter"));
    assert_eq!(attrs.get(&HtmlAttr::Id), Some("disk"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("72"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("256"));
    assert_eq!(attrs.get(&HtmlAttr::Value), Some("72"));
    assert_eq!(attrs.get(&HtmlAttr::Min), Some("0"));
    assert_eq!(attrs.get(&HtmlAttr::Max), Some("256"));
    assert_eq!(attrs.get(&HtmlAttr::Low), Some("64"));
    assert_eq!(attrs.get(&HtmlAttr::High), Some("192"));
    assert_eq!(attrs.get(&HtmlAttr::Optimum), Some("32"));
}

#[test]
fn meter_root_clamps_and_sanitizes_value_attrs() {
    let high = meter_api(meter::Props::new().id("disk").value(150.0).max(100.0)).root_attrs();

    assert_eq!(high.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("100"));
    assert_eq!(high.get(&HtmlAttr::Value), Some("100"));

    let non_finite = meter_api(meter::Props::new().id("disk").value(f64::NAN)).root_attrs();

    assert_eq!(
        non_finite.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("0")
    );
    assert_eq!(non_finite.get(&HtmlAttr::Value), Some("0"));
    assert!(
        meter_api(meter::Props::new().id("disk").value(f64::INFINITY))
            .range_attrs()
            .styles()
            .contains(&(CssProperty::Width, String::from("0%")))
    );
}

#[test]
fn meter_computes_percent_segment_and_zone_attrs() {
    assert_eq!(meter::compute_percent(150.0, 50.0, 250.0), 50.0);
    assert_eq!(meter::compute_percent(0.0, 50.0, 250.0), 0.0);
    assert_eq!(meter::compute_percent(300.0, 50.0, 250.0), 100.0);

    assert_eq!(
        meter::compute_segment(10.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(10.0)),
        meter::Segment::Optimal
    );
    assert_eq!(
        meter::compute_segment(50.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(10.0)),
        meter::Segment::SubOptimal
    );
    assert_eq!(
        meter::compute_segment(90.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(10.0)),
        meter::Segment::SubSubOptimal
    );
    assert_eq!(
        meter::Zone::from_segment(&meter::Segment::SubSubOptimal),
        meter::Zone::Critical
    );

    assert_eq!(
        meter::compute_segment(90.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(20.0)),
        meter::Segment::SubOptimal,
        "optimum equal to low keeps the middle zone optimal"
    );
    assert_eq!(
        meter::compute_segment(10.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(80.0)),
        meter::Segment::SubOptimal,
        "optimum equal to high keeps the middle zone optimal"
    );
    assert_eq!(
        meter::compute_segment(20.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(90.0)),
        meter::Segment::SubOptimal,
        "low boundary is sub-optimal when the optimal region is high"
    );
    assert_eq!(
        meter::compute_segment(80.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(90.0)),
        meter::Segment::SubOptimal,
        "high boundary is sub-optimal when the optimal region is high"
    );
    assert_eq!(
        meter::compute_segment(90.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(90.0)),
        meter::Segment::Optimal,
        "values above high are optimal when optimum is high"
    );
    assert_eq!(
        meter::compute_segment(20.0, 0.0, 100.0, Some(20.0), Some(80.0), Some(10.0)),
        meter::Segment::SubOptimal,
        "low boundary is sub-optimal when the optimal region is low"
    );
    assert_eq!(
        meter::compute_segment(10.0, 10.0, 30.0, Some(18.0), Some(22.0), None),
        meter::Segment::SubOptimal,
        "default optimum is the midpoint"
    );
    assert_eq!(
        meter::compute_segment(50.0, 0.0, 100.0, Some(90.0), Some(10.0), Some(95.0)),
        meter::Segment::SubSubOptimal,
        "thresholds are normalized before segment classification"
    );
    assert_eq!(
        meter::compute_percent(50.0, 100.0, 100.0),
        0.0,
        "invalid bounds resolve to zero percent"
    );

    let api = meter_api(
        meter::Props::new()
            .id("strength")
            .value(90.0)
            .low(20.0)
            .high(80.0)
            .optimum(10.0),
    );

    let root = api.root_attrs();
    let range = api.range_attrs();

    assert_eq!(
        root.get(&HtmlAttr::Data("ars-segment")),
        Some("sub-sub-optimal")
    );
    assert_eq!(root.get(&HtmlAttr::Data("ars-zone")), Some("critical"));
    assert!(
        range
            .styles()
            .contains(&(CssProperty::Width, String::from("90%")))
    );

    let normalized_thresholds = meter_api(
        meter::Props::new()
            .id("normalized")
            .value(50.0)
            .low(90.0)
            .high(10.0)
            .optimum(95.0),
    )
    .root_attrs();

    assert_eq!(normalized_thresholds.get(&HtmlAttr::Low), Some("90"));
    assert_eq!(normalized_thresholds.get(&HtmlAttr::High), Some("90"));
    assert_eq!(normalized_thresholds.get(&HtmlAttr::Optimum), Some("95"));
}

#[test]
fn meter_part_attrs_delegates_for_all_parts() {
    let api = meter_api(meter::Props::new().id("meter").value(50.0));

    assert_eq!(api.part_attrs(meter::Part::Root), api.root_attrs());
    assert_eq!(api.part_attrs(meter::Part::Label), api.label_attrs());
    assert_eq!(api.part_attrs(meter::Part::Track), api.track_attrs());
    assert_eq!(api.part_attrs(meter::Part::Range), api.range_attrs());
    assert_eq!(
        api.part_attrs(meter::Part::ValueText),
        api.value_text_attrs()
    );
    assert_eq!(
        api.label_attrs().get(&HtmlAttr::Data("ars-part")),
        Some("label")
    );
    assert_eq!(
        api.track_attrs().get(&HtmlAttr::Data("ars-part")),
        Some("track")
    );
}

#[test]
fn stat_props_builders_write_all_fields() {
    let options = number::FormatOptions {
        min_fraction_digits: 1,
        max_fraction_digits: 1,
        ..number::FormatOptions::default()
    };

    let props = stat::Props::new()
        .id("revenue")
        .label("Revenue")
        .value("$42k")
        .change(-4.5)
        .trend(stat::Trend::Down)
        .help_text("Trailing 30 days")
        .loading(true)
        .format_options(options.clone());

    assert_eq!(props.id, "revenue");
    assert_eq!(props.label, "Revenue");
    assert_eq!(props.value, "$42k");
    assert_eq!(props.change, Some(-4.5));
    assert_eq!(props.trend, Some(stat::Trend::Down));
    assert_eq!(props.help_text, Some("Trailing 30 days".to_string()));
    assert!(props.loading);
    assert_eq!(props.format_options, Some(options));
}

#[test]
fn stat_root_and_structural_parts_match_contract() {
    let api = stat_api(
        stat::Props::new()
            .id("revenue")
            .label("Total Revenue")
            .value("$45,231")
            .loading(true)
            .help_text("Trailing 30 days"),
    );

    let root = api.root_attrs();

    assert_eq!(root.get(&HtmlAttr::Role), Some("group"));
    assert_eq!(root.get(&HtmlAttr::Id), Some("revenue"));
    assert_eq!(
        root.get(&HtmlAttr::Aria(AriaAttr::Label)),
        Some("Total Revenue: $45,231")
    );
    assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
    assert_eq!(root.get(&HtmlAttr::Data("ars-loading")), Some("true"));

    assert_eq!(api.part_attrs(stat::Part::Label), api.label_attrs());
    assert_eq!(api.part_attrs(stat::Part::Value), api.value_attrs());
    assert_eq!(api.part_attrs(stat::Part::HelpText), api.help_text_attrs());
}

#[test]
fn stat_derives_and_exposes_trend_direction() {
    let up = stat_api(stat::Props::new().id("stat").change(12.5));

    assert_eq!(up.resolved_trend(), Some(stat::Trend::Up));
    assert_eq!(
        up.change_attrs().get(&HtmlAttr::Data("ars-trend")),
        Some("up")
    );
    assert_eq!(
        up.trend_indicator_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::Hidden)),
        Some("true")
    );

    let down = stat_api(stat::Props::new().id("stat").change(-3.2));

    assert_eq!(down.resolved_trend(), Some(stat::Trend::Down));
    assert_eq!(
        down.change_attrs().get(&HtmlAttr::Data("ars-trend")),
        Some("down")
    );

    let neutral = stat_api(stat::Props::new().id("stat").change(0.0));

    assert_eq!(neutral.resolved_trend(), Some(stat::Trend::Neutral));
    assert_eq!(
        neutral.change_attrs().get(&HtmlAttr::Data("ars-trend")),
        Some("neutral")
    );

    let override_trend = stat_api(
        stat::Props::new()
            .id("stat")
            .change(12.5)
            .trend(stat::Trend::Down),
    );

    assert_eq!(override_trend.resolved_trend(), Some(stat::Trend::Down));
}

#[test]
fn stat_formats_change_and_accessible_label() {
    let api = stat_api(stat::Props::new().id("stat").change(12.5));

    assert_eq!(api.formatted_change(), Some("↑ 12.5%".to_string()));
    assert_eq!(
        api.change_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
        Some("12.5% increase")
    );
}

#[test]
fn progress_initializes_determinate_indeterminate_and_complete_states() {
    let determinate = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(40.0)
            .min(0.0)
            .max(80.0),
    );

    assert_eq!(determinate.state(), &progress::State::Idle);
    assert_eq!(determinate.context().percent, 50.0);

    let indeterminate = progress_service(progress::Props::new().id("upload"));

    assert_eq!(indeterminate.state(), &progress::State::Loading);
    assert!(indeterminate.context().indeterminate);

    let complete = progress_service(progress::Props::new().id("upload").default_value(100.0));

    assert_eq!(complete.state(), &progress::State::Complete);
    assert_eq!(complete.context().percent, 100.0);
}

#[test]
fn progress_props_builders_and_percent_math_match_contract() {
    let options = number::FormatOptions {
        max_fraction_digits: 2,
        ..number::FormatOptions::default()
    };

    let props = progress::Props::new()
        .id("upload")
        .value(Some(45.0))
        .default_value(10.0)
        .min(10.0)
        .max(80.0)
        .orientation(progress::Orientation::Vertical)
        .format_options(options.clone());

    assert_eq!(props.id, "upload");
    assert_eq!(props.value, Some(Some(45.0)));
    assert_eq!(props.default_value, Some(10.0));
    assert_eq!(props.min, 10.0);
    assert_eq!(props.max, 80.0);
    assert_eq!(props.orientation, progress::Orientation::Vertical);
    assert_eq!(props.format_options, Some(options));
    assert_eq!(
        progress::Context::compute_percent(Some(45.0), 10.0, 80.0),
        50.0
    );
    assert_eq!(
        progress::Context::compute_percent(Some(f64::NAN), 10.0, 80.0),
        0.0
    );
    assert_eq!(
        progress::Context::compute_percent(Some(f64::INFINITY), 10.0, 80.0),
        0.0
    );
    assert_eq!(
        progress::Context::compute_percent(Some(50.0), 100.0, 100.0),
        0.0
    );
}

#[test]
fn progress_root_attrs_reflect_determinate_and_indeterminate_modes() {
    let determinate = progress_service(progress::Props::new().id("upload").default_value(25.0));

    assert!(!determinate.connect(&|_| {}).is_indeterminate());

    let attrs = determinate.connect(&|_| {}).root_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Role), Some("progressbar"));
    assert_eq!(attrs.get(&HtmlAttr::Id), Some("upload"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("25"));
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));

    let indeterminate = progress_service(progress::Props::new().id("upload"));

    assert!(indeterminate.connect(&|_| {}).is_indeterminate());

    let attrs = indeterminate.connect(&|_| {}).root_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("loading"));
    assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ValueNow)));
}

#[test]
fn progress_root_clamps_and_sanitizes_value_now() {
    let high = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(150.0)
            .max(100.0),
    );

    assert_eq!(
        high.connect(&|_| {})
            .root_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("100")
    );

    let non_finite = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(f64::NAN)
            .min(10.0)
            .max(80.0),
    );

    let api = non_finite.connect(&|_| {});

    assert_eq!(api.percent(), 0.0);
    assert_eq!(
        api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("10")
    );
    assert!(
        api.range_attrs()
            .styles()
            .contains(&(CssProperty::Width, String::from("0%")))
    );
}

#[test]
fn progress_invalid_bounds_do_not_mark_complete() {
    let service = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(50.0)
            .min(100.0)
            .max(0.0),
    );

    let api = service.connect(&|_| {});

    assert_eq!(service.state(), &progress::State::Idle);
    assert_eq!(api.value_text(), "0% complete");
    assert_eq!(
        api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("0")
    );
    assert_eq!(
        api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueText)),
        Some("0% complete")
    );
    assert_eq!(
        api.root_attrs().get(&HtmlAttr::Data("ars-state")),
        Some("idle")
    );
}

#[test]
fn progress_on_props_changed_reports_each_controlled_change() {
    let old = progress::Props::new()
        .id("upload")
        .value(Some(10.0))
        .min(0.0)
        .max(100.0)
        .orientation(progress::Orientation::Horizontal);

    assert!(progress::Machine::on_props_changed(&old, &old).is_empty());

    let value_events = progress::Machine::on_props_changed(&old, &old.clone().value(Some(20.0)));

    assert_eq!(value_events, vec![progress::Event::SetValue(Some(20.0))]);

    let min_events = progress::Machine::on_props_changed(&old, &old.clone().min(10.0));

    assert_eq!(min_events, vec![progress::Event::SyncProps]);

    let max_events = progress::Machine::on_props_changed(&old, &old.clone().max(120.0));

    assert_eq!(max_events, vec![progress::Event::SyncProps]);

    let orientation_events = progress::Machine::on_props_changed(
        &old,
        &old.clone().orientation(progress::Orientation::Vertical),
    );

    assert_eq!(orientation_events, vec![progress::Event::SyncProps]);
}

#[test]
fn progress_set_props_syncs_controlled_value_bounds_and_orientation() {
    let mut service = progress_service(
        progress::Props::new()
            .id("upload")
            .value(Some(20.0))
            .min(0.0)
            .max(100.0),
    );

    drop(
        service.set_props(
            progress::Props::new()
                .id("upload")
                .value(Some(50.0))
                .min(10.0)
                .max(90.0)
                .orientation(progress::Orientation::Vertical),
        ),
    );

    let ctx = service.context();

    assert_eq!(ctx.value.get(), &Some(50.0));
    assert_eq!(ctx.min, 10.0);
    assert_eq!(ctx.max, 90.0);
    assert_eq!(ctx.orientation, progress::Orientation::Vertical);
    assert_eq!(ctx.percent, 50.0);
}

#[test]
fn progress_set_props_preserves_value_control_mode_transitions() {
    let mut becomes_controlled =
        progress_service(progress::Props::new().id("upload").default_value(20.0));

    assert!(!becomes_controlled.context().value.is_controlled());

    drop(becomes_controlled.set_props(progress::Props::new().id("upload").value(Some(50.0))));

    assert!(becomes_controlled.context().value.is_controlled());
    assert_eq!(becomes_controlled.context().value.get(), &Some(50.0));

    drop(becomes_controlled.send(progress::Event::Reset));

    assert!(becomes_controlled.context().value.is_controlled());
    assert_eq!(becomes_controlled.context().value.get(), &Some(50.0));
    assert!(!becomes_controlled.context().indeterminate);

    let mut becomes_uncontrolled =
        progress_service(progress::Props::new().id("upload").value(Some(50.0)));

    assert!(becomes_uncontrolled.context().value.is_controlled());

    drop(
        becomes_uncontrolled.set_props(
            progress::Props::new()
                .id("upload")
                .default_value(15.0)
                .max(100.0),
        ),
    );

    assert!(!becomes_uncontrolled.context().value.is_controlled());
    assert_eq!(becomes_uncontrolled.context().value.get(), &Some(15.0));

    drop(becomes_uncontrolled.send(progress::Event::SetValue(Some(40.0))));

    assert_eq!(becomes_uncontrolled.context().value.get(), &Some(40.0));

    drop(
        becomes_uncontrolled.set_props(
            progress::Props::new()
                .id("upload")
                .default_value(15.0)
                .max(200.0),
        ),
    );

    assert!(!becomes_uncontrolled.context().value.is_controlled());
    assert_eq!(becomes_uncontrolled.context().value.get(), &Some(40.0));
    assert_eq!(becomes_uncontrolled.context().percent, 20.0);
}

#[test]
fn progress_complete_event_sets_public_completion_consistently() {
    let mut controlled = progress_service(progress::Props::new().id("upload").value(Some(50.0)));

    drop(controlled.send(progress::Event::Complete));

    let controlled_api = controlled.connect(&|_| {});

    assert_eq!(controlled.state(), &progress::State::Complete);
    assert_eq!(controlled.context().value.get(), &Some(100.0));
    assert_eq!(controlled_api.percent(), 100.0);
    assert_eq!(controlled_api.value_text(), "Complete");
    assert_eq!(
        controlled_api
            .root_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("100")
    );

    let mut controlled_indeterminate =
        progress_service(progress::Props::new().id("upload").value(None));

    drop(controlled_indeterminate.send(progress::Event::Complete));

    let controlled_indeterminate_api = controlled_indeterminate.connect(&|_| {});

    assert_eq!(controlled_indeterminate.state(), &progress::State::Complete);
    assert!(!controlled_indeterminate_api.is_indeterminate());
    assert_eq!(
        controlled_indeterminate_api
            .root_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("100")
    );

    let mut invalid_bounds = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(50.0)
            .min(100.0)
            .max(0.0),
    );

    drop(invalid_bounds.send(progress::Event::Complete));

    let invalid_api = invalid_bounds.connect(&|_| {});

    assert_eq!(invalid_bounds.state(), &progress::State::Idle);
    assert_eq!(invalid_api.percent(), 0.0);
    assert_eq!(invalid_api.value_text(), "0% complete");
    assert_eq!(
        invalid_api
            .root_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("0")
    );
}

#[test]
fn progress_part_equality_and_hash_include_circle_radius() {
    assert_ne!(progress::Part::Root, progress::Part::Label);
    assert_ne!(
        progress::Part::CircleRange { radius: 10.0 },
        progress::Part::CircleRange { radius: 20.0 }
    );
    assert_ne!(
        hash_progress_part(&progress::Part::Root),
        hash_progress_part(&progress::Part::Label)
    );
    assert_ne!(
        hash_progress_part(&progress::Part::CircleRange { radius: 10.0 }),
        hash_progress_part(&progress::Part::CircleRange { radius: 20.0 })
    );
}

#[test]
fn progress_api_set_value_dispatches_event() {
    let service = progress_service(progress::Props::new().id("upload"));

    let sent = std::cell::RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    api.set_value(Some(42.0));

    assert_eq!(*sent.borrow(), vec![progress::Event::SetValue(Some(42.0))]);
}

#[test]
fn progress_events_update_value_max_completion_reset_and_orientation() {
    let mut service = progress_service(
        progress::Props::new()
            .id("upload")
            .default_value(20.0)
            .orientation(progress::Orientation::Vertical),
    );

    let root = service.connect(&|_| {}).root_attrs();

    assert_eq!(
        root.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
        Some("vertical")
    );
    assert_eq!(
        root.get(&HtmlAttr::Data("ars-orientation")),
        Some("vertical")
    );

    drop(service.send(progress::Event::SetMax(200.0)));
    drop(service.send(progress::Event::SetValue(Some(50.0))));

    assert_eq!(service.context().max, 200.0);
    assert_eq!(service.context().percent, 25.0);

    drop(service.send(progress::Event::Complete));

    assert_eq!(service.state(), &progress::State::Complete);
    assert_eq!(service.context().value.get(), &Some(200.0));
    assert_eq!(service.context().percent, 100.0);

    drop(service.send(progress::Event::Reset));

    assert_eq!(service.state(), &progress::State::Idle);
    assert!(service.context().indeterminate);
    assert_eq!(service.context().percent, 0.0);
}

#[test]
fn progress_range_value_text_and_circle_attrs_are_derived() {
    let service = progress_service(progress::Props::new().id("upload").default_value(25.0));

    let api = service.connect(&|_| {});

    assert_eq!(api.percent(), 25.0);
    assert_eq!(api.value_text(), "25% complete");
    assert!(
        api.range_attrs()
            .styles()
            .contains(&(CssProperty::Width, String::from("25%")))
    );
    assert_eq!(
        api.range_attrs().get(&HtmlAttr::Data("ars-indeterminate")),
        Some("false")
    );

    let circle = api.circle_range_attrs(10.0);

    assert!(circle.get(&HtmlAttr::StrokeDasharray).is_some());
    assert!(circle.get(&HtmlAttr::StrokeDashoffset).is_some());
}

#[test]
fn progress_determinate_value_text_comes_from_messages() {
    let messages = progress::Messages {
        determinate: ars_core::MessageFn::new(std::sync::Arc::new(
            |percent: &str, _locale: &ars_i18n::Locale| format!("{percent} uploaded"),
        )
            as std::sync::Arc<progress::DeterminateTextFn>),
        ..progress::Messages::default()
    };
    let service = Service::<progress::Machine>::new(
        progress::Props::new().id("upload").default_value(25.0),
        &Env::default(),
        &messages,
    );

    assert_eq!(service.connect(&|_| {}).value_text(), "25% uploaded");
}

#[test]
fn progress_part_attrs_delegates_for_all_parts() {
    let service = progress_service(progress::Props::new().id("upload").default_value(25.0));

    let api = service.connect(&|_| {});

    assert_eq!(api.part_attrs(progress::Part::Root), api.root_attrs());
    assert_eq!(api.part_attrs(progress::Part::Label), api.label_attrs());
    assert_eq!(api.part_attrs(progress::Part::Track), api.track_attrs());
    assert_eq!(api.part_attrs(progress::Part::Range), api.range_attrs());
    assert_eq!(
        api.part_attrs(progress::Part::ValueText),
        api.value_text_attrs()
    );
    assert_eq!(
        api.part_attrs(progress::Part::CircleTrack),
        api.circle_track_attrs()
    );
    assert_eq!(
        api.part_attrs(progress::Part::CircleRange { radius: 10.0 }),
        api.circle_range_attrs(10.0)
    );
}
