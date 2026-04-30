//! `Landmark` component connect API.
//!
//! `Landmark` is a stateless, framework-agnostic attribute mapper for page
//! structure. It maps semantic landmark roles to ARIA role tokens and label
//! attributes while leaving actual element selection to framework adapters.

use alloc::string::String;

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Locale,
    MessageFn,
};

/// The semantic role of a landmark region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Page-level banner landmark, usually rendered as `<header>`.
    Banner,

    /// Navigation landmark, usually rendered as `<nav>`.
    Navigation,

    /// Main content landmark, usually rendered as `<main>`.
    Main,

    /// Complementary content landmark, usually rendered as `<aside>`.
    Complementary,

    /// Page-level content information landmark, usually rendered as `<footer>`.
    ContentInfo,

    /// Search landmark, rendered as `<search>` or a fallback element.
    Search,

    /// Form landmark, recognized by assistive technology only when named.
    Form,

    /// Region landmark, recognized by assistive technology only when named.
    Region,
}

impl Role {
    /// Returns the WAI-ARIA role token for this landmark role.
    #[must_use]
    pub const fn aria_role(self) -> &'static str {
        match self {
            Self::Banner => "banner",
            Self::Navigation => "navigation",
            Self::Main => "main",
            Self::Complementary => "complementary",
            Self::ContentInfo => "contentinfo",
            Self::Search => "search",
            Self::Form => "form",
            Self::Region => "region",
        }
    }

    /// Returns whether this landmark role requires an accessible name to be
    /// exposed consistently by assistive technology.
    #[must_use]
    pub const fn requires_accessible_name(self) -> bool {
        matches!(self, Self::Form | Self::Region)
    }
}

/// Props for the `Landmark` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The semantic landmark role.
    pub role: Role,

    /// Optional ID of an external element that labels this landmark.
    ///
    /// When set, `aria-labelledby` takes precedence over localized
    /// `aria-label` output.
    pub labelledby_id: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            role: Role::Region,
            labelledby_id: None,
        }
    }
}

impl Props {
    /// Returns fresh landmark props with the documented defaults.
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

    /// Sets the semantic landmark role.
    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Sets the ID of an external labelling element.
    #[must_use]
    pub fn labelledby_id(mut self, id: impl Into<String>) -> Self {
        self.labelledby_id = Some(id.into());
        self
    }

    /// Clears any external labelling element ID.
    #[must_use]
    pub fn unlabelled(mut self) -> Self {
        self.labelledby_id = None;
        self
    }
}

/// Messages for the `Landmark` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible name for the landmark region.
    ///
    /// Form and Region landmarks require a non-empty accessible name to be
    /// exposed consistently by assistive technology.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str(""),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the `Landmark` connect API.
#[derive(ComponentPart)]
#[scope = "landmark"]
pub enum Part {
    /// The root landmark element.
    Root,
}

/// API for the `Landmark` component.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Creates a new API from landmark props, environment, and localized
    /// messages.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        Self {
            props,
            locale: env.locale.clone(),
            messages: messages.clone(),
        }
    }

    /// Returns the underlying landmark props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the requested semantic landmark role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.props.role
    }

    /// Returns the external labelling element ID, when present and non-empty.
    #[must_use]
    pub fn labelledby_id(&self) -> Option<&str> {
        self.non_empty_labelledby_id()
    }

    /// Returns `true` when adapters should prefer their generic fallback
    /// element for the default rendering path.
    ///
    /// Adapters can still call [`Self::root_attrs`] with
    /// `is_native_landmark_element` set to `false` for any role when they render
    /// an explicit-role fallback element.
    #[must_use]
    pub const fn prefers_generic_fallback_element(&self) -> bool {
        matches!(self.props.role, Role::Search)
    }

    /// Resolves the localized accessible label for this landmark.
    #[must_use]
    pub fn label(&self) -> String {
        (self.messages.label)(&self.locale)
    }

    /// Returns whether this landmark has either `aria-labelledby` or a
    /// localized `aria-label` containing non-whitespace text.
    #[must_use]
    pub fn has_accessible_name(&self) -> bool {
        self.non_empty_labelledby_id().is_some() || label_has_text(&self.label())
    }

    /// Returns whether connecting this landmark should emit the development
    /// warning for a required but missing accessible name.
    #[must_use]
    pub fn missing_accessible_name_warning_needed(&self) -> bool {
        self.props.role.requires_accessible_name() && !self.has_accessible_name()
    }

    /// Returns root attributes for the landmark element.
    ///
    /// The core always emits stable component data attributes. It emits an
    /// explicit ARIA `role` only for fallback elements, and it never
    /// emits both `aria-labelledby` and `aria-label`.
    #[must_use]
    pub fn root_attrs(&self, is_native_landmark_element: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.props.id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if !is_native_landmark_element {
            attrs.set(HtmlAttr::Role, self.props.role.aria_role());
        }

        if let Some(labelledby_id) = self.non_empty_labelledby_id() {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby_id);
        } else {
            let label = self.label();

            if label_has_text(&label) {
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
            } else if self.missing_accessible_name_warning_needed() {
                warn_missing_accessible_name(self.props.role);
            }
        }

        attrs
    }

    fn non_empty_labelledby_id(&self) -> Option<&str> {
        self.props
            .labelledby_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
    }
}

