//! Meter component connect API.
//!
//! `Meter` is a stateless, framework-agnostic attribute mapper for scalar
//! measurements with HTML meter threshold semantics.

use alloc::{format, string::String, sync::Arc};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Env, HtmlAttr,
    MessageFn,
};
use ars_i18n::{Locale, number};

type ValueTextFn = dyn Fn(f64, f64, f64, &Locale, &number::FormatOptions) -> String + Send + Sync;

/// Props for the Meter component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Current value.
    pub value: f64,

    /// Lower bound.
    pub min: f64,

    /// Upper bound.
    pub max: f64,

    /// Threshold below which the measurement is considered low.
    pub low: Option<f64>,

    /// Threshold above which the measurement is considered high.
    pub high: Option<f64>,

    /// The value considered optimal.
    pub optimum: Option<f64>,

    /// Format options for locale-aware value text.
    pub format_options: Option<number::FormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            low: None,
            high: None,
            optimum: None,
            format_options: None,
        }
    }
}

impl Props {
    /// Returns fresh meter props with the documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the current value.
    #[must_use]
    pub const fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Sets the lower bound.
    #[must_use]
    pub const fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Sets the upper bound.
    #[must_use]
    pub const fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Sets the low threshold.
    #[must_use]
    pub const fn low(mut self, low: f64) -> Self {
        self.low = Some(low);
        self
    }

    /// Sets the high threshold.
    #[must_use]
    pub const fn high(mut self, high: f64) -> Self {
        self.high = Some(high);
        self
    }

    /// Sets the optimum value.
    #[must_use]
    pub const fn optimum(mut self, optimum: f64) -> Self {
        self.optimum = Some(optimum);
        self
    }

    /// Sets locale-aware number formatting options.
    #[must_use]
    pub fn format_options(mut self, options: number::FormatOptions) -> Self {
        self.format_options = Some(options);
        self
    }
}

/// Semantic classification mirroring the HTML `<meter>` algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    /// Value is in the optimal range.
    Optimal,

    /// Value is sub-optimal but not the worst range.
    SubOptimal,

    /// Value is in the worst range.
    SubSubOptimal,
}

impl Segment {
    /// Returns the `data-ars-segment` value for this segment.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optimal => "optimal",
            Self::SubOptimal => "sub-optimal",
            Self::SubSubOptimal => "sub-sub-optimal",
        }
    }
}

/// Semantic zone classification for assistive announcements and styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    /// Value is in the optimal range.
    Optimal,

    /// Value is sub-optimal but not critical.
    SubOptimal,

    /// Value is in the critical range.
    Critical,
}

impl Zone {
    /// Derives the zone from the current meter segment.
    #[must_use]
    pub const fn from_segment(segment: &Segment) -> Self {
        match segment {
            Segment::Optimal => Self::Optimal,
            Segment::SubOptimal => Self::SubOptimal,
            Segment::SubSubOptimal => Self::Critical,
        }
    }

    /// Returns the `data-ars-zone` value for this zone.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optimal => "optimal",
            Self::SubOptimal => "sub-optimal",
            Self::Critical => "critical",
        }
    }
}

/// Computes the semantic segment for the given meter parameters.
#[must_use]
pub fn compute_segment(
    value: f64,
    min: f64,
    max: f64,
    low: Option<f64>,
    high: Option<f64>,
    optimum: Option<f64>,
) -> Segment {
    let (min, max) = normalize_bounds(min, max);
    let value = clamp_value(value, min, max);
    let (low, high, optimum) = normalize_thresholds(min, max, low, high, optimum);

    if optimum < low {
        if value < low {
            Segment::Optimal
        } else if value <= high {
            Segment::SubOptimal
        } else {
            Segment::SubSubOptimal
        }
    } else if optimum > high {
        if value > high {
            Segment::Optimal
        } else if value >= low {
            Segment::SubOptimal
        } else {
            Segment::SubSubOptimal
        }
    } else if value >= low && value <= high {
        Segment::Optimal
    } else {
        Segment::SubOptimal
    }
}

