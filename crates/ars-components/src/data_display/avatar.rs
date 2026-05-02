//! Avatar component state machine and connect API.
//!
//! `Avatar` displays identity with an image first, then a fallback derived from
//! a name or a default icon marker when no initials can be resolved.

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::{
    fmt::{self, Debug, Display},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Env,
    HtmlAttr, MessageFn, SafeUrl, TransitionPlan,
};
use ars_i18n::{Locale, take_graphemes};

type InitialsCallback = dyn Fn(String) -> String + Send + Sync;
type InitialsMessageFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Validated image source URL for Avatar images.
///
/// Avatar image sources accept the shared safe URL policy plus browser-generated
/// `blob:` URLs and selected raster `data:image/*` URLs. SVG data URLs are
/// intentionally rejected.
///
/// ```
/// use ars_components::data_display::avatar::ImageSrc;
/// use ars_core::SafeUrl;
///
/// let safe = SafeUrl::from_static("/avatar.png");
/// let relative = ImageSrc::from_safe_url(&safe);
/// assert_eq!(relative.as_str(), "/avatar.png");
///
/// let blob = ImageSrc::new("blob:https://example.com/avatar").unwrap();
/// assert_eq!(blob.as_str(), "blob:https://example.com/avatar");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageSrc(Cow<'static, str>);

impl ImageSrc {
    /// Creates a validated image source.
    ///
    /// # Errors
    ///
    /// Returns [`ImageSrcError`] when the value is not valid for Avatar image sources.
    pub fn new(src: impl Into<String>) -> Result<Self, ImageSrcError> {
        let src = src.into();

        if is_allowed_image_src(&src) {
            Ok(Self(Cow::Owned(src)))
        } else {
            Err(ImageSrcError(src))
        }
    }

    /// Creates an image source from an already validated shared safe URL.
    #[must_use]
    pub fn from_safe_url(src: &SafeUrl) -> Self {
        Self(Cow::Owned(src.as_str().to_string()))
    }

    /// Creates an image source from a static URL accepted by [`SafeUrl`].
    ///
    /// # Panics
    ///
    /// Panics when `src` uses a disallowed or unknown scheme under the shared safe URL policy.
    #[must_use]
    pub fn from_static(src: &'static str) -> Self {
        Self::from_safe_url(&SafeUrl::from_static(src))
    }

    /// Borrows the validated image source string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<SafeUrl> for ImageSrc {
    fn from(value: SafeUrl) -> Self {
        Self::from_safe_url(&value)
    }
}

impl From<&'static str> for ImageSrc {
    fn from(value: &'static str) -> Self {
        Self::from_static(value)
    }
}

impl Display for ImageSrc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when an Avatar image source fails validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageSrcError(
    /// The rejected image source string.
    pub String,
);

impl Display for ImageSrcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe image source: {}", self.0)
    }
}

impl core::error::Error for ImageSrcError {}

/// Props for the Avatar component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Validated image URL.
    pub src: Option<ImageSrc>,

    /// Full name for initials derivation and default accessible text.
    pub name: Option<String>,

    /// Explicit accessible label for the root image wrapper.
    pub aria_label: Option<String>,

    /// Delay before revealing fallback while an image loads.
    pub fallback_delay: Duration,

    /// Visual size token.
    pub size: Size,

    /// Visual shape token.
    pub shape: Shape,

    /// Custom initials extraction logic.
    pub get_initials: Option<Callback<InitialsCallback>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: None,
            name: None,
            aria_label: None,
            fallback_delay: Duration::from_millis(600),
            size: Size::Md,
            shape: Shape::Circle,
            get_initials: None,
        }
    }
}

impl Props {
    /// Returns fresh avatar props with the documented defaults.
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

    /// Sets the image URL.
    #[must_use]
    pub fn src(mut self, src: impl Into<ImageSrc>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Attempts to set the image URL after validating its image-source policy.
    ///
    /// ```
    /// use ars_components::data_display::avatar::Props;
    ///
    /// let props = Props::new()
    ///     .id("avatar")
    ///     .try_src("blob:https://example.com/avatar")
    ///     .unwrap();
    ///
    /// assert_eq!(props.src.as_ref().unwrap().as_str(), "blob:https://example.com/avatar");
    /// assert!(Props::new().try_src("javascript:alert(1)").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ImageSrcError`] when the URL is not valid for Avatar image sources.
    pub fn try_src(mut self, src: impl Into<String>) -> Result<Self, ImageSrcError> {
        self.src = Some(ImageSrc::new(src)?);
        Ok(self)
    }

    /// Clears the image URL.
    #[must_use]
    pub fn no_src(mut self) -> Self {
        self.src = None;
        self
    }

    /// Sets the full name used for initials and accessible text.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears the full name.
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets the explicit accessible label for the root image wrapper.
    ///
    /// ```
    /// use ars_components::data_display::avatar::Props;
    ///
    /// let props = Props::new().id("avatar").aria_label("Current user");
    /// assert_eq!(props.aria_label.as_deref(), Some("Current user"));
    /// ```
    #[must_use]
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Clears the explicit accessible label.
    #[must_use]
    pub fn no_aria_label(mut self) -> Self {
        self.aria_label = None;
        self
    }

    /// Sets the fallback reveal delay.
    #[must_use]
    pub const fn fallback_delay(mut self, fallback_delay: Duration) -> Self {
        self.fallback_delay = fallback_delay;
        self
    }

    /// Sets the visual size token.
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the visual shape token.
    #[must_use]
    pub const fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Sets the custom initials extraction callback.
    #[must_use]
    pub fn get_initials(mut self, get_initials: Callback<InitialsCallback>) -> Self {
        self.get_initials = Some(get_initials);
        self
    }

    /// Clears the custom initials extraction callback.
    #[must_use]
    pub fn no_get_initials(mut self) -> Self {
        self.get_initials = None;
        self
    }
}

/// Visual size of the avatar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    /// Extra-small avatar size.
    Xs,

