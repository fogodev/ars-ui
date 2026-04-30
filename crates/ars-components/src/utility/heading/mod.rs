//! `Heading` component connect API.
//!
//! `Heading` is a stateless, framework-agnostic attribute mapper for semantic
//! headings. It resolves a heading level from explicit props or from
//! [`HeadingContext`], and emits fallback ARIA attributes when an adapter cannot
//! render a native `<h1>` through `<h6>` element.

use alloc::string::{String, ToString as _};
use core::fmt::{self, Display};

use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};

/// Props for the `Heading` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Explicit heading level override. When absent, the nearest
    /// [`HeadingContext`] supplies the resolved level.
    pub level: Option<Level>,
}

impl Props {
    /// Returns fresh heading props with the documented defaults.
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

    /// Sets an explicit heading level override.
    #[must_use]
    pub const fn level(mut self, level: Level) -> Self {
        self.level = Some(level);
        self
    }

    /// Clears the explicit heading level so context drives resolution.
    #[must_use]
    pub const fn auto_level(mut self) -> Self {
        self.level = None;
        self
    }
}

/// The resolved semantic level for a `Heading`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Level {
    /// First-level heading.
    One = 1,

    /// Second-level heading.
    Two = 2,

    /// Third-level heading.
    Three = 3,

    /// Fourth-level heading.
    Four = 4,

    /// Fifth-level heading.
    Five = 5,

    /// Sixth-level heading, the maximum HTML heading level.
    Six = 6,
}

impl Level {
    /// Converts a numeric level into a [`Level`], clamping values below one to
    /// [`Level::One`] and values above six to [`Level::Six`].
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 | 1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            4 => Self::Four,
            5 => Self::Five,
            _ => Self::Six,
        }
    }

    /// Returns this heading level as its numeric `1..=6` value.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_u8().to_string())
    }
}

/// Context that tracks the current heading level for nested sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadingContext {
    /// The current heading level, clamped to HTML's `1..=6` range.
    pub level: Level,
}

impl HeadingContext {
    /// Creates a heading context starting at [`Level::One`].
    #[must_use]
    pub const fn new() -> Self {
        Self { level: Level::One }
    }

    /// Creates a heading context from an explicit level.
    #[must_use]
    pub const fn from_level(level: Level) -> Self {
        Self { level }
    }

    /// Returns the current heading level.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Returns a context incremented by one level, capped at [`Level::Six`].
    #[must_use]
    pub const fn incremented(&self) -> Self {
        Self {
            level: Level::from_u8(self.level.as_u8() + 1),
        }
    }
}

impl Default for HeadingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Context-only `HeadingLevelProvider` support.
pub mod heading_level_provider {
    use super::{HeadingContext, Level};

    /// Props for the `HeadingLevelProvider` context wrapper.
    ///
    /// The provider renders no DOM of its own; adapters use this value to
    /// publish a starting [`HeadingContext`] to descendant headings.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Props {
        /// Starting heading level to provide to descendants.
        pub level: Level,
    }

    impl Default for Props {
        fn default() -> Self {
            Self { level: Level::One }
        }
    }

    impl Props {
        /// Returns fresh provider props starting at [`Level::One`].
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Sets the starting heading level to provide to descendants.
        #[must_use]
        pub const fn level(mut self, level: Level) -> Self {
            self.level = level;
            self
        }
    }

    /// Returns the heading context descendants should receive from this
    /// provider.
    #[must_use]
    pub const fn context_for(props: &Props) -> HeadingContext {
        HeadingContext::from_level(props.level)
    }
}

/// Logical `Section` wrapper support for heading level nesting.
pub mod section {
    use super::HeadingContext;

    /// Props for the `Section` context wrapper.
    ///
    /// The agnostic core stores no DOM-facing options because `Section`
    /// exists only to publish an incremented [`HeadingContext`] to descendants.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Props;

    impl Props {
        /// Returns fresh `Section` props.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }
    }

    /// Returns the heading context descendants should receive for this section.
    #[must_use]
    pub const fn context_for(parent: &HeadingContext) -> HeadingContext {
        parent.incremented()
    }
}

/// Structural parts exposed by the `Heading` connect API.
#[derive(ComponentPart)]
#[scope = "heading"]
pub enum Part {
    /// The root heading element.
    Root,
}

/// API for the `Heading` component.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
    ctx: HeadingContext,
}

impl Api {
    /// Creates a new API from heading props and the nearest heading context.
    #[must_use]
    pub const fn new(props: Props, ctx: HeadingContext) -> Self {
        Self { props, ctx }
    }

    /// Returns the underlying heading props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the heading context used by this API.
    #[must_use]
    pub const fn context(&self) -> &HeadingContext {
        &self.ctx
    }

    /// Returns the component instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns the resolved heading level, preferring the explicit prop over
    /// the inherited context.
    #[must_use]
    pub const fn resolved_level(&self) -> Level {
        if let Some(level) = self.props.level {
            level
        } else {
            self.ctx.level
        }
    }

