//! Stat component connect API.
//!
//! `Stat` is a stateless, framework-agnostic attribute mapper for compact
//! metric summaries with optional trend metadata.

use alloc::{format, string::String, sync::Arc};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, MessageFn,
};
use ars_i18n::{Locale, number};

type PrefixFn = dyn Fn(&Locale) -> String + Send + Sync;
type ChangeLabelFn = dyn Fn(f64, Trend, &Locale) -> String + Send + Sync;

/// Trend direction for the Stat component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trend {
    /// The value is increasing.
    Up,

    /// The value is decreasing.
    Down,

    /// The value is neutral.
    Neutral,
}

impl Trend {
    /// Returns the `data-ars-trend` value for this trend.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Neutral => "neutral",
        }
    }
}

/// Props for the Stat component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The formatted metric value.
    pub value: String,

    /// The metric label.
    pub label: String,

    /// Optional change delta as a percentage.
    pub change: Option<f64>,

    /// Override trend direction.
    pub trend: Option<Trend>,

    /// Optional supplementary description.
    pub help_text: Option<String>,

    /// Whether the stat is loading.
    pub loading: bool,

    /// Formatting options used for the change delta.
    pub format_options: Option<number::FormatOptions>,
}

impl Props {
    /// Returns fresh stat props with the documented defaults.
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

    /// Sets the formatted metric value.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    /// Sets the metric label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the optional change delta percentage.
    #[must_use]
    pub const fn change(mut self, change: f64) -> Self {
        self.change = Some(change);
        self
    }

    /// Sets the explicit trend direction.
    #[must_use]
    pub const fn trend(mut self, trend: Trend) -> Self {
        self.trend = Some(trend);
        self
    }

    /// Sets the supplementary help text.
    #[must_use]
    pub fn help_text(mut self, help_text: impl Into<String>) -> Self {
        self.help_text = Some(help_text.into());
        self
    }

    /// Sets the loading state.
    #[must_use]
    pub const fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets locale-aware number formatting options for the change delta.
    #[must_use]
    pub fn format_options(mut self, options: number::FormatOptions) -> Self {
        self.format_options = Some(options);
        self
    }
}