    /// Small avatar size.
    Sm,

    /// Medium avatar size.
    Md,

    /// Large avatar size.
    Lg,

    /// Extra-large avatar size.
    Xl,
}

impl Size {
    /// Returns the `data-ars-size` value for this avatar size.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
            Self::Xl => "xl",
        }
    }
}

/// Visual shape of the avatar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// Circular avatar crop.
    Circle,

    /// Square avatar crop.
    Square,
}

impl Shape {
    /// Returns the `data-ars-shape` value for this avatar shape.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Square => "square",
        }
    }
}

/// Current load phase of the avatar image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadingStatus {
    /// Image is loading.
    Loading,

    /// Image has loaded successfully.
    Loaded,

    /// Image failed to load or is absent.
    Error,
}

/// States for the Avatar component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    /// Image is loading.
    Loading,

    /// Image has loaded successfully.
    Loaded,

    /// Image failed to load.
    Error,

    /// Fallback is shown because no image source is available.
    Fallback,
}

/// Events for the Avatar component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The image `load` event fired successfully.
    ImageLoad,

    /// The image `error` event fired.
    ImageError,

    /// The image source changed.
    SetSrc(Option<ImageSrc>),

    /// The fallback reveal delay elapsed while loading.
    FallbackDelayElapsed,
}

/// Context for the Avatar component.
#[derive(Clone, Debug)]
pub struct Context {
    /// The current image URL.
    pub src: Option<ImageSrc>,

    /// Current load phase.
    pub loading_status: LoadingStatus,

    /// Whether the fallback is currently visible.
    pub fallback_visible: bool,

    /// Resolved locale for initials extraction.
    pub locale: Locale,

    /// Resolved avatar messages.
    pub messages: Messages,
}

/// Messages for the Avatar component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Locale-aware initials extraction function.
    pub initials_fn: MessageFn<InitialsMessageFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            initials_fn: MessageFn::new(default_initials),
        }
    }
}

impl ComponentMessages for Messages {}

/// Renderable fallback content resolved by the Avatar API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackContent {
    /// Text initials derived from the avatar name.
    Initials(String),

    /// Default icon fallback when no initials are available.
    Icon,
}

/// Structural parts exposed by the Avatar connect API.
#[derive(ComponentPart)]
#[scope = "avatar"]
pub enum Part {
    /// The root avatar element.
    Root,

    /// The image element.
    Image,

    /// The fallback element.
    Fallback,
}

/// Structural parts exposed by the Avatar group API.
#[derive(ComponentPart)]
#[scope = "avatar"]
pub enum GroupPart {
    /// The avatar stack group element.
    Group,

    /// A child item in an avatar stack.
    GroupItem {
        /// Zero-based index in the avatar stack.
        index: usize,
    },
}

