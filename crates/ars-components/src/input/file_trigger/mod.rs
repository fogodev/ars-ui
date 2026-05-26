//! `FileTrigger` stateless connect API.
//!
//! This module implements the framework-agnostic `FileTrigger` contract defined
//! in `spec/components/input/file-trigger.md`. Core code only maps props to
//! attributes and returns a typed picker-opening intent; adapters own the
//! concrete hidden input reference or platform file-picker handle.

use alloc::{string::String, sync::Arc, vec::Vec};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, HasId, HtmlAttr, Locale,
    MessageFn,
};

/// Camera capture direction for mobile file pickers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    /// Prefer the front-facing camera.
    User,

    /// Prefer the rear-facing environment camera.
    Environment,
}

impl CaptureMode {
    /// Returns the native `capture` attribute token for this mode.
    #[must_use]
    pub const fn as_attr_value(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Environment => "environment",
        }
    }
}

/// Props for the `FileTrigger` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Accepted MIME types or file extensions. Empty accepts all files.
    pub accept: Vec<String>,

    /// Whether selecting multiple files is allowed.
    pub multiple: bool,

    /// Whether directory selection is allowed through `webkitdirectory`.
    pub directory: bool,

    /// Optional mobile camera capture preference.
    pub capture: Option<CaptureMode>,

    /// Whether trigger activation is disabled.
    pub disabled: bool,

    /// Form field name for the hidden native file input.
    pub name: Option<String>,
}

impl Props {
    /// Returns fresh props with the documented defaults.
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

    /// Sets the accepted MIME types or file extensions.
    #[must_use]
    pub fn accept<I, S>(mut self, accept: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.accept = accept.into_iter().map(Into::into).collect();
        self
    }

    /// Clears accepted file-type restrictions.
    #[must_use]
    pub fn clear_accept(mut self) -> Self {
        self.accept.clear();
        self
    }

    /// Sets whether multiple file selection is allowed.
    #[must_use]
    pub const fn multiple(mut self, value: bool) -> Self {
        self.multiple = value;
        self
    }

    /// Sets whether directory selection is allowed.
    #[must_use]
    pub const fn directory(mut self, value: bool) -> Self {
        self.directory = value;
        self
    }

    /// Sets the mobile camera capture preference.
    #[must_use]
    pub const fn capture(mut self, mode: CaptureMode) -> Self {
        self.capture = Some(mode);
        self
    }

    /// Clears the mobile camera capture preference.
    #[must_use]
    pub const fn clear_capture(mut self) -> Self {
        self.capture = None;
        self
    }

    /// Sets whether trigger activation is disabled.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets the form field name for the hidden native file input.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears the form field name.
    #[must_use]
    pub fn clear_name(mut self) -> Self {
        self.name = None;
        self
    }
}

/// Dynamic callable signature for [`Messages::input_label`].
pub type InputLabelFn = dyn Fn(bool, &Locale) -> String + Send + Sync;