/// Computes the fill percentage for the given value within `[min, max]`.
#[must_use]
pub fn compute_percent(value: f64, min: f64, max: f64) -> f64 {
    if !valid_bounds(min, max) {
        return 0.0;
    }

    let (min, max) = normalize_bounds(min, max);

    if !value.is_finite() {
        return 0.0;
    }

    ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
}

/// Messages for the Meter component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Formats the meter value for display and screen readers.
    pub value_text: MessageFn<ValueTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            value_text: MessageFn::new(Arc::new(
                |value: f64,
                 min: f64,
                 max: f64,
                 locale: &Locale,
                 options: &number::FormatOptions| {
                    let formatter = number::Formatter::new(locale, options.clone());
                    formatter.format_percent(
                        compute_percent(value, min, max) / 100.0,
                        Some(options.max_fraction_digits),
                    )
                },
            ) as Arc<ValueTextFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the Meter connect API.
#[derive(ComponentPart)]
#[scope = "meter"]
pub enum Part {
    /// The root meter element.
    Root,

    /// The visible label element.
    Label,

    /// The visual track element.
    Track,

    /// The filled range element.
    Range,

    /// The visible formatted value element.
    ValueText,
}

/// API for the Meter component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("meter::Api")
            .field("props", &self.props)
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Api {
    /// Creates a new API from props and environment values.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns the semantic segment for the current value.
    #[must_use]
    pub fn segment(&self) -> Segment {
        let (min, max) = normalize_bounds(self.props.min, self.props.max);

        compute_segment(
            clamp_value(self.props.value, min, max),
            min,
            max,
            self.props.low,
            self.props.high,
            self.props.optimum,
        )
    }

    /// Returns the semantic zone for the current value.
    #[must_use]
    pub fn zone(&self) -> Zone {
        Zone::from_segment(&self.segment())
    }

    /// Returns the fill percentage for the current value.
    #[must_use]
    pub fn percent(&self) -> f64 {
        let (min, max) = normalize_bounds(self.props.min, self.props.max);

        compute_percent(clamp_value(self.props.value, min, max), min, max)
    }

    /// Returns the locale-aware value text.
    #[must_use]
    pub fn value_text(&self) -> String {
        let (min, max) = normalize_bounds(self.props.min, self.props.max);
        let value = clamp_value(self.props.value, min, max);

        (self.messages.value_text)(
            value,
            min,
            max,
            &self.locale,
            &self.props.format_options.clone().unwrap_or_default(),
        )
    }

    /// Returns root attributes for the meter.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        let segment = self.segment();
        let zone = Zone::from_segment(&segment);
        let (min, max) = normalize_bounds(self.props.min, self.props.max);
        let value = clamp_value(self.props.value, min, max);
        let (low, high, optimum) = normalize_thresholds(
            min,
            max,
            self.props.low,
            self.props.high,
            self.props.optimum,
        );

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.props.id.clone())
            .set(HtmlAttr::Role, "meter")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                label_id(&self.props.id),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), max.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueText), self.value_text())
            .set(HtmlAttr::Value, value.to_string())
            .set(HtmlAttr::Min, min.to_string())
            .set(HtmlAttr::Max, max.to_string())
            .set(HtmlAttr::Data("ars-segment"), segment.as_str())
            .set(HtmlAttr::Data("ars-zone"), zone.as_str());

        if self.props.low.is_some() {
            attrs.set(HtmlAttr::Low, low.to_string());
        }

        if self.props.high.is_some() {
            attrs.set(HtmlAttr::High, high.to_string());
        }

        if self.props.optimum.is_some() {
            attrs.set(HtmlAttr::Optimum, optimum.to_string());
        }

        attrs
    }

    /// Returns label attributes for the meter.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Label);

        attrs
            .set(HtmlAttr::Id, label_id(&self.props.id))
            .set(HtmlAttr::For, self.props.id.clone());

        attrs
    }

    /// Returns track attributes for the meter.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        part_attrs(&Part::Track)
    }

    /// Returns range attributes for the meter.
    #[must_use]
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Range);

        attrs.set_style(CssProperty::Width, format!("{}%", self.percent()));

        attrs
    }

    /// Returns value text attributes for the meter.
    #[must_use]
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ValueText);

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::ValueText => self.value_text_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn label_id(id: &str) -> String {
    format!("{id}-label")
}