/// Machine for the Avatar component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = ars_core::NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let has_src = props.src.is_some();

        let state = if has_src {
            State::Loading
        } else {
            State::Fallback
        };

        let loading_status = if has_src {
            LoadingStatus::Loading
        } else {
            LoadingStatus::Error
        };

        (
            state,
            Context {
                src: props.src.clone(),
                loading_status,
                fallback_visible: !has_src || props.fallback_delay == Duration::ZERO,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Loading, Event::ImageLoad) => Some(TransitionPlan::to(State::Loaded).apply(
                |ctx: &mut Context| {
                    ctx.loading_status = LoadingStatus::Loaded;
                    ctx.fallback_visible = false;
                },
            )),

            (State::Loading, Event::ImageError) => {
                Some(TransitionPlan::to(State::Error).apply(|ctx: &mut Context| {
                    ctx.loading_status = LoadingStatus::Error;
                    ctx.fallback_visible = true;
                }))
            }

            (State::Loading, Event::FallbackDelayElapsed) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.fallback_visible = true;
                }))
            }

            (_, Event::SetSrc(Some(src))) => {
                let src = src.clone();
                let fallback_visible = props.fallback_delay == Duration::ZERO;
                Some(
                    TransitionPlan::to(State::Loading).apply(move |ctx: &mut Context| {
                        ctx.src = Some(src);
                        ctx.loading_status = LoadingStatus::Loading;
                        ctx.fallback_visible = fallback_visible;
                    }),
                )
            }

            (_, Event::SetSrc(None)) => Some(TransitionPlan::to(State::Fallback).apply(
                |ctx: &mut Context| {
                    ctx.src = None;
                    ctx.loading_status = LoadingStatus::Error;
                    ctx.fallback_visible = true;
                },
            )),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.src == new.src {
            Vec::new()
        } else {
            alloc::vec![Event::SetSrc(new.src.clone())]
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// API for the Avatar component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("avatar::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Extracts initials from the current avatar name.
    #[must_use]
    pub fn initials(&self) -> String {
        if let Some(name) = &self.props.name {
            if let Some(get_initials) = &self.props.get_initials {
                return get_initials(name.clone());
            }

            return (self.ctx.messages.initials_fn)(name, &self.ctx.locale);
        }

        String::new()
    }

    /// Returns the fallback content adapters should render.
    #[must_use]
    pub fn fallback_content(&self) -> FallbackContent {
        let initials = self.initials();

        if initials.is_empty() {
            FallbackContent::Icon
        } else {
            FallbackContent::Initials(initials)
        }
    }

    /// Returns whether the image is visible.
    #[must_use]
    pub const fn is_image_visible(&self) -> bool {
        matches!(self.state, State::Loaded)
    }

    /// Returns whether the fallback is visible.
    #[must_use]
    pub const fn is_fallback_visible(&self) -> bool {
        self.ctx.fallback_visible || matches!(self.state, State::Error | State::Fallback)
    }

    /// Returns root attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "img")
            .set(HtmlAttr::Data("ars-shape"), self.props.shape.as_str())
            .set(HtmlAttr::Data("ars-size"), self.props.size.as_str())
            .set(HtmlAttr::Data("ars-state"), state_attr(*self.state));

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.as_str());
        }

        if let Some(name) = &self.props.name {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                self.props.aria_label.as_deref().unwrap_or(name.as_str()),
            );
        } else if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }

        attrs
    }

    /// Returns image attributes.
    #[must_use]
    pub fn image_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Image.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Alt, "")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(src) = &self.ctx.src {
            attrs.set(HtmlAttr::Src, src.as_str());
        }

        if !self.is_image_visible() {
            attrs.set_style(CssProperty::Display, "none");
        }

        attrs
    }

    /// Returns fallback attributes.
    #[must_use]
    pub fn fallback_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Fallback.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(
                HtmlAttr::Data("ars-fallback"),
                match self.fallback_content() {
                    FallbackContent::Initials(_) => "initials",
                    FallbackContent::Icon => "icon",
                },
            );

        if !self.is_fallback_visible() {
            attrs.set_style(CssProperty::Display, "none");
        }

        attrs
    }

    /// Dispatches an image-load event.
    pub fn on_image_load(&self) {
        (self.send)(Event::ImageLoad);
    }

    /// Dispatches an image-error event.
    pub fn on_image_error(&self) {
        (self.send)(Event::ImageError);
    }

    /// Dispatches the fallback-delay elapsed event.
    pub fn on_fallback_delay_elapsed(&self) {
        (self.send)(Event::FallbackDelayElapsed);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Image => self.image_attrs(),
            Part::Fallback => self.fallback_attrs(),
        }
    }
}

/// Props for an avatar stack group.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct GroupProps {
    /// Component instance ID.
    pub id: String,

    /// Visual size token applied to all grouped avatars.
    pub size: Size,

    /// Visual shape token applied to all grouped avatars.
    pub shape: Shape,

    /// CSS overlap amount between adjacent avatars.
    pub overlap: String,

    /// Accessible label for the avatar group.
    pub aria_label: Option<String>,
}

impl Default for GroupProps {
    fn default() -> Self {
        Self {
            id: String::new(),
            size: Size::Md,
            shape: Shape::Circle,
            overlap: String::from("0.5rem"),
            aria_label: None,
        }
    }
}

impl GroupProps {
    /// Returns fresh avatar group props with the documented defaults.
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

    /// Sets the visual size token for grouped avatars.
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the visual shape token for grouped avatars.
    #[must_use]
    pub const fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Sets the CSS overlap amount between adjacent avatars.
    #[must_use]
    pub fn overlap(mut self, overlap: impl Into<String>) -> Self {
        self.overlap = overlap.into();
        self
    }