/// Messages for the `FileTrigger` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the hidden native file input.
    pub input_label: MessageFn<InputLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            input_label: MessageFn::new(Arc::new(|multiple: bool, _locale: &Locale| {
                if multiple {
                    String::from("Choose files")
                } else {
                    String::from("Choose file")
                }
            }) as Arc<InputLabelFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the file-trigger connect API.
#[derive(ComponentPart)]
#[scope = "file-trigger"]
pub enum Part {
    /// Root wrapper element.
    Root,

    /// Consumer-rendered pressable trigger element.
    Trigger,

    /// Hidden native `<input type="file">` element.
    Input,
}

/// Adapter-resolvable intent to open the native file picker.
///
/// This marker carries no DOM target. Adapters resolve it against their own
/// hidden input reference or platform file-picker abstraction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenPickerIntent;

/// API for the `FileTrigger` component.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Creates a new API from props, locale, and localized messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::input::file_trigger::{Api, Messages, Props};
    /// use ars_i18n::locales::en_us;
    ///
    /// let api = Api::new(Props::new().multiple(true), en_us(), Messages::default());
    /// assert!(api.open_picker_intent().is_some());
    /// ```
    #[must_use]
    pub const fn new(props: Props, locale: Locale, messages: Messages) -> Self {
        Self {
            props,
            locale,
            messages,
        }
    }

    /// Returns the underlying props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component instance ID.
    #[must_use]
    pub const fn id(&self) -> &str {
        self.props.id.as_str()
    }

    /// Returns whether trigger activation is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.props.disabled
    }

    /// Returns the accepted MIME types or file extensions.
    #[must_use]
    pub fn accept(&self) -> &[String] {
        &self.props.accept
    }

    /// Root wrapper attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.props.id.as_str());

        if self.props.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Pressable trigger attributes.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.props.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Hidden native file input attributes.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "file")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.messages.input_label)(self.props.multiple, &self.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::TabIndex, "-1");

        if !self.props.accept.is_empty() {
            attrs.set(HtmlAttr::Accept, self.props.accept.join(","));
        }

        if self.props.multiple {
            attrs.set_bool(HtmlAttr::Multiple, true);
        }

        if self.props.directory {
            attrs.set_bool(HtmlAttr::WebkitDirectory, true);
        }

        if let Some(capture) = self.props.capture {
            attrs.set(HtmlAttr::Capture, capture.as_attr_value());
        }

        if let Some(name) = self.props.name.as_deref() {
            attrs.set(HtmlAttr::Name, name);
        }

        attrs
    }

    /// Returns adapter-resolvable picker intent when activation is enabled.
    #[must_use]
    pub const fn open_picker_intent(&self) -> Option<OpenPickerIntent> {
        if self.props.disabled {
            None
        } else {
            Some(OpenPickerIntent)
        }
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Input => self.input_attrs(),
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

    fn en_locale() -> Locale {
        ars_i18n::locales::en_us()
    }

    fn api(props: Props) -> Api {
        Api::new(props, en_locale(), Messages::default())
    }

    #[test]
    fn props_default_matches_spec() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert!(props.accept.is_empty());
        assert!(!props.multiple);
        assert!(!props.directory);
        assert_eq!(props.capture, None);
        assert!(!props.disabled);
        assert_eq!(props.name, None);
    }

    #[test]
    fn props_builder_round_trips() {
        let props = Props::new()
            .id("upload")
            .accept(["image/*", ".pdf"])
            .multiple(true)
            .directory(true)
            .capture(CaptureMode::Environment)
            .disabled(true)
            .name("attachments");

        assert_eq!(props.id, "upload");
        assert_eq!(props.accept, vec!["image/*", ".pdf"]);
        assert!(props.multiple);
        assert!(props.directory);
        assert_eq!(props.capture, Some(CaptureMode::Environment));
        assert!(props.disabled);
        assert_eq!(props.name.as_deref(), Some("attachments"));
    }

    #[test]
    fn props_clear_builders_round_trip() {
        let props = Props::new()
            .id("file-trigger")
            .accept(["image/*"])
            .multiple(true)
            .directory(true)
            .capture(CaptureMode::User)
            .disabled(true)
            .name("avatar")
            .clear_accept()
            .clear_capture()
            .clear_name();

        assert_eq!(props.id, "file-trigger");
        assert!(props.accept.is_empty());
        assert!(props.multiple);
        assert!(props.directory);
        assert_eq!(props.capture, None);
        assert!(props.disabled);
        assert_eq!(props.name, None);
    }

    #[test]
    fn has_id_derive_round_trips() {
        let mut props = Props::default().with_id(String::from("files"));

        assert_eq!(HasId::id(&props), "files");

        props.set_id(String::from("uploads"));

        assert_eq!(HasId::id(&props), "uploads");
    }

    #[test]
    fn root_attrs_include_scope_part_id_and_disabled_state() {
        let attrs = api(Props::new().id("files").disabled(true)).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("files"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("file-trigger")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&ars_core::AttrValue::Bool(true))
        );
    }

    #[test]
    fn trigger_attrs_include_scope_part_and_disabled_state() {
        let attrs = api(Props::new().disabled(true)).trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("file-trigger")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("trigger"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn input_attrs_include_file_input_a11y_attrs() {
        let attrs = api(Props::new()).input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("file-trigger")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("input"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("file"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Choose file")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn input_attrs_reflect_accept_multiple_directory_capture_and_name() {
        let attrs = api(Props::new()
            .accept(["image/*", ".pdf"])
            .multiple(true)
            .directory(true)
            .capture(CaptureMode::User)
            .name("attachments"))
        .input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Accept), Some("image/*,.pdf"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Multiple),
            Some(&ars_core::AttrValue::Bool(true))
        );
        assert_eq!(
            attrs.get_value(&HtmlAttr::WebkitDirectory),
            Some(&ars_core::AttrValue::Bool(true))
        );
        assert_eq!(attrs.get(&HtmlAttr::Capture), Some("user"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("attachments"));
    }

    #[test]
    fn open_picker_intent_is_suppressed_when_disabled() {
        assert_eq!(
            api(Props::new()).open_picker_intent(),
            Some(OpenPickerIntent)
        );
        assert_eq!(api(Props::new().disabled(true)).open_picker_intent(), None);
    }

    #[test]
    fn api_accessors_report_current_props() {
        let enabled = api(Props::new()
            .id("picker")
            .accept(["image/*", ".pdf"])
            .multiple(true));

        assert_eq!(enabled.id(), "picker");
        assert!(!enabled.is_disabled());
        assert_eq!(
            enabled.accept(),
            &["image/*".to_string(), ".pdf".to_string()]
        );
        assert_eq!(enabled.props().id, "picker");

        let disabled = api(Props::new().id("blocked").disabled(true));

        assert_eq!(disabled.id(), "blocked");
        assert!(disabled.is_disabled());
        assert!(disabled.accept().is_empty());
    }

    #[test]
    fn part_attrs_dispatches_each_part() {
        let api = api(Props::new());

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Input), api.input_attrs());
    }

    #[test]
    fn file_trigger_root_snapshot() {
        assert_snapshot!(snapshot_attrs(&api(Props::new().id("files")).root_attrs()));
    }

    #[test]
    fn file_trigger_root_disabled_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api(Props::new().id("files").disabled(true)).root_attrs()
        ));
    }

    #[test]
    fn file_trigger_trigger_snapshot() {
        assert_snapshot!(snapshot_attrs(&api(Props::new()).trigger_attrs()));
    }

    #[test]
    fn file_trigger_trigger_disabled_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api(Props::new().disabled(true)).trigger_attrs()
        ));
    }

    #[test]
    fn file_trigger_input_default_snapshot() {
        assert_snapshot!(snapshot_attrs(&api(Props::new()).input_attrs()));
    }

    #[test]
    fn file_trigger_input_accept_multiple_directory_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api(Props::new()
                .accept(["image/*", ".pdf"])
                .multiple(true)
                .directory(true))
            .input_attrs()
        ));
    }

    #[test]
    fn file_trigger_input_capture_name_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api(Props::new()
                .capture(CaptureMode::Environment)
                .name("attachments"))
            .input_attrs()
        ));
    }

    #[test]
    fn file_trigger_input_plural_label_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api(Props::new().multiple(true)).input_attrs()
        ));
    }
}