    /// Returns root attributes for either a native heading element or an ARIA
    /// fallback element.
    ///
    /// Native `<h1>` through `<h6>` elements already expose the heading role and
    /// level, so `role` and `aria-level` are only emitted for non-semantic
    /// fallback elements.
    #[must_use]
    pub fn root_attrs(&self, is_native_heading_element: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.props.id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if !is_native_heading_element {
            attrs.set(HtmlAttr::Role, "heading").set(
                HtmlAttr::Aria(AriaAttr::Level),
                self.resolved_level().to_string(),
            );
        }

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrMap, ConnectApi, HasId, HtmlAttr};
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn level_from_u8_clamps_to_one_through_six() {
        assert_eq!(Level::from_u8(0), Level::One);
        assert_eq!(Level::from_u8(1), Level::One);
        assert_eq!(Level::from_u8(2), Level::Two);
        assert_eq!(Level::from_u8(3), Level::Three);
        assert_eq!(Level::from_u8(4), Level::Four);
        assert_eq!(Level::from_u8(5), Level::Five);
        assert_eq!(Level::from_u8(6), Level::Six);
        assert_eq!(Level::from_u8(7), Level::Six);
        assert_eq!(Level::Three.as_u8(), 3);
        assert_eq!(Level::Three.to_string(), "3");
    }

    #[test]
    fn heading_context_defaults_to_level_one() {
        let ctx = HeadingContext::new();

        assert_eq!(ctx.level, Level::One);
        assert_eq!(ctx.level(), Level::One);
        assert_eq!(HeadingContext::default(), ctx);
    }

    #[test]
    fn heading_context_incremented_caps_at_six() {
        let ctx = HeadingContext::from_level(Level::Five);

        assert_eq!(ctx.incremented().level(), Level::Six);
        assert_eq!(ctx.incremented().incremented().level(), Level::Six);
    }

    #[test]
    fn section_context_for_increments_parent_level() {
        let parent = HeadingContext::from_level(Level::Two);

        assert_eq!(section::Props::new(), section::Props);
        assert_eq!(section::context_for(&parent).level(), Level::Three);
    }

    #[test]
    fn heading_level_provider_context_uses_explicit_starting_level() {
        let props = heading_level_provider::Props::new().level(Level::Four);

        let context = heading_level_provider::context_for(&props);

        assert_eq!(props.level, Level::Four);
        assert_eq!(context.level(), Level::Four);
        assert_eq!(
            heading_level_provider::context_for(&heading_level_provider::Props::default()).level(),
            Level::One
        );
    }

    #[test]
    fn props_builder_round_trips_all_fields() {
        let props = Props::new().id("heading-1").level(Level::Four);

        assert_eq!(props.id, "heading-1");
        assert_eq!(props.level, Some(Level::Four));
        assert_eq!(props.auto_level().level, None);
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        let mut props = Props::new().with_id(String::from("heading-a"));

        assert_eq!(HasId::id(&props), "heading-a");

        props.set_id(String::from("heading-b"));

        assert_eq!(HasId::id(&props), "heading-b");
    }

    #[test]
    fn api_resolved_level_uses_context_when_level_is_none() {
        let api = Api::new(Props::new(), HeadingContext::from_level(Level::Three));

        assert_eq!(api.resolved_level(), Level::Three);
        assert_eq!(api.context().level(), Level::Three);
    }

    #[test]
    fn api_resolved_level_prefers_explicit_level() {
        let api = Api::new(
            Props::new().id("heading-explicit").level(Level::Five),
            HeadingContext::from_level(Level::Two),
        );

        assert_eq!(api.id(), "heading-explicit");
        assert_eq!(api.props().level, Some(Level::Five));
        assert_eq!(api.resolved_level(), Level::Five);
    }

    #[test]
    fn heading_root_native_level_one() {
        let api = Api::new(Props::new().id("heading-native"), HeadingContext::new());

        assert_snapshot!(
            "heading_root_native_level_one",
            snapshot_attrs(&api.root_attrs(true))
        );
    }

    #[test]
    fn heading_root_native_nested_level_three() {
        let api = Api::new(
            Props::new().id("heading-nested"),
            HeadingContext::from_level(Level::Three),
        );

        assert_snapshot!(
            "heading_root_native_nested_level_three",
            snapshot_attrs(&api.root_attrs(true))
        );
    }

    #[test]
    fn heading_root_fallback_level_three() {
        let api = Api::new(
            Props::new().id("heading-fallback"),
            HeadingContext::from_level(Level::Three),
        );

        assert_snapshot!(
            "heading_root_fallback_level_three",
            snapshot_attrs(&api.root_attrs(false))
        );
    }

    #[test]
    fn heading_root_fallback_explicit_level_overrides_context() {
        let api = Api::new(
            Props::new().id("heading-override").level(Level::Five),
            HeadingContext::from_level(Level::Two),
        );

        assert_snapshot!(
            "heading_root_fallback_explicit_level_overrides_context",
            snapshot_attrs(&api.root_attrs(false))
        );
    }

    #[test]
    fn connect_api_root_uses_native_heading_attrs() {
        let api = Api::new(Props::new().id("heading-connect"), HeadingContext::new());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("heading-connect"));
        assert_eq!(attrs.get(&HtmlAttr::Role), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Level)), None);
    }
}