    /// Sets the accessible label for the avatar group.
    #[must_use]
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Clears the accessible label for the avatar group.
    #[must_use]
    pub fn no_aria_label(mut self) -> Self {
        self.aria_label = None;
        self
    }
}

/// API for avatar stack group attributes.
pub struct GroupApi {
    props: GroupProps,
}

impl Debug for GroupApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("avatar::GroupApi")
            .field("props", &self.props)
            .finish()
    }
}

impl GroupApi {
    /// Creates a new avatar group API.
    ///
    /// ```
    /// use ars_components::data_display::avatar::{GroupApi, GroupProps, Size};
    /// use ars_core::{CssProperty, HtmlAttr};
    ///
    /// let api = GroupApi::new(GroupProps::new().id("team").size(Size::Lg));
    /// let attrs = api.group_attrs();
    ///
    /// assert_eq!(attrs.get(&HtmlAttr::Id), Some("team"));
    /// assert!(attrs.styles().contains(&(
    ///     CssProperty::Custom("ars-avatar-group-overlap"),
    ///     String::from("0.5rem"),
    /// )));
    /// ```
    #[must_use]
    pub const fn new(props: GroupProps) -> Self {
        Self { props }
    }

    /// Returns avatar group root attributes.
    #[must_use]
    pub fn group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = GroupPart::Group.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-size"), self.props.size.as_str())
            .set(HtmlAttr::Data("ars-shape"), self.props.shape.as_str())
            .set_style(
                CssProperty::Custom("ars-avatar-group-overlap"),
                self.props.overlap.as_str(),
            );

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.as_str());
        }

        if let Some(label) = &self.props.aria_label {
            attrs
                .set(HtmlAttr::Role, "group")
                .set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }

        attrs
    }

    /// Returns attributes for a child avatar group item.
    #[must_use]
    pub fn group_item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            GroupPart::GroupItem { index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set_style(
                CssProperty::Custom("ars-avatar-group-index"),
                index.to_string(),
            );

        attrs
    }
}

const fn state_attr(state: State) -> &'static str {
    match state {
        State::Loading => "loading",
        State::Loaded => "loaded",
        State::Error => "error",
        State::Fallback => "fallback",
    }
}

fn default_initials(name: &str, locale: &Locale) -> String {
    if matches!(locale.language(), "zh" | "ja" | "ko") {
        return take_graphemes(name, 2);
    }

    let parts = name.split_whitespace().collect::<Vec<_>>();

    match parts.as_slice() {
        [] => String::new(),

        [single] => take_graphemes(single, 1).to_uppercase(),

        [first, .., last] => {
            let first = take_graphemes(first, 1);
            let last = take_graphemes(last, 1);

            format!("{first}{last}").to_uppercase()
        }
    }
}

fn is_allowed_image_src(src: &str) -> bool {
    SafeUrl::new(src).is_ok() || is_blob_url(src) || is_raster_data_image_url(src)
}

fn is_blob_url(src: &str) -> bool {
    src.trim_start().starts_with("blob:")
}

fn is_raster_data_image_url(src: &str) -> bool {
    let lower = src.trim_start().to_ascii_lowercase();

    ["png", "jpeg", "jpg", "gif", "webp", "avif"]
        .iter()
        .any(|kind| lower.starts_with(&format!("data:image/{kind};base64,")))
}

#[cfg(test)]
mod tests {
    use alloc::{format, sync::Arc, vec::Vec};

