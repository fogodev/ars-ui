//! Contextual help composition API.
//!
//! `ContextualHelp` is a thin composition layer over the existing
//! [`popover::Machine`](crate::overlay::popover::Machine), providing a
//! pre-wired trigger button, variant-aware labeling, and structured content
//! anatomy. Framework adapters create the popover machine inside the
//! contextual help component so consumers never interact with popover directly.

use alloc::string::String;
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Locale,
    MessageFn,
};
use ars_i18n::Direction;
use ars_interactions::{KeyboardEventData, KeyboardKey};

use crate::overlay::{
    popover,
    positioning::{Placement, PositioningOptions},
};

/// Visual style of the contextual help trigger icon and popover chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// "?" icon — deeper educational guidance, may link to docs.
    #[default]
    Help,

    /// "i" icon — brief, specific, contextual clarification.
    Info,
}

impl Variant {
    /// Returns the `data-ars-variant` token for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Info => "info",
        }
    }
}

/// Immutable configuration for a [`ContextualHelp`](self) instance.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Help or Info variant.
    pub variant: Variant,

    /// Preferred placement relative to the trigger. Default: [`Placement::BottomStart`].
    pub placement: Placement,

    /// Offset along the main axis in pixels.
    pub offset: f64,

    /// Offset along the cross axis in pixels.
    pub cross_offset: f64,

    /// Whether the popover flips when it would overflow.
    pub should_flip: bool,

    /// Padding between the popover and container edges.
    pub container_padding: f64,

    /// Text direction override (inherited from locale if `None`).
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            variant: Variant::Help,
            placement: Placement::BottomStart,
            offset: 0.0,
            cross_offset: 0.0,
            should_flip: true,
            container_padding: 12.0,
            dir: None,
        }
    }
}

impl Props {
    /// Returns fresh contextual help props with documented defaults.
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

    /// Sets the help or info variant.
    #[must_use]
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the preferred popover placement.
    #[must_use]
    pub const fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the main-axis offset in pixels.
    #[must_use]
    pub const fn offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the cross-axis offset in pixels.
    #[must_use]
    pub const fn cross_offset(mut self, cross_offset: f64) -> Self {
        self.cross_offset = cross_offset;
        self
    }

    /// Sets whether the popover may flip when it would overflow.
    #[must_use]
    pub const fn should_flip(mut self, should_flip: bool) -> Self {
        self.should_flip = should_flip;
        self
    }

    /// Sets the padding between the popover and container edges.
    #[must_use]
    pub const fn container_padding(mut self, container_padding: f64) -> Self {
        self.container_padding = container_padding;
        self
    }

    /// Sets the text direction override.
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }

    /// Builds the hardcoded popover configuration for this contextual help
    /// instance.
    #[must_use]
    pub fn popover_props(&self) -> popover::Props {
        popover::Props {
            id: self.id.clone(),
            positioning: PositioningOptions {
                placement: self.placement,
                flip: self.should_flip,
                shift_padding: self.container_padding,
                ..PositioningOptions::default()
            },
            offset: self.offset,
            cross_offset: self.cross_offset,
            ..popover::Props::default()
        }
    }
}

/// Anatomy parts exposed by the contextual help connect API.
#[derive(ComponentPart)]
#[scope = "contextual-help"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger button that toggles the help popover.
    Trigger,

    /// The non-modal dialog content surface.
    Content,

    /// The required heading element linked from `aria-labelledby`.
    Heading,

    /// The required body element containing the main help text.
    Body,

    /// The optional footer element for links or actions.
    Footer,

    /// The visually hidden dismiss button for screen readers.
    DismissButton,
}

/// Localizable strings for contextual help.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the trigger button in Help variant. Default: `"Help"`.
    pub help_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the trigger button in Info variant. Default: `"Information"`.
    pub info_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the visually-hidden dismiss button. Default: `"Dismiss"`.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            help_label: MessageFn::static_str("Help"),
            info_label: MessageFn::static_str("Information"),
            close_label: MessageFn::static_str("Dismiss"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Connected API surface for contextual help.
pub struct Api<'a> {
    popover_api: popover::Api<'a>,
    props: &'a Props,
    locale: Locale,
    messages: Messages,
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("props", self.props)
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .finish_non_exhaustive()
    }
}