/// Messages for the Stat component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Prefix for positive change display.
    pub increase_prefix: MessageFn<PrefixFn>,

    /// Prefix for negative change display.
    pub decrease_prefix: MessageFn<PrefixFn>,

    /// Locale-aware full accessible label for a change.
    pub change_label: MessageFn<ChangeLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            increase_prefix: MessageFn::static_str("↑"),
            decrease_prefix: MessageFn::static_str("↓"),
            change_label: MessageFn::new(Arc::new(|pct: f64, trend: Trend, _locale: &Locale| {
                let suffix = match trend {
                    Trend::Up => "increase",
                    Trend::Down => "decrease",
                    Trend::Neutral => "no change",
                };
                format!("{pct:.1}% {suffix}")
            }) as Arc<ChangeLabelFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the Stat connect API.
#[derive(ComponentPart)]
#[scope = "stat"]
pub enum Part {
    /// The root stat group.
    Root,

    /// The metric label.
    Label,

    /// The metric value.
    Value,

    /// The optional change delta.
    Change,

    /// The decorative trend indicator.
    TrendIndicator,

    /// The optional help text.
    HelpText,
}

/// API for the Stat component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("stat::Api")
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

    /// Derives trend from the change value unless explicitly set.
    #[must_use]
    pub fn resolved_trend(&self) -> Option<Trend> {
        self.props.trend.or_else(|| {
            self.props.change.map(|change| {
                if change > 0.0 {
                    Trend::Up
                } else if change < 0.0 {
                    Trend::Down
                } else {
                    Trend::Neutral
                }
            })
        })
    }

    /// Formats the change delta for display.
    #[must_use]
    pub fn formatted_change(&self) -> Option<String> {
        let change = self.props.change?;

        let formatter = number::Formatter::new(
            &self.locale,
            self.props.format_options.clone().unwrap_or_default(),
        );

        let percent = formatter.format_percent(change.abs() / 100.0, Some(1));

        match self.resolved_trend() {
            Some(Trend::Up) => Some(format!(
                "{} {}",
                (self.messages.increase_prefix)(&self.locale),
                percent
            )),

            Some(Trend::Down) => Some(format!(
                "{} {}",
                (self.messages.decrease_prefix)(&self.locale),
                percent
            )),

            Some(Trend::Neutral) | None => Some(percent),
        }
    }

    /// Returns the screen-reader-friendly change announcement.
    #[must_use]
    pub fn change_aria_label(&self) -> Option<String> {
        let change = self.props.change?;
        let trend = self.resolved_trend().unwrap_or(Trend::Neutral);

        Some((self.messages.change_label)(
            change.abs(),
            trend,
            &self.locale,
        ))
    }

    /// Returns root attributes for the stat.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, self.props.id.clone())
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                format!("{}: {}", self.props.label, self.props.value),
            );

        if self.props.loading {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Busy), "true")
                .set_bool(HtmlAttr::Data("ars-loading"), true);
        }

        attrs
    }

    /// Returns label attributes for the stat.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        part_attrs(&Part::Label)
    }

    /// Returns value attributes for the stat.
    #[must_use]
    pub fn value_attrs(&self) -> AttrMap {
        part_attrs(&Part::Value)
    }

    /// Returns change attributes for the stat.
    #[must_use]
    pub fn change_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Change);

        if let Some(trend) = self.resolved_trend() {
            attrs.set(HtmlAttr::Data("ars-trend"), trend.as_str());
        }

        if let Some(label) = self.change_aria_label() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }

        attrs
    }

    /// Returns trend indicator attributes for the stat.
    #[must_use]
    pub fn trend_indicator_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::TrendIndicator);

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(trend) = self.resolved_trend() {
            attrs.set(HtmlAttr::Data("ars-trend"), trend.as_str());
        }

        attrs
    }

    /// Returns help text attributes for the stat.
    #[must_use]
    pub fn help_text_attrs(&self) -> AttrMap {
        part_attrs(&Part::HelpText)
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Value => self.value_attrs(),
            Part::Change => self.change_attrs(),
            Part::TrendIndicator => self.trend_indicator_attrs(),
            Part::HelpText => self.help_text_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
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
    fn stat_props_builders_write_all_fields() {
        let options = number::FormatOptions {
            min_fraction_digits: 1,
            max_fraction_digits: 1,
            ..number::FormatOptions::default()
        };

        let props = Props::new()
            .id("revenue")
            .label("Revenue")
            .value("$42k")
            .change(-4.5)
            .trend(Trend::Down)
            .help_text("Trailing 30 days")
            .loading(true)
            .format_options(options.clone());

        assert_eq!(props.id, "revenue");
        assert_eq!(props.label, "Revenue");
        assert_eq!(props.value, "$42k");
        assert_eq!(props.change, Some(-4.5));
        assert_eq!(props.trend, Some(Trend::Down));
        assert_eq!(props.help_text, Some("Trailing 30 days".to_string()));
        assert!(props.loading);
        assert_eq!(props.format_options, Some(options));
    }

    #[test]
    fn stat_root_and_structural_parts_match_contract() {
        let api = api(Props::new()
            .id("revenue")
            .label("Total Revenue")
            .value("$45,231")
            .loading(true)
            .help_text("Trailing 30 days"));

        let root = api.root_attrs();

        assert_eq!(root.get(&HtmlAttr::Role), Some("group"));
        assert_eq!(root.get(&HtmlAttr::Id), Some("revenue"));
        assert_eq!(
            root.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Total Revenue: $45,231")
        );
        assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
        assert_eq!(root.get(&HtmlAttr::Data("ars-loading")), Some("true"));
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Value), api.value_attrs());
        assert_eq!(api.part_attrs(Part::HelpText), api.help_text_attrs());
    }

    #[test]
    fn stat_derives_and_exposes_trend_direction() {
        let up = api(Props::new().id("stat").change(12.5));

        assert_eq!(up.resolved_trend(), Some(Trend::Up));
        assert_eq!(
            up.change_attrs().get(&HtmlAttr::Data("ars-trend")),
            Some("up")
        );
        assert_eq!(
            up.trend_indicator_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Hidden)),
            Some("true")
        );

        let down = api(Props::new().id("stat").change(-3.2));

        assert_eq!(down.resolved_trend(), Some(Trend::Down));
        assert_eq!(
            down.change_attrs().get(&HtmlAttr::Data("ars-trend")),
            Some("down")
        );

        let neutral = api(Props::new().id("stat").change(0.0));

        assert_eq!(neutral.resolved_trend(), Some(Trend::Neutral));
        assert_eq!(
            neutral.change_attrs().get(&HtmlAttr::Data("ars-trend")),
            Some("neutral")
        );

        let override_trend = api(Props::new().id("stat").change(12.5).trend(Trend::Down));

        assert_eq!(override_trend.resolved_trend(), Some(Trend::Down));
    }

    #[test]
    fn stat_formats_change_and_accessible_label() {
        let api = api(Props::new().id("stat").change(12.5));

        assert_eq!(api.formatted_change(), Some("↑ 12.5%".to_string()));
        assert_eq!(
            api.change_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("12.5% increase")
        );
    }

    #[test]
    fn stat_root_default_snapshot() {
        assert_snapshot!(
            "stat_root_default",
            snapshot_attrs(
                &api(Props::new()
                    .id("stat")
                    .label("Total Revenue")
                    .value("$45,231"))
                .root_attrs()
            )
        );
    }

    #[test]
    fn stat_root_loading_snapshot() {
        assert_snapshot!(
            "stat_root_loading",
            snapshot_attrs(
                &api(Props::new()
                    .id("stat")
                    .label("Total Revenue")
                    .value("$45,231")
                    .loading(true))
                .root_attrs()
            )
        );
    }

    #[test]
    fn stat_structural_parts_snapshot() {
        let api = api(Props::new().id("stat"));

        assert_snapshot!("stat_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!("stat_value", snapshot_attrs(&api.value_attrs()));
        assert_snapshot!("stat_help_text", snapshot_attrs(&api.help_text_attrs()));
    }

    #[test]
    fn stat_change_up_snapshot() {
        assert_snapshot!(
            "stat_change_up",
            snapshot_attrs(&api(Props::new().id("stat").change(12.5)).change_attrs())
        );
    }

    #[test]
    fn stat_change_down_snapshot() {
        assert_snapshot!(
            "stat_change_down",
            snapshot_attrs(&api(Props::new().id("stat").change(-3.2)).change_attrs())
        );
    }

    #[test]
    fn stat_trend_indicator_snapshot() {
        assert_snapshot!(
            "stat_trend_indicator",
            snapshot_attrs(&api(Props::new().id("stat").change(12.5)).trend_indicator_attrs())
        );
    }
}