    use ars_core::{
        AriaAttr, Callback, ComponentPart as _, ConnectApi, CssProperty, HtmlAttr, Machine as _,
        SafeUrl, Service,
    };
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new()
            .id("avatar-1")
            .src("/avatar.png")
            .name("Ada Lovelace")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages::default())
    }

    fn send(service: &mut Service<Machine>, event: Event) {
        drop(service.send(event));
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let callback = Callback::new(|name: String| format!("custom:{name}"));

        let props = Props::new()
            .id("avatar-1")
            .src(SafeUrl::from_static("/avatar.png"))
            .name("Ada Lovelace")
            .aria_label("Ada profile photo")
            .fallback_delay(Duration::from_millis(250))
            .size(Size::Xl)
            .shape(Shape::Square)
            .get_initials(callback.clone());

        assert_eq!(props.id, "avatar-1");
        assert_eq!(
            props.src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(props.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(props.aria_label.as_deref(), Some("Ada profile photo"));
        assert_eq!(props.fallback_delay, Duration::from_millis(250));
        assert_eq!(props.size, Size::Xl);
        assert_eq!(props.shape, Shape::Square);
        assert_eq!(props.get_initials, Some(callback.clone()));

        let cleared = props.no_aria_label();

        assert_eq!(cleared.id, "avatar-1");
        assert_eq!(
            cleared.src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(cleared.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(cleared.fallback_delay, Duration::from_millis(250));
        assert_eq!(cleared.size, Size::Xl);
        assert_eq!(cleared.shape, Shape::Square);
        assert_eq!(cleared.get_initials, Some(callback));
        assert_eq!(cleared.aria_label, None);
    }

    #[test]
    fn props_builder_try_src_rejects_unsafe_urls() {
        let error = Props::new()
            .try_src("javascript:alert(1)")
            .expect_err("unsafe avatar src is rejected");

        assert_eq!(
            error.to_string(),
            "unsafe image source: javascript:alert(1)"
        );
    }

    #[test]
    fn props_builder_try_src_accepts_avatar_image_sources() {
        let blob = Props::new()
            .try_src("blob:https://example.com/avatar")
            .expect("blob URLs are valid image sources");

        assert_eq!(
            blob.src.as_ref().map(ImageSrc::as_str),
            Some("blob:https://example.com/avatar")
        );

        let data = Props::new()
            .try_src("data:image/png;base64,iVBORw0KGgo=")
            .expect("raster data image URLs are valid image sources");

        assert_eq!(
            data.src.as_ref().map(ImageSrc::as_str),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );

        for kind in ["png", "jpeg", "jpg", "gif", "webp", "avif"] {
            let src = format!("data:image/{kind};base64,AAAA");

            let props = Props::new()
                .try_src(src.clone())
                .expect("supported raster data image URLs are valid");

            assert_eq!(props.src.as_ref().map(ImageSrc::as_str), Some(src.as_str()));
        }
    }

    #[test]
    fn props_builder_try_src_rejects_svg_data_urls() {
        let error = Props::new()
            .try_src("data:image/svg+xml,<svg></svg>")
            .expect_err("svg data URLs are rejected");

        assert_eq!(
            error.to_string(),
            "unsafe image source: data:image/svg+xml,<svg></svg>"
        );
    }

    #[test]
    fn props_builder_try_src_rejects_malformed_data_image_urls() {
        for src in [
            "data:image/png,AAAA",
            "data:image/png;charset=utf-8;base64,AAAA",
            "data:text/plain;base64,AAAA",
        ] {
            assert!(
                Props::new().try_src(src).is_err(),
                "{src} must not be accepted as an avatar image source"
            );
        }
    }

    #[test]
    fn public_wrapper_helpers_and_debug_impls_are_exercised() {
        let image = ImageSrc::from_static("/avatar.png");

        assert_eq!(image.to_string(), "/avatar.png");

        let callback = Callback::new(|name: String| name);

        let props = Props::new()
            .id("avatar-1")
            .src("/avatar.png")
            .name("Ada Lovelace")
            .aria_label("Ada profile")
            .fallback_delay(Duration::from_millis(250))
            .size(Size::Lg)
            .shape(Shape::Square)
            .get_initials(callback);

        let no_src = props.clone().no_src();

        assert_eq!(no_src.src, None);
        assert_eq!(no_src.id, "avatar-1");
        assert_eq!(no_src.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(no_src.aria_label.as_deref(), Some("Ada profile"));
        assert_eq!(no_src.fallback_delay, Duration::from_millis(250));
        assert_eq!(no_src.size, Size::Lg);
        assert_eq!(no_src.shape, Shape::Square);

        let no_name = props.clone().no_name();

        assert_eq!(no_name.name, None);
        assert_eq!(
            no_name.src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(no_name.id, "avatar-1");
        assert_eq!(no_name.aria_label.as_deref(), Some("Ada profile"));
        assert_eq!(no_name.fallback_delay, Duration::from_millis(250));
        assert_eq!(no_name.size, Size::Lg);
        assert_eq!(no_name.shape, Shape::Square);

        let no_get_initials = props.no_get_initials();

        assert_eq!(no_get_initials.get_initials, None);
        assert_eq!(
            no_get_initials.src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(no_get_initials.id, "avatar-1");
        assert_eq!(no_get_initials.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(no_get_initials.aria_label.as_deref(), Some("Ada profile"));
        assert_eq!(no_get_initials.fallback_delay, Duration::from_millis(250));
        assert_eq!(no_get_initials.size, Size::Lg);
        assert_eq!(no_get_initials.shape, Shape::Square);

        let service = service(Props::new().id("avatar-1"));
        let api_debug = format!("{:?}", service.connect(&|_| {}));

        assert!(api_debug.contains("avatar::Api"));
        assert!(api_debug.contains("state"));

        let group_props = GroupProps::new()
            .id("group-1")
            .size(Size::Lg)
            .shape(Shape::Square)
            .overlap("0.75rem")
            .aria_label("Team")
            .no_aria_label();

        assert_eq!(group_props.id, "group-1");
        assert_eq!(group_props.size, Size::Lg);
        assert_eq!(group_props.shape, Shape::Square);
        assert_eq!(group_props.overlap, "0.75rem");
        assert_eq!(group_props.aria_label, None);

        let group = GroupApi::new(group_props);
        let group_debug = format!("{group:?}");

        assert!(group_debug.contains("avatar::GroupApi"));
        assert_eq!(
            group.group_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
            None
        );
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn loading_initial_state_when_src_present() {
        let service = service(props());

        assert_eq!(service.state(), &State::Loading);
        assert_eq!(
            service.context().src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(service.context().loading_status, LoadingStatus::Loading);
        assert!(!service.context().fallback_visible);
    }

    #[test]
    fn zero_fallback_delay_shows_fallback_while_initial_src_loads() {
        let service = service(props().fallback_delay(Duration::ZERO));

        assert_eq!(service.state(), &State::Loading);
        assert_eq!(
            service.context().src.as_ref().map(ImageSrc::as_str),
            Some("/avatar.png")
        );
        assert_eq!(service.context().loading_status, LoadingStatus::Loading);
        assert!(service.context().fallback_visible);
    }

    #[test]
    fn fallback_initial_state_when_src_absent() {
        let service = service(Props::new().id("avatar-1").name("Ada Lovelace"));

        assert_eq!(service.state(), &State::Fallback);
        assert_eq!(service.context().loading_status, LoadingStatus::Error);
        assert!(service.context().fallback_visible);
    }

    #[test]
    fn image_load_transitions_to_loaded() {
        let mut service = service(props());

        send(&mut service, Event::ImageLoad);

        assert_eq!(service.state(), &State::Loaded);
        assert_eq!(service.context().loading_status, LoadingStatus::Loaded);
        assert!(!service.context().fallback_visible);
    }

    #[test]
    fn image_error_transitions_to_error_and_shows_fallback() {
        let mut service = service(props());

        send(&mut service, Event::ImageError);

        assert_eq!(service.state(), &State::Error);
        assert_eq!(service.context().loading_status, LoadingStatus::Error);
        assert!(service.context().fallback_visible);
    }

    #[test]
    fn set_src_some_restarts_loading() {
        let mut service = service(Props::new().id("avatar-1").name("Ada Lovelace"));

        send(
            &mut service,
            Event::SetSrc(Some(ImageSrc::from_static("/next.png"))),
        );

        assert_eq!(service.state(), &State::Loading);
        assert_eq!(
            service.context().src.as_ref().map(ImageSrc::as_str),
            Some("/next.png")
        );
        assert_eq!(service.context().loading_status, LoadingStatus::Loading);
        assert!(!service.context().fallback_visible);
    }

    #[test]
    fn set_src_some_with_zero_fallback_delay_shows_fallback_while_loading() {
        let mut service = service(
            Props::new()
                .id("avatar-1")
                .name("Ada Lovelace")
                .fallback_delay(Duration::ZERO),
        );

        send(
            &mut service,
            Event::SetSrc(Some(ImageSrc::from_static("/next.png"))),
        );

        assert_eq!(service.state(), &State::Loading);
        assert_eq!(
            service.context().src.as_ref().map(ImageSrc::as_str),
            Some("/next.png")
        );
        assert_eq!(service.context().loading_status, LoadingStatus::Loading);
        assert!(service.context().fallback_visible);
    }

    #[test]
    fn set_src_none_clears_image_and_shows_fallback() {
        let mut service = service(props());

        send(&mut service, Event::SetSrc(None));

        assert_eq!(service.state(), &State::Fallback);
        assert_eq!(service.context().src, None);
        assert_eq!(service.context().loading_status, LoadingStatus::Error);
        assert!(service.context().fallback_visible);
    }

    #[test]
    fn fallback_delay_elapsed_reveals_fallback_while_loading() {
        let mut service = service(props());

        send(&mut service, Event::FallbackDelayElapsed);

        assert_eq!(service.state(), &State::Loading);
        assert_eq!(service.context().loading_status, LoadingStatus::Loading);
        assert!(service.context().fallback_visible);
        assert!(service.connect(&|_| {}).is_fallback_visible());
    }

    #[test]
    fn non_loading_image_events_and_delay_elapsed_are_noops() {
        for setup in [Event::ImageLoad, Event::ImageError, Event::SetSrc(None)] {
            let mut service = service(props());
            send(&mut service, setup);

            let state = *service.state();

            let src = service.context().src.clone();

            let loading_status = service.context().loading_status;

            let fallback_visible = service.context().fallback_visible;

            send(&mut service, Event::ImageLoad);

            assert_eq!(service.state(), &state);
            assert_eq!(service.context().src, src);
            assert_eq!(service.context().loading_status, loading_status);
            assert_eq!(service.context().fallback_visible, fallback_visible);

            send(&mut service, Event::ImageError);

            assert_eq!(service.state(), &state);
            assert_eq!(service.context().src, src);
            assert_eq!(service.context().loading_status, loading_status);
            assert_eq!(service.context().fallback_visible, fallback_visible);

            send(&mut service, Event::FallbackDelayElapsed);

            assert_eq!(service.state(), &state);
            assert_eq!(service.context().src, src);
            assert_eq!(service.context().loading_status, loading_status);
            assert_eq!(service.context().fallback_visible, fallback_visible);
        }
    }

    #[test]
    fn on_props_changed_emits_set_src_only_when_src_changes() {
        let old = props();

        let renamed = Props::new()
            .id("avatar-1")
            .src("/avatar.png")
            .name("Grace Hopper");

        let next = Props::new()
            .id("avatar-1")
            .src("/next.png")
            .name("Ada Lovelace");

        assert!(Machine::on_props_changed(&old, &renamed).is_empty());
        assert_eq!(
            Machine::on_props_changed(&old, &next),
            [Event::SetSrc(Some(ImageSrc::from_static("/next.png")))]
        );
    }

    #[test]
    fn initials_cover_names_locales_and_overrides() {
        let default = service(props());

        assert_eq!(default.connect(&|_| {}).initials(), "AL");

        let mononym = service(Props::new().id("avatar-1").name("Prince"));

        assert_eq!(mononym.connect(&|_| {}).initials(), "P");

        let cjk_env = Env {
            locale: Locale::parse("zh-Hans").expect("zh-Hans parses"),
            ..Env::default()
        };

        let cjk = Service::<Machine>::new(
            Props::new().id("avatar-1").name("张伟明"),
            &cjk_env,
            &Messages::default(),
        );

        assert_eq!(cjk.connect(&|_| {}).initials(), "张伟");

        let combining_mark = service(Props::new().id("avatar-1").name("e\u{301}mile Zola"));

        assert_eq!(combining_mark.connect(&|_| {}).initials(), "E\u{301}Z");

        let emoji_cluster = service(
            Props::new()
                .id("avatar-1")
                .name("👨\u{200d}👩\u{200d}👧 Family"),
        );

        assert_eq!(
            emoji_cluster.connect(&|_| {}).initials(),
            "👨\u{200d}👩\u{200d}👧F"
        );

        let messages = Messages {
            initials_fn: MessageFn::new(|name: &str, locale: &Locale| {
                format!("{name}:{}", locale.language())
            }),
        };

        let custom_messages = Service::<Machine>::new(props(), &Env::default(), &messages);

        assert_eq!(
            custom_messages.connect(&|_| {}).initials(),
            "Ada Lovelace:en"
        );

        let custom_prop =
            service(props().get_initials(Callback::new(|name: String| format!("{name}:prop"))));

        assert_eq!(custom_prop.connect(&|_| {}).initials(), "Ada Lovelace:prop");

        let empty = service(Props::new().id("avatar-1"));

        assert_eq!(empty.connect(&|_| {}).initials(), "");
    }

    #[test]
    fn fallback_content_uses_initials_then_icon() {
        assert_eq!(
            service(props()).connect(&|_| {}).fallback_content(),
            FallbackContent::Initials("AL".into())
        );
        assert_eq!(
            service(Props::new().id("avatar-1"))
                .connect(&|_| {})
                .fallback_content(),
            FallbackContent::Icon
        );
    }

    #[test]
    fn every_size_and_shape_emits_expected_data_attr() {
        for (size, expected) in [
            (Size::Xs, "xs"),
            (Size::Sm, "sm"),
            (Size::Md, "md"),
            (Size::Lg, "lg"),
            (Size::Xl, "xl"),
        ] {
            assert_eq!(size.as_str(), expected);

            let attrs = service(props().size(size)).connect(&|_| {}).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-size")), Some(expected));
        }

        for (shape, expected) in [(Shape::Circle, "circle"), (Shape::Square, "square")] {
            assert_eq!(shape.as_str(), expected);

            let attrs = service(props().shape(shape)).connect(&|_| {}).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-shape")), Some(expected));
        }
    }

    #[test]
    fn root_attrs_emit_accessible_name_and_state() {
        let attrs = service(props()).connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("img"));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("avatar-1"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Ada Lovelace")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("loading"));

        let explicit = service(props().aria_label("Ada profile photo"))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(
            explicit.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Ada profile photo")
        );

        let nameless = service(Props::new().id("avatar-1").aria_label("User photo"))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(
            nameless.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("User photo")
        );
    }

    #[test]
    fn image_and_fallback_attrs_reflect_visibility() {
        let mut loaded = service(props());

        send(&mut loaded, Event::ImageLoad);

        let api = loaded.connect(&|_| {});

        let image = api.image_attrs();

        assert_eq!(image.get(&HtmlAttr::Src), Some("/avatar.png"));
        assert_eq!(image.get(&HtmlAttr::Alt), Some(""));
        assert_eq!(image.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(
            api.fallback_attrs().get(&HtmlAttr::Data("ars-fallback")),
            Some("initials")
        );

        let mut errored = service(props());

        send(&mut errored, Event::ImageError);

        let api = errored.connect(&|_| {});

        let image = api.image_attrs();

        assert_eq!(image.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert!(
            image
                .styles()
                .contains(&(CssProperty::Display, String::from("none")))
        );
    }

    #[test]
    fn event_helpers_dispatch_typed_events() {
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let send = move |event| {
            captured.lock().expect("events lock").push(event);
        };

        let service = service(props());

        let api = service.connect(&send);

        api.on_image_load();
        api.on_image_error();
        api.on_fallback_delay_elapsed();

        assert_eq!(
            *events.lock().expect("events lock"),
            [
                Event::ImageLoad,
                Event::ImageError,
                Event::FallbackDelayElapsed
            ]
        );
    }

    #[test]
    fn group_props_builder_and_attrs() {
        let props = GroupProps::new()
            .id("group-1")
            .size(Size::Lg)
            .shape(Shape::Square)
            .overlap("0.75rem")
            .aria_label("Team members");

        let api = GroupApi::new(props.clone());

        assert_eq!(props.id, "group-1");
        assert_eq!(props.size, Size::Lg);
        assert_eq!(props.shape, Shape::Square);
        assert_eq!(props.overlap, "0.75rem");
        assert_eq!(props.aria_label.as_deref(), Some("Team members"));

        let attrs = api.group_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("group"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Team members")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-size")), Some("lg"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-shape")), Some("square"));
        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-avatar-group-overlap"),
            String::from("0.75rem")
        )));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("group-1"));

        let item = api.group_item_attrs(2);

        assert_eq!(item.get(&HtmlAttr::Data("ars-index")), Some("2"));
        assert!(item.styles().contains(&(
            CssProperty::Custom("ars-avatar-group-index"),
            String::from("2")
        )));
    }

    #[test]
    fn group_part_metadata_covers_root_and_indexed_item() {
        assert_eq!(GroupPart::scope(), "avatar");
        assert_eq!(GroupPart::ROOT, GroupPart::Group);
        assert_eq!(
            GroupPart::all(),
            [GroupPart::Group, GroupPart::GroupItem { index: 0 }]
        );

        assert_eq!(
            GroupPart::Group.data_attrs(),
            [
                (HtmlAttr::Data("ars-scope"), "avatar"),
                (HtmlAttr::Data("ars-part"), "group"),
            ]
        );
        assert_eq!(
            GroupPart::GroupItem { index: 7 }.data_attrs(),
            [
                (HtmlAttr::Data("ars-scope"), "avatar"),
                (HtmlAttr::Data("ars-part"), "group-item"),
            ]
        );
    }

    #[test]
    fn part_attrs_delegates_for_all_parts() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Image), api.image_attrs());
        assert_eq!(api.part_attrs(Part::Fallback), api.fallback_attrs());
    }

    #[test]
    fn avatar_attr_snapshots() {
        let loading = service(props());

        assert_snapshot!(
            "avatar_root_loading",
            snapshot_attrs(&loading.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "avatar_image_hidden",
            snapshot_attrs(&loading.connect(&|_| {}).image_attrs())
        );
        assert_snapshot!(
            "avatar_fallback_initials_hidden",
            snapshot_attrs(&loading.connect(&|_| {}).fallback_attrs())
        );

        let mut loaded = service(props());

        send(&mut loaded, Event::ImageLoad);

        assert_snapshot!(
            "avatar_root_loaded",
            snapshot_attrs(&loaded.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "avatar_image_visible",
            snapshot_attrs(&loaded.connect(&|_| {}).image_attrs())
        );

        let mut errored = service(props());

        send(&mut errored, Event::ImageError);

        assert_snapshot!(
            "avatar_root_error",
            snapshot_attrs(&errored.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "avatar_fallback_initials_visible",
            snapshot_attrs(&errored.connect(&|_| {}).fallback_attrs())
        );

        let fallback = service(Props::new().id("avatar-1"));

        assert_snapshot!(
            "avatar_root_fallback",
            snapshot_attrs(&fallback.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "avatar_fallback_icon_visible",
            snapshot_attrs(&fallback.connect(&|_| {}).fallback_attrs())
        );
    }

    #[test]
    fn avatar_variant_and_group_snapshots() {
        assert_snapshot!(
            "avatar_root_xs_square",
            snapshot_attrs(
                &service(props().size(Size::Xs).shape(Shape::Square))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
        assert_snapshot!(
            "avatar_group_root",
            snapshot_attrs(
                &GroupApi::new(
                    GroupProps::new()
                        .id("group-1")
                        .size(Size::Lg)
                        .shape(Shape::Square)
                        .overlap("0.75rem")
                        .aria_label("Team members")
                )
                .group_attrs()
            )
        );
        assert_snapshot!(
            "avatar_group_item",
            snapshot_attrs(&GroupApi::new(GroupProps::new()).group_item_attrs(2))
        );
    }
}