const fn normalize_bounds(min: f64, max: f64) -> (f64, f64) {
    if valid_bounds(min, max) {
        (min, max)
    } else {
        (0.0, 100.0)
    }
}

const fn valid_bounds(min: f64, max: f64) -> bool {
    min.is_finite() && max.is_finite() && min < max
}

const fn clamp_value(value: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

fn normalize_thresholds(
    min: f64,
    max: f64,
    low: Option<f64>,
    high: Option<f64>,
    optimum: Option<f64>,
) -> (f64, f64, f64) {
    let low = low.map_or(min, |low| clamp_value(low, min, max));
    let high = high.map_or(max, |high| clamp_value(high, min, max).max(low));
    let optimum = optimum.map_or((min + max) / 2.0, |optimum| clamp_value(optimum, min, max));

    (low, high, optimum)
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    fn api(props: Props) -> Api {
        Api::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn meter_root_default_snapshot() {
        assert_snapshot!(
            "meter_root_default",
            snapshot_attrs(&api(Props::new().id("meter").value(50.0)).root_attrs())
        );
    }

    #[test]
    fn meter_root_threshold_critical_snapshot() {
        assert_snapshot!(
            "meter_root_threshold_critical",
            snapshot_attrs(
                &api(Props::new()
                    .id("meter")
                    .value(90.0)
                    .low(20.0)
                    .high(80.0)
                    .optimum(10.0))
                .root_attrs()
            )
        );
    }

    #[test]
    fn meter_range_snapshot() {
        assert_snapshot!(
            "meter_range",
            snapshot_attrs(&api(Props::new().id("meter").value(40.0)).range_attrs())
        );
    }

    #[test]
    fn meter_value_text_snapshot() {
        assert_snapshot!(
            "meter_value_text",
            snapshot_attrs(&api(Props::new().id("meter").value(40.0)).value_text_attrs())
        );
    }

    #[test]
    fn meter_sanitizes_non_finite_and_out_of_range_values() {
        let high = api(Props::new().id("meter").value(150.0).max(100.0));
        let high_root = high.root_attrs();

        assert_eq!(
            high_root.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("100")
        );
        assert_eq!(high_root.get(&HtmlAttr::Value), Some("100"));
        assert_eq!(high.percent(), 100.0);

        let non_finite = api(Props::new().id("meter").value(f64::NAN));
        let non_finite_root = non_finite.root_attrs();

        assert_eq!(
            non_finite_root.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("0")
        );
        assert_eq!(non_finite_root.get(&HtmlAttr::Value), Some("0"));
        assert_eq!(compute_percent(f64::INFINITY, 0.0, 100.0), 0.0);
        assert_eq!(compute_percent(50.0, 100.0, 100.0), 0.0);

        let invalid_bounds = api(Props::new().id("meter").value(50.0).min(100.0).max(0.0));

        assert_eq!(invalid_bounds.value_text(), "50%");
        assert_eq!(
            invalid_bounds
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("50%")
        );
        assert!(
            invalid_bounds
                .range_attrs()
                .styles()
                .contains(&(CssProperty::Width, String::from("50%")))
        );
    }

    #[test]
    fn meter_normalizes_thresholds_for_segment_classification() {
        assert_eq!(
            compute_segment(50.0, 0.0, 100.0, Some(90.0), Some(10.0), Some(95.0)),
            Segment::SubSubOptimal
        );
    }
}