impl<'a> Api<'a> {
    /// Creates a contextual help API wrapping an active popover connect surface.
    #[must_use]
    pub fn new(
        popover_api: popover::Api<'a>,
        props: &'a Props,
        env: &Env,
        messages: &Messages,
    ) -> Self {
        Self {
            popover_api,
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns `true` when the underlying popover is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.popover_api.is_open()
    }

    /// Returns attributes for the root container element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        let label = match self.props.variant {
            Variant::Help => &self.messages.help_label,
            Variant::Info => &self.messages.info_label,
        };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label(&self.locale))
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.popover_api.content_id(),
            )
            .set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str());

        attrs
    }

    /// Returns attributes for the content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.popover_api.heading_id(),
            )
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Id, self.popover_api.content_id())
            .set(
                HtmlAttr::Data("ars-state"),
                if self.is_open() { "open" } else { "closed" },
            );

        let resolved_dir = self.props.dir.map_or_else(
            || self.locale.direction(),
            |dir| dir.resolve(self.locale.direction()),
        );

        attrs.set(HtmlAttr::Dir, resolved_dir.as_html_attr());

        attrs
    }

    /// Returns attributes for the heading element.
    #[must_use]
    pub fn heading_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.popover_api.heading_id());

        attrs
    }

    /// Returns attributes for the body element.
    #[must_use]
    pub fn body_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Body.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the optional footer element.
    #[must_use]
    pub fn footer_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Footer.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the visually hidden dismiss button.
    #[must_use]
    pub fn dismiss_button_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.messages.close_label)(&self.locale),
            );

        attrs
    }

    /// Adapter handler: the trigger element was activated.
    pub fn on_trigger_click(&self) {
        self.popover_api.toggle();
    }

    /// Adapter handler: the visually hidden dismiss button was activated.
    pub fn on_dismiss_button_click(&self) {
        self.popover_api.close();
    }

    /// Adapter handler: a key was pressed on the content element.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            self.popover_api.close();
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Content => self.content_attrs(),
            Part::Heading => self.heading_attrs(),
            Part::Body => self.body_attrs(),
            Part::Footer => self.footer_attrs(),
            Part::DismissButton => self.dismiss_button_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        rc::Rc,
        string::{String, ToString},
        vec::Vec,
    };
    use core::cell::RefCell;

    use ars_core::{ConnectApi as _, Service};
    use insta::assert_snapshot;

    use super::*;
    use crate::overlay::popover;

    fn test_props() -> Props {
        Props {
            id: "help".to_string(),
            ..Props::default()
        }
    }

    fn keyboard_data(key: KeyboardKey) -> KeyboardEventData {
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

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn contextual_help_api(open: bool, props: Props) -> Api<'static> {
        let mut popover_props = props.popover_props();

        popover_props.default_open = open;

        let help_props = Box::leak(Box::new(props));

        let service = Box::leak(Box::new(Service::<popover::Machine>::new(
            popover_props,
            &Env::default(),
            &popover::Messages::default(),
        )));

        let popover_api = service.connect(&|_| {});

        let messages = Messages::default();

        Api::new(popover_api, help_props, &Env::default(), &messages)
    }

    fn drain_events(events: &Rc<RefCell<Vec<popover::Event>>>) -> Vec<popover::Event> {
        events.borrow().clone()
    }

    // ── Issue-named behavioral tests ────────────────────────────────

    #[test]
    fn trigger_help_variant_has_aria_label() {
        let attrs = contextual_help_api(false, test_props()).trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Help"));
    }

    #[test]
    fn trigger_info_variant_has_aria_label() {
        let attrs = contextual_help_api(
            false,
            Props {
                variant: Variant::Info,
                ..test_props()
            },
        )
        .trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Information")
        );
    }

    #[test]
    fn trigger_has_aria_haspopup_dialog() {
        let attrs = contextual_help_api(false, test_props()).trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("dialog")
        );
    }

    #[test]
    fn trigger_reflects_variant_in_data_attr() {
        let help_attrs = contextual_help_api(false, test_props()).trigger_attrs();
        let info_attrs = contextual_help_api(
            false,
            Props {
                variant: Variant::Info,
                ..test_props()
            },
        )
        .trigger_attrs();

        assert_eq!(help_attrs.get(&HtmlAttr::Data("ars-variant")), Some("help"));
        assert_eq!(info_attrs.get(&HtmlAttr::Data("ars-variant")), Some("info"));
    }

    #[test]
    fn content_open_state_attrs() {
        let attrs = contextual_help_api(true, test_props()).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("dialog"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("open"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("help-heading")
        );
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("help-content"));
    }

    #[test]
    fn content_closed_state_attrs() {
        let attrs = contextual_help_api(false, test_props()).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("closed"));
    }

    #[test]
    fn heading_and_body_parts_emit_scope_and_part() {
        let api = contextual_help_api(false, test_props());

        let heading_attrs = api.heading_attrs();
        let body_attrs = api.body_attrs();

        assert_eq!(
            heading_attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("contextual-help")
        );
        assert_eq!(
            heading_attrs.get(&HtmlAttr::Data("ars-part")),
            Some("heading")
        );
        assert_eq!(
            body_attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("contextual-help")
        );
        assert_eq!(body_attrs.get(&HtmlAttr::Data("ars-part")), Some("body"));
    }

    #[test]
    fn connect_api_part_attrs_dispatch() {
        let api = contextual_help_api(true, test_props());

        for part in Part::all() {
            let expected = match part {
                Part::Root => api.root_attrs(),
                Part::Trigger => api.trigger_attrs(),
                Part::Content => api.content_attrs(),
                Part::Heading => api.heading_attrs(),
                Part::Body => api.body_attrs(),
                Part::Footer => api.footer_attrs(),
                Part::DismissButton => api.dismiss_button_attrs(),
            };

            assert_eq!(api.part_attrs(part), expected);
        }
    }

    #[test]
    fn on_trigger_click_toggles_popover() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let events_capture = Rc::clone(&events);

        let help_props = Box::leak(Box::new(test_props()));

        let service = Box::leak(Box::new(Service::<popover::Machine>::new(
            help_props.popover_props(),
            &Env::default(),
            &popover::Messages::default(),
        )));

        let send = Box::leak(Box::new(move |event| {
            events_capture.borrow_mut().push(event);
        }));

        let api = Api::new(
            service.connect(send),
            help_props,
            &Env::default(),
            &Messages::default(),
        );

        api.on_trigger_click();

        assert_eq!(drain_events(&events), vec![popover::Event::Toggle]);
    }

    #[test]
    fn dismiss_button_attrs_include_type_button() {
        let attrs = contextual_help_api(true, test_props()).dismiss_button_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
    }

    #[test]
    fn on_dismiss_button_click_closes_popover() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let events_capture = Rc::clone(&events);

        let help_props = Box::leak(Box::new(test_props()));
        let service = Box::leak(Box::new(Service::<popover::Machine>::new(
            help_props.popover_props(),
            &Env::default(),
            &popover::Messages::default(),
        )));

        let _open = service.send(popover::Event::Open);

        let send = Box::leak(Box::new(move |event| {
            events_capture.borrow_mut().push(event);
        }));

        let api = Api::new(
            service.connect(send),
            help_props,
            &Env::default(),
            &Messages::default(),
        );

        api.on_dismiss_button_click();

        assert!(drain_events(&events).contains(&popover::Event::Close));
    }

    #[test]
    fn on_content_keydown_escape_closes() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let events_capture = Rc::clone(&events);

        let help_props = Box::leak(Box::new(test_props()));

        let service = Box::leak(Box::new(Service::<popover::Machine>::new(
            help_props.popover_props(),
            &Env::default(),
            &popover::Messages::default(),
        )));

        let _open = service.send(popover::Event::Open);

        let send = Box::leak(Box::new(move |event| {
            events_capture.borrow_mut().push(event);
        }));

        let api = Api::new(
            service.connect(send),
            help_props,
            &Env::default(),
            &Messages::default(),
        );

        api.on_content_keydown(&keyboard_data(KeyboardKey::Escape));

        assert!(drain_events(&events).contains(&popover::Event::Close));
    }

    #[test]
    fn props_id_builder_sets_instance_id() {
        assert_eq!(Props::new().id("ctx-help").id, "ctx-help");
    }

    #[test]
    fn popover_props_hardcodes_non_modal_dismiss_policy() {
        let props = test_props().popover_props();

        assert!(!props.modal);
        assert!(props.close_on_escape);
        assert!(props.close_on_interact_outside);
        assert_eq!(props.positioning.placement, Placement::BottomStart);
        assert!(props.positioning.flip);
        assert_eq!(props.positioning.shift_padding, 12.0);
    }

    #[test]
    fn popover_props_forwards_non_default_offset_cross_offset_and_flip() {
        let popover = Props::new()
            .offset(3.0)
            .cross_offset(4.0)
            .should_flip(false)
            .popover_props();

        assert_eq!(popover.offset, 3.0);
        assert_eq!(popover.cross_offset, 4.0);
        assert!(!popover.positioning.flip);
    }

    #[test]
    fn content_attrs_inherits_locale_direction_when_dir_unset() {
        let attrs = contextual_help_api(false, test_props()).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("ltr"));
    }

    #[test]
    fn content_attrs_honors_explicit_dir_override() {
        let attrs = contextual_help_api(
            false,
            Props {
                dir: Some(Direction::Rtl),
                ..test_props()
            },
        )
        .content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
    }

    #[test]
    fn content_attrs_resolves_auto_dir_from_locale() {
        let attrs = contextual_help_api(
            false,
            Props {
                dir: Some(Direction::Auto),
                ..test_props()
            },
        )
        .content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("ltr"));
    }

    // ── Snapshot tests ──────────────────────────────────────────────

    #[test]
    fn contextual_help_root_closed_snapshot() {
        assert_snapshot!(
            "contextual_help_root_closed",
            snapshot_attrs(&contextual_help_api(false, test_props()).root_attrs())
        );
    }

    #[test]
    fn contextual_help_root_open_snapshot() {
        assert_snapshot!(
            "contextual_help_root_open",
            snapshot_attrs(&contextual_help_api(true, test_props()).root_attrs())
        );
    }

    #[test]
    fn contextual_help_trigger_closed_help_snapshot() {
        assert_snapshot!(
            "contextual_help_trigger_closed_help",
            snapshot_attrs(&contextual_help_api(false, test_props()).trigger_attrs())
        );
    }

    #[test]
    fn contextual_help_trigger_open_help_snapshot() {
        assert_snapshot!(
            "contextual_help_trigger_open_help",
            snapshot_attrs(&contextual_help_api(true, test_props()).trigger_attrs())
        );
    }

    #[test]
    fn contextual_help_trigger_closed_info_snapshot() {
        assert_snapshot!(
            "contextual_help_trigger_closed_info",
            snapshot_attrs(
                &contextual_help_api(
                    false,
                    Props {
                        variant: Variant::Info,
                        ..test_props()
                    },
                )
                .trigger_attrs()
            )
        );
    }

    #[test]
    fn contextual_help_trigger_open_info_snapshot() {
        assert_snapshot!(
            "contextual_help_trigger_open_info",
            snapshot_attrs(
                &contextual_help_api(
                    true,
                    Props {
                        variant: Variant::Info,
                        ..test_props()
                    },
                )
                .trigger_attrs()
            )
        );
    }

    #[test]
    fn contextual_help_content_closed_snapshot() {
        assert_snapshot!(
            "contextual_help_content_closed",
            snapshot_attrs(&contextual_help_api(false, test_props()).content_attrs())
        );
    }

    #[test]
    fn contextual_help_content_open_snapshot() {
        assert_snapshot!(
            "contextual_help_content_open",
            snapshot_attrs(&contextual_help_api(true, test_props()).content_attrs())
        );
    }

    #[test]
    fn contextual_help_heading_snapshot() {
        assert_snapshot!(
            "contextual_help_heading",
            snapshot_attrs(&contextual_help_api(true, test_props()).heading_attrs())
        );
    }

    #[test]
    fn contextual_help_body_snapshot() {
        assert_snapshot!(
            "contextual_help_body",
            snapshot_attrs(&contextual_help_api(true, test_props()).body_attrs())
        );
    }

    #[test]
    fn contextual_help_footer_snapshot() {
        assert_snapshot!(
            "contextual_help_footer",
            snapshot_attrs(&contextual_help_api(true, test_props()).footer_attrs())
        );
    }

    #[test]
    fn contextual_help_dismiss_button_snapshot() {
        assert_snapshot!(
            "contextual_help_dismiss_button",
            snapshot_attrs(&contextual_help_api(true, test_props()).dismiss_button_attrs())
        );
    }
}