fn label_has_text(label: &str) -> bool {
    !label.trim().is_empty()
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(true),
        }
    }
}

#[cfg(feature = "debug")]
fn warn_missing_accessible_name(role: Role) {
    if role.requires_accessible_name() {
        log::warn!(
            "landmark: role '{}' requires an accessible name (aria-label or aria-labelledby)",
            role.aria_role()
        );
    }
}

#[cfg(all(
    debug_assertions,
    not(feature = "debug"),
    feature = "std",
    not(all(
        target_arch = "wasm32",
        not(any(target_os = "emscripten", target_os = "wasi"))
    ))
))]
fn warn_missing_accessible_name(role: Role) {
    if role.requires_accessible_name() {
        eprintln!(
            "landmark: role '{}' requires an accessible name (aria-label or aria-labelledby)",
            role.aria_role()
        );
    }
}

#[cfg(not(any(
    feature = "debug",
    all(
        debug_assertions,
        feature = "std",
        not(all(
            target_arch = "wasm32",
            not(any(target_os = "emscripten", target_os = "wasi"))
        ))
    )
)))]
const fn warn_missing_accessible_name(_role: Role) {}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrMap, ConnectApi, Env, HasId, HtmlAttr, Locale, MessageFn};
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn api(props: Props) -> Api {
        Api::new(props, &Env::default(), &Messages::default())
    }

    fn labelled_api(props: Props, label: &'static str) -> Api {
        Api::new(
            props,
            &Env::default(),
            &Messages {
                label: MessageFn::static_str(label),
            },
        )
    }

    #[test]
    fn role_aria_role_maps_all_landmarks() {
        let cases = [
            (Role::Banner, "banner"),
            (Role::Navigation, "navigation"),
            (Role::Main, "main"),
            (Role::Complementary, "complementary"),
            (Role::ContentInfo, "contentinfo"),
            (Role::Search, "search"),
            (Role::Form, "form"),
            (Role::Region, "region"),
        ];

        for (role, expected) in cases {
            assert_eq!(role.aria_role(), expected);
        }
    }

    #[test]
    fn props_default_is_region_without_labelledby() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert_eq!(props.role, Role::Region);
        assert_eq!(props.labelledby_id, None);
        assert_eq!(Props::new(), props);
    }

    #[test]
    fn props_builder_round_trips_all_fields() {
        let props = Props::new()
            .id("landmark-1")
            .role(Role::Navigation)
            .labelledby_id("landmark-label");

        assert_eq!(props.id, "landmark-1");
        assert_eq!(props.role, Role::Navigation);
        assert_eq!(props.labelledby_id.as_deref(), Some("landmark-label"));
        assert_eq!(props.unlabelled().labelledby_id, None);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        let mut props = Props::new().with_id(String::from("landmark-a"));

        assert_eq!(HasId::id(&props), "landmark-a");

        props.set_id(String::from("landmark-b"));

        assert_eq!(HasId::id(&props), "landmark-b");
    }

    #[test]
    fn messages_default_label_is_empty() {
        let messages = Messages::default();
        let locale = Locale::parse("en-US").expect("en-US must parse");

        assert_eq!((messages.label)(&locale), "");
    }

    #[test]
    fn api_prefers_generic_fallback_element_only_for_search() {
        for role in [
            Role::Banner,
            Role::Navigation,
            Role::Main,
            Role::Complementary,
            Role::ContentInfo,
            Role::Form,
            Role::Region,
        ] {
            assert!(!api(Props::new().role(role)).prefers_generic_fallback_element());
        }

        assert!(api(Props::new().role(Role::Search)).prefers_generic_fallback_element());
    }

    #[test]
    fn accessible_name_helpers_identify_required_and_missing_names() {
        let named = labelled_api(Props::new().role(Role::Region), "Activity");
        let blank_label = labelled_api(Props::new().role(Role::Region), "   ");

        let labelledby = api(Props::new()
            .role(Role::Form)
            .labelledby_id("external-label"));

        let empty_labelledby = api(Props::new().role(Role::Form).labelledby_id(""));
        let blank_labelledby = api(Props::new().role(Role::Region).labelledby_id("   "));

        let missing_region = api(Props::new().role(Role::Region));

        let missing_navigation = api(Props::new().role(Role::Navigation));

        assert!(Role::Form.requires_accessible_name());
        assert!(Role::Region.requires_accessible_name());
        assert!(!Role::Navigation.requires_accessible_name());
        assert!(named.has_accessible_name());
        assert!(!blank_label.has_accessible_name());
        assert!(blank_label.missing_accessible_name_warning_needed());
        assert!(labelledby.has_accessible_name());
        assert!(!empty_labelledby.has_accessible_name());
        assert!(!blank_labelledby.has_accessible_name());
        assert!(empty_labelledby.missing_accessible_name_warning_needed());
        assert!(blank_labelledby.missing_accessible_name_warning_needed());
        assert!(!missing_region.has_accessible_name());
        assert!(missing_region.missing_accessible_name_warning_needed());
        assert!(!missing_navigation.missing_accessible_name_warning_needed());
    }

    #[test]
    fn landmark_root_banner() {
        assert_snapshot!(
            "landmark_root_banner",
            snapshot_attrs(&api(Props::new().id("banner").role(Role::Banner)).root_attrs(true))
        );
    }

    #[test]
    fn landmark_root_navigation() {
        assert_snapshot!(
            "landmark_root_navigation",
            snapshot_attrs(
                &labelled_api(
                    Props::new().id("navigation").role(Role::Navigation),
                    "Primary navigation",
                )
                .root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_root_main() {
        assert_snapshot!(
            "landmark_root_main",
            snapshot_attrs(&api(Props::new().id("main").role(Role::Main)).root_attrs(true))
        );
    }

    #[test]
    fn landmark_root_complementary() {
        assert_snapshot!(
            "landmark_root_complementary",
            snapshot_attrs(
                &labelled_api(
                    Props::new().id("complementary").role(Role::Complementary),
                    "Related content",
                )
                .root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_root_contentinfo() {
        assert_snapshot!(
            "landmark_root_contentinfo",
            snapshot_attrs(
                &api(Props::new().id("contentinfo").role(Role::ContentInfo)).root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_root_search() {
        assert_snapshot!(
            "landmark_root_search",
            snapshot_attrs(
                &labelled_api(Props::new().id("search").role(Role::Search), "Site search")
                    .root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_root_form_labelled() {
        assert_snapshot!(
            "landmark_root_form_labelled",
            snapshot_attrs(
                &labelled_api(Props::new().id("form").role(Role::Form), "Account settings")
                    .root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_root_region_labelled() {
        assert_snapshot!(
            "landmark_root_region_labelled",
            snapshot_attrs(
                &labelled_api(Props::new().id("region").role(Role::Region), "Activity")
                    .root_attrs(true)
            )
        );
    }

    #[test]
    fn landmark_fallback_roots_emit_explicit_roles_for_every_role() {
        let cases = [
            (Role::Banner, "landmark_fallback_banner"),
            (Role::Navigation, "landmark_fallback_navigation"),
            (Role::Main, "landmark_fallback_main"),
            (Role::Complementary, "landmark_fallback_complementary"),
            (Role::ContentInfo, "landmark_fallback_contentinfo"),
            (Role::Search, "landmark_fallback_search"),
            (Role::Form, "landmark_fallback_form"),
            (Role::Region, "landmark_fallback_region"),
        ];

        for (role, snapshot_name) in cases {
            let attrs = labelled_api(
                Props::new()
                    .id(format!("fallback-{}", role.aria_role()))
                    .role(role),
                "Fallback landmark",
            )
            .root_attrs(false);

            assert_eq!(attrs.get(&HtmlAttr::Role), Some(role.aria_role()));
            assert_snapshot!(snapshot_name, snapshot_attrs(&attrs));
        }
    }

    #[test]
    fn landmark_root_labelledby_takes_precedence() {
        let attrs = labelled_api(
            Props::new()
                .id("labelledby")
                .role(Role::Region)
                .labelledby_id("external-label"),
            "Ignored label",
        )
        .root_attrs(true);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("external-label")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);

        assert_snapshot!(
            "landmark_root_labelledby_takes_precedence",
            snapshot_attrs(&attrs)
        );
    }

    #[test]
    fn landmark_root_empty_labelledby_falls_back_to_label_message() {
        let api = labelled_api(
            Props::new()
                .id("empty-labelledby")
                .role(Role::Region)
                .labelledby_id(""),
            "Activity",
        );

        assert_eq!(api.labelledby_id(), None);

        let attrs = api.root_attrs(true);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)), None);
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Activity")
        );
    }

    #[test]
    fn landmark_root_blank_labelledby_is_treated_as_missing_name() {
        let region = api(Props::new().role(Role::Region).labelledby_id("   "));

        assert_eq!(region.labelledby_id(), None);
        assert!(!region.has_accessible_name());
        assert!(region.missing_accessible_name_warning_needed());

        let api = api(Props::new()
            .id("blank-labelledby")
            .role(Role::Navigation)
            .labelledby_id("   "));
        let attrs = api.root_attrs(true);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);
    }

    #[test]
    fn landmark_root_blank_label_message_is_treated_as_missing_name() {
        let api = labelled_api(Props::new().id("blank-label").role(Role::Navigation), "   ");

        assert!(!api.has_accessible_name());

        let attrs = api.root_attrs(true);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)), None);
    }

    #[test]
    fn landmark_root_label_message_sets_aria_label() {
        let api = labelled_api(
            Props::new().id("message-label").role(Role::Navigation),
            "Secondary navigation",
        );

        assert_eq!(api.label(), "Secondary navigation");

        let attrs = api.root_attrs(true);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Secondary navigation")
        );

        assert_snapshot!(
            "landmark_root_label_message_sets_aria_label",
            snapshot_attrs(&attrs)
        );
    }

    #[test]
    fn connect_api_root_matches_root_attrs() {
        let api = labelled_api(Props::new().id("connect").role(Role::Search), "Search");

        assert_eq!(api.role(), Role::Search);
        assert_eq!(api.props().id, "connect");
        assert_eq!(api.props().role, Role::Search);
        assert_eq!(api.labelledby_id(), None);
        assert_eq!(api.part_attrs(Part::Root), api.root_attrs(true));
    }

    #[cfg(not(feature = "debug"))]
    #[test]
    fn form_and_region_missing_label_warning_path_does_not_panic() {
        let form_attrs = api(Props::new().id("form").role(Role::Form)).root_attrs(true);
        let region_attrs = api(Props::new().id("region").role(Role::Region)).root_attrs(true);

        assert_eq!(form_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);
        assert_eq!(region_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn form_and_region_missing_label_emit_debug_warnings() {
        use std::sync::{Mutex, Once};

        struct TestLogger;

        impl log::Log for TestLogger {
            fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
                metadata.level() <= log::Level::Warn
            }

            fn log(&self, record: &log::Record<'_>) {
                if self.enabled(record.metadata()) {
                    WARNINGS
                        .lock()
                        .expect("warning capture mutex must not be poisoned")
                        .push(record.args().to_string());
                }
            }

            fn flush(&self) {}
        }

        static LOGGER: TestLogger = TestLogger;
        static INIT: Once = Once::new();
        static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

        INIT.call_once(|| {
            log::set_logger(&LOGGER).expect("test logger must install once");
            log::set_max_level(log::LevelFilter::Warn);
        });

        WARNINGS
            .lock()
            .expect("warning capture mutex must not be poisoned")
            .clear();

        drop(api(Props::new().role(Role::Form)).root_attrs(true));
        drop(api(Props::new().role(Role::Region)).root_attrs(true));

        drop(labelled_api(Props::new().role(Role::Region), "Activity").root_attrs(true));

        warn_missing_accessible_name(Role::Navigation);

        log::logger().flush();

        let warnings = WARNINGS
            .lock()
            .expect("warning capture mutex must not be poisoned");

        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|message| {
            message
                == "landmark: role 'form' requires an accessible name (aria-label or aria-labelledby)"
        }));
        assert!(warnings.iter().any(|message| {
            message
                == "landmark: role 'region' requires an accessible name (aria-label or aria-labelledby)"
        }));
    }
}
