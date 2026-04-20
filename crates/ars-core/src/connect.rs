//! Typed connect primitives used by component `connect()` APIs.
//!
//! This module defines the typed HTML attribute, DOM event, CSS property, and
//! attribute-map contracts used by the architecture specification. They provide
//! a framework-agnostic vocabulary for converting machine state into DOM-facing
//! metadata without relying on raw string literals throughout the codebase.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Display};

/// Typed `aria-*` attribute names used by [`HtmlAttr::Aria`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AriaAttr {
    /// `aria-activedescendant`
    ActiveDescendant,

    /// `aria-autocomplete`
    AutoComplete,

    /// `aria-checked`
    Checked,

    /// `aria-disabled`
    Disabled,

    /// `aria-errormessage`
    ErrorMessage,

    /// `aria-expanded`
    Expanded,

    /// `aria-haspopup`
    HasPopup,

    /// `aria-hidden`
    Hidden,

    /// `aria-invalid`
    Invalid,

    /// `aria-keyshortcuts`
    KeyShortcuts,

    /// `aria-label`
    Label,

    /// `aria-labelledby`
    LabelledBy,

    /// `aria-level`
    Level,

    /// `aria-modal`
    Modal,

    /// `aria-multiline`
    MultiLine,

    /// `aria-multiselectable`
    MultiSelectable,

    /// `aria-orientation`
    Orientation,

    /// `aria-placeholder`
    Placeholder,

    /// `aria-pressed`
    Pressed,

    /// `aria-readonly`
    ReadOnly,

    /// `aria-required`
    Required,

    /// `aria-roledescription`
    RoleDescription,

    /// `aria-selected`
    Selected,

    /// `aria-sort`
    Sort,

    /// `aria-valuemax`
    ValueMax,

    /// `aria-valuemin`
    ValueMin,

    /// `aria-valuenow`
    ValueNow,

    /// `aria-valuetext`
    ValueText,

    /// `aria-atomic`
    Atomic,

    /// `aria-busy`
    Busy,

    /// `aria-live`
    Live,

    /// `aria-relevant`
    Relevant,

    /// `aria-dropeffect`
    DropEffect,

    /// `aria-grabbed`
    Grabbed,

    /// `aria-colcount`
    ColCount,

    /// `aria-colindex`
    ColIndex,

    /// `aria-colspan`
    ColSpan,

    /// `aria-controls`
    Controls,

    /// `aria-current`
    Current,

    /// `aria-describedby`
    DescribedBy,

    /// `aria-description`
    Description,

    /// `aria-details`
    Details,

    /// `aria-flowto`
    FlowTo,

    /// `aria-owns`
    Owns,

    /// `aria-posinset`
    PosInSet,

    /// `aria-rowcount`
    RowCount,

    /// `aria-rowindex`
    RowIndex,

    /// `aria-rowspan`
    RowSpan,

    /// `aria-setsize`
    SetSize,
}

impl AriaAttr {
    /// Returns the HTML attribute spelling for this ARIA discriminant.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ActiveDescendant => "aria-activedescendant",
            Self::AutoComplete => "aria-autocomplete",
            Self::Checked => "aria-checked",
            Self::Disabled => "aria-disabled",
            Self::ErrorMessage => "aria-errormessage",
            Self::Expanded => "aria-expanded",
            Self::HasPopup => "aria-haspopup",
            Self::Hidden => "aria-hidden",
            Self::Invalid => "aria-invalid",
            Self::KeyShortcuts => "aria-keyshortcuts",
            Self::Label => "aria-label",
            Self::LabelledBy => "aria-labelledby",
            Self::Level => "aria-level",
            Self::Modal => "aria-modal",
            Self::MultiLine => "aria-multiline",
            Self::MultiSelectable => "aria-multiselectable",
            Self::Orientation => "aria-orientation",
            Self::Placeholder => "aria-placeholder",
            Self::Pressed => "aria-pressed",
            Self::ReadOnly => "aria-readonly",
            Self::Required => "aria-required",
            Self::RoleDescription => "aria-roledescription",
            Self::Selected => "aria-selected",
            Self::Sort => "aria-sort",
            Self::ValueMax => "aria-valuemax",
            Self::ValueMin => "aria-valuemin",
            Self::ValueNow => "aria-valuenow",
            Self::ValueText => "aria-valuetext",
            Self::Atomic => "aria-atomic",
            Self::Busy => "aria-busy",
            Self::Live => "aria-live",
            Self::Relevant => "aria-relevant",
            Self::DropEffect => "aria-dropeffect",
            Self::Grabbed => "aria-grabbed",
            Self::ColCount => "aria-colcount",
            Self::ColIndex => "aria-colindex",
            Self::ColSpan => "aria-colspan",
            Self::Controls => "aria-controls",
            Self::Current => "aria-current",
            Self::DescribedBy => "aria-describedby",
            Self::Description => "aria-description",
            Self::Details => "aria-details",
            Self::FlowTo => "aria-flowto",
            Self::Owns => "aria-owns",
            Self::PosInSet => "aria-posinset",
            Self::RowCount => "aria-rowcount",
            Self::RowIndex => "aria-rowindex",
            Self::RowSpan => "aria-rowspan",
            Self::SetSize => "aria-setsize",
        }
    }
}

impl Display for AriaAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed HTML attribute names used by `connect()` APIs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HtmlAttr {
    /// `data-*` attributes with a compile-time suffix.
    Data(&'static str),

    /// `aria-*` attributes backed by [`AriaAttr`].
    Aria(AriaAttr),

    /// `accesskey`
    AccessKey,

    /// `autocapitalize`
    AutoCapitalize,

    /// `autocorrect`
    AutoCorrect,

    /// `autofocus`
    AutoFocus,

    /// `class`
    Class,

    /// `contenteditable`
    ContentEditable,

    /// `dir`
    Dir,

    /// `draggable`
    Draggable,

    /// `enterkeyhint`
    EnterKeyHint,

    /// `hidden`
    Hidden,

    /// `id`
    Id,

    /// `inert`
    Inert,

    /// `inputmode`
    InputMode,

    /// `is`
    Is,

    /// `itemid`
    ItemId,

    /// `itemprop`
    ItemProp,

    /// `itemref`
    ItemRef,

    /// `itemscope`
    ItemScope,

    /// `itemtype`
    ItemType,

    /// `lang`
    Lang,

    /// `nonce`
    Nonce,

    /// `popover`
    Popover,

    /// `role`
    Role,

    /// `slot`
    Slot,

    /// `spellcheck`
    SpellCheck,

    /// `tabindex`
    TabIndex,

    /// `title`
    Title,

    /// `translate`
    Translate,

    /// `writingsuggestions`
    WritingSuggestions,

    /// `accept`
    Accept,

    /// `accept-charset`
    AcceptCharset,

    /// `action`
    Action,

    /// `alpha`
    Alpha,

    /// `autocomplete`
    AutoComplete,

    /// `capture`
    Capture,

    /// `checked`
    Checked,

    /// `cols`
    Cols,

    /// `colorspace`
    ColorSpace,

    /// `command`
    Command,

    /// `commandfor`
    CommandFor,

    /// `disabled`
    Disabled,

    /// `dirname`
    DirName,

    /// `enctype`
    EncType,

    /// `for`
    For,

    /// `form`
    Form,

    /// `formaction`
    FormAction,

    /// `formenctype`
    FormEncType,

    /// `formmethod`
    FormMethod,

    /// `formnovalidate`
    FormNoValidate,

    /// `formtarget`
    FormTarget,

    /// `high`
    High,

    /// `list`
    List,

    /// `low`
    Low,

    /// `max`
    Max,

    /// `maxlength`
    MaxLength,

    /// `method`
    Method,

    /// `min`
    Min,

    /// `minlength`
    MinLength,

    /// `multiple`
    Multiple,

    /// `name`
    Name,

    /// `novalidate`
    NoValidate,

    /// `optimum`
    Optimum,

    /// `pattern`
    Pattern,

    /// `placeholder`
    Placeholder,

    /// `readonly`
    ReadOnly,

    /// `required`
    Required,

    /// `rows`
    Rows,

    /// `selected`
    Selected,

    /// `size`
    Size,

    /// `step`
    Step,

    /// `type`
    Type,

    /// `value`
    Value,

    /// `wrap`
    Wrap,

    /// `as`
    As,

    /// `async`
    Async,

    /// `blocking`
    Blocking,

    /// `charset`
    Charset,

    /// `color`
    Color,

    /// `defer`
    Defer,

    /// `http-equiv`
    HttpEquiv,

    /// `imagesizes`
    ImageSizes,

    /// `imagesrcset`
    ImageSrcSet,

    /// `allow`
    Allow,

    /// `alt`
    Alt,

    /// `autoplay`
    AutoPlay,

    /// `controls`
    Controls,

    /// `crossorigin`
    CrossOrigin,

    /// `decoding`
    Decoding,

    /// `default`
    Default,

    /// `download`
    Download,

    /// `fetchpriority`
    FetchPriority,

    /// `height`
    Height,

    /// `href`
    Href,

    /// `hreflang`
    HrefLang,

    /// `integrity`
    Integrity,

    /// `ismap`
    IsMap,

    /// `kind`
    Kind,

    /// `label`
    Label,

    /// `loading`
    Loading,

    /// `loop`
    Loop,

    /// `media`
    Media,

    /// `muted`
    Muted,

    /// The object element's `data` attribute.
    ObjectData,

    /// `ping`
    Ping,

    /// `playsinline`
    PlaysInline,

    /// `poster`
    Poster,

    /// `preload`
    Preload,

    /// `referrerpolicy`
    ReferrerPolicy,

    /// `rel`
    Rel,

    /// `sandbox`
    Sandbox,

    /// `shape`
    Shape,

    /// `sizes`
    Sizes,

    /// `src`
    Src,

    /// `srcdoc`
    SrcDoc,

    /// `srclang`
    SrcLang,

    /// `srcset`
    SrcSet,

    /// `target`
    Target,

    /// `usemap`
    UseMap,

    /// `width`
    Width,

    /// `abbr`
    Abbr,

    /// `colspan`
    ColSpan,

    /// `headers`
    Headers,

    /// `rowspan`
    RowSpan,

    /// `scope`
    Scope,

    /// `span`
    Span,

    /// `shadowrootclonable`
    ShadowRootClonable,

    /// `shadowrootcustomelementregistry`
    ShadowRootCustomElementRegistry,

    /// `shadowrootdelegatesfocus`
    ShadowRootDelegatesFocus,

    /// `shadowrootmode`
    ShadowRootMode,

    /// `shadowrootserializable`
    ShadowRootSerializable,

    /// `cite`
    Cite,

    /// `closedby`
    ClosedBy,

    /// `content`
    Content,

    /// `coords`
    Coords,

    /// `datetime`
    DateTime,

    /// `open`
    Open,

    /// `reversed`
    Reversed,

    /// `start`
    Start,

    /// `summary`
    Summary,

    /// `webkitdirectory`
    WebkitDirectory,
}

impl HtmlAttr {
    /// Returns the static attribute name for non-`data-*` variants.
    ///
    /// `HtmlAttr::Data(_)` requires runtime formatting and therefore returns `None`.
    #[must_use]
    pub const fn static_name(&self) -> Option<&'static str> {
        match self {
            Self::Data(_) => None,
            Self::Aria(attr) => Some(attr.as_str()),
            Self::AccessKey => Some("accesskey"),
            Self::AutoCapitalize => Some("autocapitalize"),
            Self::AutoCorrect => Some("autocorrect"),
            Self::AutoFocus => Some("autofocus"),
            Self::Class => Some("class"),
            Self::ContentEditable => Some("contenteditable"),
            Self::Dir => Some("dir"),
            Self::Draggable => Some("draggable"),
            Self::EnterKeyHint => Some("enterkeyhint"),
            Self::Hidden => Some("hidden"),
            Self::Id => Some("id"),
            Self::Inert => Some("inert"),
            Self::InputMode => Some("inputmode"),
            Self::Is => Some("is"),
            Self::ItemId => Some("itemid"),
            Self::ItemProp => Some("itemprop"),
            Self::ItemRef => Some("itemref"),
            Self::ItemScope => Some("itemscope"),
            Self::ItemType => Some("itemtype"),
            Self::Lang => Some("lang"),
            Self::Nonce => Some("nonce"),
            Self::Popover => Some("popover"),
            Self::Role => Some("role"),
            Self::Slot => Some("slot"),
            Self::SpellCheck => Some("spellcheck"),
            Self::TabIndex => Some("tabindex"),
            Self::Title => Some("title"),
            Self::Translate => Some("translate"),
            Self::WritingSuggestions => Some("writingsuggestions"),
            Self::Accept => Some("accept"),
            Self::AcceptCharset => Some("accept-charset"),
            Self::Action => Some("action"),
            Self::Alpha => Some("alpha"),
            Self::AutoComplete => Some("autocomplete"),
            Self::Capture => Some("capture"),
            Self::Checked => Some("checked"),
            Self::Cols => Some("cols"),
            Self::ColorSpace => Some("colorspace"),
            Self::Command => Some("command"),
            Self::CommandFor => Some("commandfor"),
            Self::Disabled => Some("disabled"),
            Self::DirName => Some("dirname"),
            Self::EncType => Some("enctype"),
            Self::For => Some("for"),
            Self::Form => Some("form"),
            Self::FormAction => Some("formaction"),
            Self::FormEncType => Some("formenctype"),
            Self::FormMethod => Some("formmethod"),
            Self::FormNoValidate => Some("formnovalidate"),
            Self::FormTarget => Some("formtarget"),
            Self::High => Some("high"),
            Self::List => Some("list"),
            Self::Low => Some("low"),
            Self::Max => Some("max"),
            Self::MaxLength => Some("maxlength"),
            Self::Method => Some("method"),
            Self::Min => Some("min"),
            Self::MinLength => Some("minlength"),
            Self::Multiple => Some("multiple"),
            Self::Name => Some("name"),
            Self::NoValidate => Some("novalidate"),
            Self::Optimum => Some("optimum"),
            Self::Pattern => Some("pattern"),
            Self::Placeholder => Some("placeholder"),
            Self::ReadOnly => Some("readonly"),
            Self::Required => Some("required"),
            Self::Rows => Some("rows"),
            Self::Selected => Some("selected"),
            Self::Size => Some("size"),
            Self::Step => Some("step"),
            Self::Type => Some("type"),
            Self::Value => Some("value"),
            Self::Wrap => Some("wrap"),
            Self::As => Some("as"),
            Self::Async => Some("async"),
            Self::Blocking => Some("blocking"),
            Self::Charset => Some("charset"),
            Self::Color => Some("color"),
            Self::Defer => Some("defer"),
            Self::HttpEquiv => Some("http-equiv"),
            Self::ImageSizes => Some("imagesizes"),
            Self::ImageSrcSet => Some("imagesrcset"),
            Self::Allow => Some("allow"),
            Self::Alt => Some("alt"),
            Self::AutoPlay => Some("autoplay"),
            Self::Controls => Some("controls"),
            Self::CrossOrigin => Some("crossorigin"),
            Self::Decoding => Some("decoding"),
            Self::Default => Some("default"),
            Self::Download => Some("download"),
            Self::FetchPriority => Some("fetchpriority"),
            Self::Height => Some("height"),
            Self::Href => Some("href"),
            Self::HrefLang => Some("hreflang"),
            Self::Integrity => Some("integrity"),
            Self::IsMap => Some("ismap"),
            Self::Kind => Some("kind"),
            Self::Label => Some("label"),
            Self::Loading => Some("loading"),
            Self::Loop => Some("loop"),
            Self::Media => Some("media"),
            Self::Muted => Some("muted"),
            Self::ObjectData => Some("data"),
            Self::Ping => Some("ping"),
            Self::PlaysInline => Some("playsinline"),
            Self::Poster => Some("poster"),
            Self::Preload => Some("preload"),
            Self::ReferrerPolicy => Some("referrerpolicy"),
            Self::Rel => Some("rel"),
            Self::Sandbox => Some("sandbox"),
            Self::Shape => Some("shape"),
            Self::Sizes => Some("sizes"),
            Self::Src => Some("src"),
            Self::SrcDoc => Some("srcdoc"),
            Self::SrcLang => Some("srclang"),
            Self::SrcSet => Some("srcset"),
            Self::Target => Some("target"),
            Self::UseMap => Some("usemap"),
            Self::Width => Some("width"),
            Self::Abbr => Some("abbr"),
            Self::ColSpan => Some("colspan"),
            Self::Headers => Some("headers"),
            Self::RowSpan => Some("rowspan"),
            Self::Scope => Some("scope"),
            Self::Span => Some("span"),
            Self::ShadowRootClonable => Some("shadowrootclonable"),
            Self::ShadowRootCustomElementRegistry => Some("shadowrootcustomelementregistry"),
            Self::ShadowRootDelegatesFocus => Some("shadowrootdelegatesfocus"),
            Self::ShadowRootMode => Some("shadowrootmode"),
            Self::ShadowRootSerializable => Some("shadowrootserializable"),
            Self::Cite => Some("cite"),
            Self::ClosedBy => Some("closedby"),
            Self::Content => Some("content"),
            Self::Coords => Some("coords"),
            Self::DateTime => Some("datetime"),
            Self::Open => Some("open"),
            Self::Reversed => Some("reversed"),
            Self::Start => Some("start"),
            Self::Summary => Some("summary"),
            Self::WebkitDirectory => Some("webkitdirectory"),
        }
    }
}

impl Display for HtmlAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(suffix) => write!(f, "data-{suffix}"),
            _ => f.write_str(
                self.static_name()
                    .expect("non-data attributes have static names"),
            ),
        }
    }
}

/// Typed DOM event names used by adapter event wiring.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HtmlEvent {
    /// `auxclick`
    AuxClick,

    /// `click`
    Click,

    /// `contextmenu`
    ContextMenu,

    /// `dblclick`
    DblClick,

    /// `mousedown`
    MouseDown,

    /// `mouseenter`
    MouseEnter,

    /// `mouseleave`
    MouseLeave,

    /// `mousemove`
    MouseMove,

    /// `mouseout`
    MouseOut,

    /// `mouseover`
    MouseOver,

    /// `mouseup`
    MouseUp,

    /// `gotpointercapture`
    GotPointerCapture,

    /// `lostpointercapture`
    LostPointerCapture,

    /// `pointercancel`
    PointerCancel,

    /// `pointerdown`
    PointerDown,

    /// `pointerenter`
    PointerEnter,

    /// `pointerleave`
    PointerLeave,

    /// `pointermove`
    PointerMove,

    /// `pointerout`
    PointerOut,

    /// `pointerover`
    PointerOver,

    /// `pointerup`
    PointerUp,

    /// `keydown`
    KeyDown,

    /// `keyup`
    KeyUp,

    /// `blur`
    Blur,

    /// `focus`
    Focus,

    /// `focusin`
    FocusIn,

    /// `focusout`
    FocusOut,

    /// `change`
    Change,

    /// `input`
    Input,

    /// `beforeinput`
    BeforeInput,

    /// `invalid`
    Invalid,

    /// `reset`
    Reset,

    /// `select`
    Select,

    /// `submit`
    Submit,

    /// `drag`
    Drag,

    /// `dragend`
    DragEnd,

    /// `dragenter`
    DragEnter,

    /// `dragleave`
    DragLeave,

    /// `dragover`
    DragOver,

    /// `dragstart`
    DragStart,

    /// `drop`
    Drop,

    /// `touchcancel`
    TouchCancel,

    /// `touchend`
    TouchEnd,

    /// `touchmove`
    TouchMove,

    /// `touchstart`
    TouchStart,

    /// `scroll`
    Scroll,

    /// `scrollend`
    ScrollEnd,

    /// `wheel`
    Wheel,

    /// `copy`
    Copy,

    /// `cut`
    Cut,

    /// `paste`
    Paste,

    /// `compositionend`
    CompositionEnd,

    /// `compositionstart`
    CompositionStart,

    /// `compositionupdate`
    CompositionUpdate,

    /// `animationcancel`
    AnimationCancel,

    /// `animationend`
    AnimationEnd,

    /// `animationiteration`
    AnimationIteration,

    /// `animationstart`
    AnimationStart,

    /// `transitioncancel`
    TransitionCancel,

    /// `transitionend`
    TransitionEnd,

    /// `transitionrun`
    TransitionRun,

    /// `transitionstart`
    TransitionStart,

    /// `abort`
    Abort,

    /// `error`
    Error,

    /// `load`
    Load,

    /// `resize`
    Resize,

    /// `canplay`
    CanPlay,

    /// `canplaythrough`
    CanPlayThrough,

    /// `durationchange`
    DurationChange,

    /// `emptied`
    Emptied,

    /// `ended`
    Ended,

    /// `loadeddata`
    LoadedData,

    /// `loadedmetadata`
    LoadedMetaData,

    /// `loadstart`
    LoadStart,

    /// `pause`
    Pause,

    /// `play`
    Play,

    /// `playing`
    Playing,

    /// `progress`
    Progress,

    /// `ratechange`
    RateChange,

    /// `seeked`
    Seeked,

    /// `seeking`
    Seeking,

    /// `stalled`
    Stalled,

    /// `suspend`
    Suspend,

    /// `timeupdate`
    TimeUpdate,

    /// `volumechange`
    VolumeChange,

    /// `waiting`
    Waiting,

    /// `cancel`
    Cancel,

    /// `close`
    Close,

    /// `fullscreenchange`
    FullscreenChange,

    /// `fullscreenerror`
    FullscreenError,

    /// `selectionchange`
    SelectionChange,

    /// `slotchange`
    SlotChange,

    /// `toggle`
    Toggle,
}

impl Display for HtmlEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let event = match self {
            Self::AuxClick => "auxclick",
            Self::Click => "click",
            Self::ContextMenu => "contextmenu",
            Self::DblClick => "dblclick",
            Self::MouseDown => "mousedown",
            Self::MouseEnter => "mouseenter",
            Self::MouseLeave => "mouseleave",
            Self::MouseMove => "mousemove",
            Self::MouseOut => "mouseout",
            Self::MouseOver => "mouseover",
            Self::MouseUp => "mouseup",
            Self::GotPointerCapture => "gotpointercapture",
            Self::LostPointerCapture => "lostpointercapture",
            Self::PointerCancel => "pointercancel",
            Self::PointerDown => "pointerdown",
            Self::PointerEnter => "pointerenter",
            Self::PointerLeave => "pointerleave",
            Self::PointerMove => "pointermove",
            Self::PointerOut => "pointerout",
            Self::PointerOver => "pointerover",
            Self::PointerUp => "pointerup",
            Self::KeyDown => "keydown",
            Self::KeyUp => "keyup",
            Self::Blur => "blur",
            Self::Focus => "focus",
            Self::FocusIn => "focusin",
            Self::FocusOut => "focusout",
            Self::Change => "change",
            Self::Input => "input",
            Self::BeforeInput => "beforeinput",
            Self::Invalid => "invalid",
            Self::Reset => "reset",
            Self::Select => "select",
            Self::Submit => "submit",
            Self::Drag => "drag",
            Self::DragEnd => "dragend",
            Self::DragEnter => "dragenter",
            Self::DragLeave => "dragleave",
            Self::DragOver => "dragover",
            Self::DragStart => "dragstart",
            Self::Drop => "drop",
            Self::TouchCancel => "touchcancel",
            Self::TouchEnd => "touchend",
            Self::TouchMove => "touchmove",
            Self::TouchStart => "touchstart",
            Self::Scroll => "scroll",
            Self::ScrollEnd => "scrollend",
            Self::Wheel => "wheel",
            Self::Copy => "copy",
            Self::Cut => "cut",
            Self::Paste => "paste",
            Self::CompositionEnd => "compositionend",
            Self::CompositionStart => "compositionstart",
            Self::CompositionUpdate => "compositionupdate",
            Self::AnimationCancel => "animationcancel",
            Self::AnimationEnd => "animationend",
            Self::AnimationIteration => "animationiteration",
            Self::AnimationStart => "animationstart",
            Self::TransitionCancel => "transitioncancel",
            Self::TransitionEnd => "transitionend",
            Self::TransitionRun => "transitionrun",
            Self::TransitionStart => "transitionstart",
            Self::Abort => "abort",
            Self::Error => "error",
            Self::Load => "load",
            Self::Resize => "resize",
            Self::CanPlay => "canplay",
            Self::CanPlayThrough => "canplaythrough",
            Self::DurationChange => "durationchange",
            Self::Emptied => "emptied",
            Self::Ended => "ended",
            Self::LoadedData => "loadeddata",
            Self::LoadedMetaData => "loadedmetadata",
            Self::LoadStart => "loadstart",
            Self::Pause => "pause",
            Self::Play => "play",
            Self::Playing => "playing",
            Self::Progress => "progress",
            Self::RateChange => "ratechange",
            Self::Seeked => "seeked",
            Self::Seeking => "seeking",
            Self::Stalled => "stalled",
            Self::Suspend => "suspend",
            Self::TimeUpdate => "timeupdate",
            Self::VolumeChange => "volumechange",
            Self::Waiting => "waiting",
            Self::Cancel => "cancel",
            Self::Close => "close",
            Self::FullscreenChange => "fullscreenchange",
            Self::FullscreenError => "fullscreenerror",
            Self::SelectionChange => "selectionchange",
            Self::SlotChange => "slotchange",
            Self::Toggle => "toggle",
        };

        f.write_str(event)
    }
}

/// Typed CSS property names used by future `AttrMap` style storage.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CssProperty {
    /// CSS custom property names, rendered with a leading `--`.
    Custom(&'static str),

    /// `box-sizing`
    BoxSizing,

    /// `width`
    Width,

    /// `min-width`
    MinWidth,

    /// `max-width`
    MaxWidth,

    /// `height`
    Height,

    /// `min-height`
    MinHeight,

    /// `max-height`
    MaxHeight,

    /// `margin`
    Margin,

    /// `margin-top`
    MarginTop,

    /// `margin-right`
    MarginRight,

    /// `margin-bottom`
    MarginBottom,

    /// `margin-left`
    MarginLeft,

    /// `padding`
    Padding,

    /// `padding-top`
    PaddingTop,

    /// `padding-right`
    PaddingRight,

    /// `padding-bottom`
    PaddingBottom,

    /// `padding-left`
    PaddingLeft,

    /// `border`
    Border,

    /// `border-width`
    BorderWidth,

    /// `border-style`
    BorderStyle,

    /// `border-color`
    BorderColor,

    /// `border-radius`
    BorderRadius,

    /// `border-collapse`
    BorderCollapse,

    /// `border-spacing`
    BorderSpacing,

    /// `inline-size`
    InlineSize,

    /// `block-size`
    BlockSize,

    /// `min-inline-size`
    MinInlineSize,

    /// `max-inline-size`
    MaxInlineSize,

    /// `min-block-size`
    MinBlockSize,

    /// `max-block-size`
    MaxBlockSize,

    /// `margin-inline`
    MarginInline,

    /// `margin-inline-start`
    MarginInlineStart,

    /// `margin-inline-end`
    MarginInlineEnd,

    /// `margin-block`
    MarginBlock,

    /// `margin-block-start`
    MarginBlockStart,

    /// `margin-block-end`
    MarginBlockEnd,

    /// `padding-inline`
    PaddingInline,

    /// `padding-inline-start`
    PaddingInlineStart,

    /// `padding-inline-end`
    PaddingInlineEnd,

    /// `padding-block`
    PaddingBlock,

    /// `padding-block-start`
    PaddingBlockStart,

    /// `padding-block-end`
    PaddingBlockEnd,

    /// `inset-inline-start`
    InsetInlineStart,

    /// `inset-inline-end`
    InsetInlineEnd,

    /// `inset-block-start`
    InsetBlockStart,

    /// `inset-block-end`
    InsetBlockEnd,

    /// `position`
    Position,

    /// `top`
    Top,

    /// `right`
    Right,

    /// `bottom`
    Bottom,

    /// `left`
    Left,

    /// `z-index`
    ZIndex,

    /// `float`
    Float,

    /// `clear`
    Clear,

    /// `display`
    Display,

    /// `flex-direction`
    FlexDirection,

    /// `flex-wrap`
    FlexWrap,

    /// `flex-flow`
    FlexFlow,

    /// `flex-grow`
    FlexGrow,

    /// `flex-shrink`
    FlexShrink,

    /// `flex-basis`
    FlexBasis,

    /// `order`
    Order,

    /// `align-items`
    AlignItems,

    /// `align-self`
    AlignSelf,

    /// `align-content`
    AlignContent,

    /// `justify-content`
    JustifyContent,

    /// `justify-items`
    JustifyItems,

    /// `justify-self`
    JustifySelf,

    /// `place-items`
    PlaceItems,

    /// `place-content`
    PlaceContent,

    /// `gap`
    Gap,

    /// `row-gap`
    RowGap,

    /// `column-gap`
    ColumnGap,

    /// `grid-template-columns`
    GridTemplateColumns,

    /// `grid-template-rows`
    GridTemplateRows,

    /// `grid-column`
    GridColumn,

    /// `grid-row`
    GridRow,

    /// `grid-area`
    GridArea,

    /// `grid-auto-flow`
    GridAutoFlow,

    /// `grid-auto-columns`
    GridAutoColumns,

    /// `grid-auto-rows`
    GridAutoRows,

    /// `color`
    Color,

    /// `font-family`
    FontFamily,

    /// `font-size`
    FontSize,

    /// `font-weight`
    FontWeight,

    /// `font-style`
    FontStyle,

    /// `line-height`
    LineHeight,

    /// `text-align`
    TextAlign,

    /// `text-decoration`
    TextDecoration,

    /// `text-transform`
    TextTransform,

    /// `text-overflow`
    TextOverflow,

    /// `text-indent`
    TextIndent,

    /// `text-shadow`
    TextShadow,

    /// `white-space`
    WhiteSpace,

    /// `word-break`
    WordBreak,

    /// `word-wrap`
    WordWrap,

    /// `overflow-wrap`
    OverflowWrap,

    /// `letter-spacing`
    LetterSpacing,

    /// `word-spacing`
    WordSpacing,

    /// `background`
    Background,

    /// `background-color`
    BackgroundColor,

    /// `background-image`
    BackgroundImage,

    /// `background-position`
    BackgroundPosition,

    /// `background-size`
    BackgroundSize,

    /// `background-repeat`
    BackgroundRepeat,

    /// `opacity`
    Opacity,

    /// `visibility`
    Visibility,

    /// `box-shadow`
    BoxShadow,

    /// `outline`
    Outline,

    /// `outline-width`
    OutlineWidth,

    /// `outline-style`
    OutlineStyle,

    /// `outline-color`
    OutlineColor,

    /// `outline-offset`
    OutlineOffset,

    /// `cursor`
    Cursor,

    /// `pointer-events`
    PointerEvents,

    /// `user-select`
    UserSelect,

    /// `overflow`
    Overflow,

    /// `overflow-x`
    OverflowX,

    /// `overflow-y`
    OverflowY,

    /// `clip`
    Clip,

    /// `clip-path`
    ClipPath,

    /// `scroll-behavior`
    ScrollBehavior,

    /// `scroll-snap-type`
    ScrollSnapType,

    /// `scroll-snap-align`
    ScrollSnapAlign,

    /// `overscroll-behavior`
    OverscrollBehavior,

    /// `transform`
    Transform,

    /// `transform-origin`
    TransformOrigin,

    /// `transition`
    Transition,

    /// `transition-property`
    TransitionProperty,

    /// `transition-duration`
    TransitionDuration,

    /// `transition-timing-function`
    TransitionTimingFunction,

    /// `transition-delay`
    TransitionDelay,

    /// `animation`
    Animation,

    /// `animation-name`
    AnimationName,

    /// `animation-duration`
    AnimationDuration,

    /// `animation-timing-function`
    AnimationTimingFunction,

    /// `animation-delay`
    AnimationDelay,

    /// `animation-iteration-count`
    AnimationIterationCount,

    /// `animation-direction`
    AnimationDirection,

    /// `animation-fill-mode`
    AnimationFillMode,

    /// `animation-play-state`
    AnimationPlayState,

    /// `aspect-ratio`
    AspectRatio,

    /// `object-fit`
    ObjectFit,

    /// `object-position`
    ObjectPosition,

    /// `contain`
    Contain,

    /// `content-visibility`
    ContentVisibility,

    /// `will-change`
    WillChange,

    /// `appearance`
    Appearance,

    /// `resize`
    Resize,

    /// `touch-action`
    TouchAction,

    /// `filter`
    Filter,

    /// `backdrop-filter`
    BackdropFilter,

    /// `content`
    Content,

    /// `list-style`
    ListStyle,

    /// `list-style-type`
    ListStyleType,

    /// `list-style-position`
    ListStylePosition,

    /// `table-layout`
    TableLayout,

    /// `vertical-align`
    VerticalAlign,
}

impl Display for CssProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let property = match self {
            Self::Custom(name) => return write!(f, "--{name}"),
            Self::BoxSizing => "box-sizing",
            Self::Width => "width",
            Self::MinWidth => "min-width",
            Self::MaxWidth => "max-width",
            Self::Height => "height",
            Self::MinHeight => "min-height",
            Self::MaxHeight => "max-height",
            Self::Margin => "margin",
            Self::MarginTop => "margin-top",
            Self::MarginRight => "margin-right",
            Self::MarginBottom => "margin-bottom",
            Self::MarginLeft => "margin-left",
            Self::Padding => "padding",
            Self::PaddingTop => "padding-top",
            Self::PaddingRight => "padding-right",
            Self::PaddingBottom => "padding-bottom",
            Self::PaddingLeft => "padding-left",
            Self::Border => "border",
            Self::BorderWidth => "border-width",
            Self::BorderStyle => "border-style",
            Self::BorderColor => "border-color",
            Self::BorderRadius => "border-radius",
            Self::BorderCollapse => "border-collapse",
            Self::BorderSpacing => "border-spacing",
            Self::InlineSize => "inline-size",
            Self::BlockSize => "block-size",
            Self::MinInlineSize => "min-inline-size",
            Self::MaxInlineSize => "max-inline-size",
            Self::MinBlockSize => "min-block-size",
            Self::MaxBlockSize => "max-block-size",
            Self::MarginInline => "margin-inline",
            Self::MarginInlineStart => "margin-inline-start",
            Self::MarginInlineEnd => "margin-inline-end",
            Self::MarginBlock => "margin-block",
            Self::MarginBlockStart => "margin-block-start",
            Self::MarginBlockEnd => "margin-block-end",
            Self::PaddingInline => "padding-inline",
            Self::PaddingInlineStart => "padding-inline-start",
            Self::PaddingInlineEnd => "padding-inline-end",
            Self::PaddingBlock => "padding-block",
            Self::PaddingBlockStart => "padding-block-start",
            Self::PaddingBlockEnd => "padding-block-end",
            Self::InsetInlineStart => "inset-inline-start",
            Self::InsetInlineEnd => "inset-inline-end",
            Self::InsetBlockStart => "inset-block-start",
            Self::InsetBlockEnd => "inset-block-end",
            Self::Position => "position",
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::ZIndex => "z-index",
            Self::Float => "float",
            Self::Clear => "clear",
            Self::Display => "display",
            Self::FlexDirection => "flex-direction",
            Self::FlexWrap => "flex-wrap",
            Self::FlexFlow => "flex-flow",
            Self::FlexGrow => "flex-grow",
            Self::FlexShrink => "flex-shrink",
            Self::FlexBasis => "flex-basis",
            Self::Order => "order",
            Self::AlignItems => "align-items",
            Self::AlignSelf => "align-self",
            Self::AlignContent => "align-content",
            Self::JustifyContent => "justify-content",
            Self::JustifyItems => "justify-items",
            Self::JustifySelf => "justify-self",
            Self::PlaceItems => "place-items",
            Self::PlaceContent => "place-content",
            Self::Gap => "gap",
            Self::RowGap => "row-gap",
            Self::ColumnGap => "column-gap",
            Self::GridTemplateColumns => "grid-template-columns",
            Self::GridTemplateRows => "grid-template-rows",
            Self::GridColumn => "grid-column",
            Self::GridRow => "grid-row",
            Self::GridArea => "grid-area",
            Self::GridAutoFlow => "grid-auto-flow",
            Self::GridAutoColumns => "grid-auto-columns",
            Self::GridAutoRows => "grid-auto-rows",
            Self::Color => "color",
            Self::FontFamily => "font-family",
            Self::FontSize => "font-size",
            Self::FontWeight => "font-weight",
            Self::FontStyle => "font-style",
            Self::LineHeight => "line-height",
            Self::TextAlign => "text-align",
            Self::TextDecoration => "text-decoration",
            Self::TextTransform => "text-transform",
            Self::TextOverflow => "text-overflow",
            Self::TextIndent => "text-indent",
            Self::TextShadow => "text-shadow",
            Self::WhiteSpace => "white-space",
            Self::WordBreak => "word-break",
            Self::WordWrap => "word-wrap",
            Self::OverflowWrap => "overflow-wrap",
            Self::LetterSpacing => "letter-spacing",
            Self::WordSpacing => "word-spacing",
            Self::Background => "background",
            Self::BackgroundColor => "background-color",
            Self::BackgroundImage => "background-image",
            Self::BackgroundPosition => "background-position",
            Self::BackgroundSize => "background-size",
            Self::BackgroundRepeat => "background-repeat",
            Self::Opacity => "opacity",
            Self::Visibility => "visibility",
            Self::BoxShadow => "box-shadow",
            Self::Outline => "outline",
            Self::OutlineWidth => "outline-width",
            Self::OutlineStyle => "outline-style",
            Self::OutlineColor => "outline-color",
            Self::OutlineOffset => "outline-offset",
            Self::Cursor => "cursor",
            Self::PointerEvents => "pointer-events",
            Self::UserSelect => "user-select",
            Self::Overflow => "overflow",
            Self::OverflowX => "overflow-x",
            Self::OverflowY => "overflow-y",
            Self::Clip => "clip",
            Self::ClipPath => "clip-path",
            Self::ScrollBehavior => "scroll-behavior",
            Self::ScrollSnapType => "scroll-snap-type",
            Self::ScrollSnapAlign => "scroll-snap-align",
            Self::OverscrollBehavior => "overscroll-behavior",
            Self::Transform => "transform",
            Self::TransformOrigin => "transform-origin",
            Self::Transition => "transition",
            Self::TransitionProperty => "transition-property",
            Self::TransitionDuration => "transition-duration",
            Self::TransitionTimingFunction => "transition-timing-function",
            Self::TransitionDelay => "transition-delay",
            Self::Animation => "animation",
            Self::AnimationName => "animation-name",
            Self::AnimationDuration => "animation-duration",
            Self::AnimationTimingFunction => "animation-timing-function",
            Self::AnimationDelay => "animation-delay",
            Self::AnimationIterationCount => "animation-iteration-count",
            Self::AnimationDirection => "animation-direction",
            Self::AnimationFillMode => "animation-fill-mode",
            Self::AnimationPlayState => "animation-play-state",
            Self::AspectRatio => "aspect-ratio",
            Self::ObjectFit => "object-fit",
            Self::ObjectPosition => "object-position",
            Self::Contain => "contain",
            Self::ContentVisibility => "content-visibility",
            Self::WillChange => "will-change",
            Self::Appearance => "appearance",
            Self::Resize => "resize",
            Self::TouchAction => "touch-action",
            Self::Filter => "filter",
            Self::BackdropFilter => "backdrop-filter",
            Self::Content => "content",
            Self::ListStyle => "list-style",
            Self::ListStyleType => "list-style-type",
            Self::ListStylePosition => "list-style-position",
            Self::TableLayout => "table-layout",
            Self::VerticalAlign => "vertical-align",
        };

        f.write_str(property)
    }
}

/// Stringly, boolean, or absent attribute values stored in an [`AttrMap`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AttrValue {
    /// String attribute value.
    String(String),

    /// Boolean attribute value.
    Bool(bool),

    /// Attribute should be removed.
    None,
}

impl AttrValue {
    /// Returns the string representation of this value, or `None` for absent values.
    #[must_use]
    pub const fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            Self::Bool(true) => Some("true"),
            Self::Bool(false) => Some("false"),
            Self::None => None,
        }
    }
}

/// HTML attributes whose values are space-separated token lists.
const SPACE_SEPARATED: &[HtmlAttr] = &[
    HtmlAttr::Class,
    HtmlAttr::Rel,
    HtmlAttr::Aria(AriaAttr::LabelledBy),
    HtmlAttr::Aria(AriaAttr::DescribedBy),
    HtmlAttr::Aria(AriaAttr::Owns),
    HtmlAttr::Aria(AriaAttr::Controls),
    HtmlAttr::Aria(AriaAttr::FlowTo),
    HtmlAttr::Aria(AriaAttr::Details),
];

/// Framework-agnostic attribute map containing only data and inline style values.
///
/// Event handlers are not stored in this map. Adapters wire typed handler methods
/// exposed by component APIs into their framework-native event systems.
///
/// This type intentionally stores attrs and styles in sorted `Vec`s instead of a
/// `HashMap` or `BTreeMap`. Component attr maps are expected to stay small
/// (typically only a handful to low dozens of entries), so the contiguous layout
/// and deterministic iteration order of a `Vec` are a better fit than hash-table
/// or tree-node overhead. Lookups still use `binary_search`, which keeps reads
/// efficient while avoiding extra allocation and pointer chasing.
///
/// When the `serde` feature is enabled, this type implements [`serde::Serialize`]
/// but intentionally does not implement `Deserialize`. `AttrMap` is a server-side
/// rendering output structure: adapters turn it into HTML attributes during SSR,
/// and hydration reads those attributes back from the DOM. The JSON round-trip
/// hydration path is for machine state snapshots, not for reconstructing an
/// `AttrMap` value on the client.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AttrMap {
    attrs: Vec<(HtmlAttr, AttrValue)>,
    styles: Vec<(CssProperty, String)>,
}

/// Destructured parts of an [`AttrMap`], for adapter-side conversion without cloning.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AttrMapParts {
    /// Sorted typed HTML attributes.
    pub attrs: Vec<(HtmlAttr, AttrValue)>,

    /// Sorted typed CSS properties.
    pub styles: Vec<(CssProperty, String)>,
}

impl AttrMap {
    /// Creates an empty attribute map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes this map into its raw typed attribute and style collections.
    #[must_use]
    pub fn into_parts(self) -> AttrMapParts {
        AttrMapParts {
            attrs: self.attrs,
            styles: self.styles,
        }
    }

    /// Returns the sorted attribute entries stored in this map.
    #[must_use]
    pub fn attrs(&self) -> &[(HtmlAttr, AttrValue)] {
        &self.attrs
    }

    /// Returns the sorted style entries stored in this map.
    #[must_use]
    pub fn styles(&self) -> &[(CssProperty, String)] {
        &self.styles
    }

    /// Sets an attribute on the map.
    ///
    /// For most attributes, later values replace earlier ones. Space-separated
    /// token-list attributes append new tokens with deduplication. Passing
    /// [`AttrValue::None`] removes the attribute.
    pub fn set(&mut self, attr: HtmlAttr, value: impl Into<AttrValue>) -> &mut Self {
        let value = value.into();

        let is_space_separated = SPACE_SEPARATED.contains(&attr);

        match self.attrs.binary_search_by(|(key, _)| key.cmp(&attr)) {
            Ok(index) => {
                if matches!(value, AttrValue::None) {
                    self.attrs.remove(index);
                } else if is_space_separated {
                    match (&mut self.attrs[index].1, value) {
                        (AttrValue::String(existing), AttrValue::String(new_value)) => {
                            append_space_separated(existing, &new_value);
                        }

                        (slot, replacement) => *slot = replacement,
                    }
                } else {
                    self.attrs[index].1 = value;
                }
            }
            Err(index) => {
                if !matches!(value, AttrValue::None) {
                    self.attrs.insert(index, (attr, value));
                }
            }
        }

        self
    }

    /// Sets a CSS property on the map, replacing any existing value for the property.
    pub fn set_style(&mut self, prop: CssProperty, value: impl Into<String>) -> &mut Self {
        let value = value.into();

        match self.styles.binary_search_by(|(key, _)| key.cmp(&prop)) {
            Ok(index) => self.styles[index].1 = value,
            Err(index) => self.styles.insert(index, (prop, value)),
        }

        self
    }

    /// Convenience method for setting a boolean-valued attribute.
    pub fn set_bool(&mut self, attr: HtmlAttr, value: bool) -> &mut Self {
        self.set(attr, AttrValue::Bool(value))
    }

    /// Returns `true` when the given attribute key is present.
    #[must_use]
    pub fn contains(&self, attr: &HtmlAttr) -> bool {
        self.attrs
            .binary_search_by(|(key, _)| key.cmp(attr))
            .is_ok()
    }

    /// Returns the string representation of the given attribute if present.
    #[must_use]
    pub fn get(&self, attr: &HtmlAttr) -> Option<&str> {
        self.get_value(attr).and_then(AttrValue::as_str)
    }

    /// Returns the raw typed value of the given attribute if present.
    #[must_use]
    pub fn get_value(&self, attr: &HtmlAttr) -> Option<&AttrValue> {
        self.attrs
            .binary_search_by(|(key, _)| key.cmp(attr))
            .ok()
            .map(|index| &self.attrs[index].1)
    }

    /// Iterates over the stored attribute entries.
    pub fn iter_attrs(&self) -> impl Iterator<Item = &(HtmlAttr, AttrValue)> {
        self.attrs.iter()
    }

    /// Iterates over the stored attribute keys and values as separate references.
    pub fn iter(&self) -> impl Iterator<Item = (&HtmlAttr, &AttrValue)> {
        self.attrs.iter().map(|(key, value)| (key, value))
    }

    /// Iterates over the stored attribute keys.
    pub fn keys(&self) -> impl Iterator<Item = &HtmlAttr> {
        self.attrs.iter().map(|(key, _)| key)
    }

    /// Iterates over the stored style entries.
    pub fn iter_styles(&self) -> impl Iterator<Item = &(CssProperty, String)> {
        self.styles.iter()
    }

    /// Merges a trusted attribute map into this one.
    ///
    /// Attribute precedence is last-write-wins except for space-separated token
    /// list attributes, which append new tokens with deduplication.
    pub fn merge(&mut self, other: AttrMap) {
        for (attr, value) in other.attrs {
            self.set(attr, value);
        }
        for (prop, value) in other.styles {
            self.set_style(prop, value);
        }
    }

    /// Merges user-provided attribute extensions into this map.
    pub fn merge_user(&mut self, user: UserAttrs) {
        self.merge(user.0);
    }
}

/// User-provided attribute extensions with a structural blocklist enforced at construction time.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UserAttrs(AttrMap);

/// Attributes that users cannot override via [`UserAttrs`].
const USER_BLOCKED: &[HtmlAttr] = &[
    HtmlAttr::Id,
    HtmlAttr::Role,
    HtmlAttr::Aria(AriaAttr::Hidden),
    HtmlAttr::Aria(AriaAttr::Modal),
    HtmlAttr::TabIndex,
    HtmlAttr::Aria(AriaAttr::Live),
];

impl UserAttrs {
    /// Creates an empty user-attribute container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a user-provided attribute unless the key is blocked.
    pub fn set(&mut self, attr: HtmlAttr, value: impl Into<AttrValue>) -> &mut Self {
        if USER_BLOCKED.contains(&attr) {
            return self;
        }

        self.0.set(attr, value);

        self
    }

    /// Sets a user-provided style value.
    pub fn set_style(&mut self, prop: CssProperty, value: impl Into<String>) -> &mut Self {
        self.0.set_style(prop, value);
        self
    }

    /// Sets a user-provided boolean attribute unless the key is blocked.
    pub fn set_bool(&mut self, attr: HtmlAttr, value: bool) -> &mut Self {
        if USER_BLOCKED.contains(&attr) {
            return self;
        }

        self.0.set_bool(attr, value);

        self
    }
}

/// Controls how dynamic styles from [`AttrMap::styles`] are rendered to the DOM.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum StyleStrategy {
    /// Render styles as inline `style` attributes.
    #[default]
    Inline,

    /// Apply styles at runtime via the CSSOM API.
    Cssom,

    /// Emit nonce-backed scoped CSS rules collected into a `<style>` block.
    Nonce(String),
}

fn append_space_separated(existing: &mut String, new_value: &str) {
    for token in new_value.split_whitespace() {
        if existing.split_whitespace().any(|current| current == token) {
            continue;
        }

        if !existing.is_empty() {
            existing.push(' ');
        }

        existing.push_str(token);
    }
}

/// Event listener configuration used when adapters bind typed handlers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EventOptions {
    /// Whether the listener is passive.
    pub passive: bool,

    /// Whether the listener captures during the capture phase.
    pub capture: bool,
}

/// Convenience constructor for `data-*` attributes.
#[must_use]
pub const fn data(name: &'static str) -> HtmlAttr {
    HtmlAttr::Data(name)
}

impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        Self::String(String::from(value))
    }
}

impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&String> for AttrValue {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for AriaAttr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for HtmlAttr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for HtmlEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CssProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use super::*;

    #[test]
    fn aria_attr_display_matches_attribute_name() {
        assert_eq!(AriaAttr::Label.to_string(), "aria-label");
        assert_eq!(AriaAttr::DescribedBy.as_str(), "aria-describedby");
    }

    #[test]
    fn html_attr_display_formats_data_and_static_names() {
        assert_eq!(HtmlAttr::Data("ars-state").to_string(), "data-ars-state");
        assert_eq!(HtmlAttr::Id.to_string(), "id");
        assert_eq!(
            HtmlAttr::Aria(AriaAttr::Expanded).to_string(),
            "aria-expanded"
        );
    }

    #[test]
    fn html_attr_static_name_is_none_for_data_attributes() {
        assert_eq!(HtmlAttr::Data("ars-scope").static_name(), None);
        assert_eq!(HtmlAttr::Role.static_name(), Some("role"));
        assert_eq!(
            HtmlAttr::Aria(AriaAttr::Controls).static_name(),
            Some("aria-controls")
        );
    }

    #[test]
    fn html_event_display_matches_dom_event_names() {
        assert_eq!(HtmlEvent::PointerDown.to_string(), "pointerdown");
        assert_eq!(HtmlEvent::BeforeInput.to_string(), "beforeinput");
        assert_eq!(HtmlEvent::TransitionEnd.to_string(), "transitionend");
    }

    #[test]
    fn css_property_display_matches_css_spelling() {
        assert_eq!(CssProperty::ZIndex.to_string(), "z-index");
        assert_eq!(
            CssProperty::ScrollSnapAlign.to_string(),
            "scroll-snap-align"
        );
        assert_eq!(
            CssProperty::Custom("ars-timer-progress").to_string(),
            "--ars-timer-progress"
        );
    }

    #[test]
    fn event_options_default_to_non_passive_bubbling_listener() {
        let options = EventOptions::default();

        assert!(!options.passive);
        assert!(!options.capture);
    }

    #[test]
    fn data_helper_constructs_data_variant() {
        assert_eq!(data("ars-part"), HtmlAttr::Data("ars-part"));
    }

    #[test]
    fn attr_map_set_and_get_store_typed_values() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, "dialog-root");
        attrs.set_bool(HtmlAttr::Hidden, true);

        assert!(attrs.contains(&HtmlAttr::Id));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("dialog-root"));
        assert_eq!(attrs.get(&HtmlAttr::Hidden), Some("true"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Hidden),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn attr_map_set_none_removes_existing_value() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Title, "before");
        attrs.set(HtmlAttr::Title, AttrValue::None);

        assert!(!attrs.contains(&HtmlAttr::Title));
        assert_eq!(attrs.get(&HtmlAttr::Title), None);
    }

    #[test]
    fn attr_map_space_separated_values_append_with_dedup() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        attrs.set(HtmlAttr::Class, "ars-touch-none");
        attrs.set(HtmlAttr::Class, "ars-touch-none");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "label-a label-b");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "label-b label-c");
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "hint");
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "hint error");

        assert_eq!(
            attrs.get(&HtmlAttr::Class),
            Some("ars-visually-hidden ars-touch-none")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("label-a label-b label-c")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("hint error")
        );
    }

    #[test]
    fn attr_map_styles_replace_by_property() {
        let mut attrs = AttrMap::new();

        attrs.set_style(CssProperty::Width, "10px");
        attrs.set_style(CssProperty::Width, "12px");
        attrs.set_style(CssProperty::Height, "20px");

        assert_eq!(
            attrs.styles(),
            &[
                (CssProperty::Width, String::from("12px")),
                (CssProperty::Height, String::from("20px")),
            ]
        );
    }

    #[test]
    fn attr_map_merge_uses_typed_semantics() {
        let mut base = AttrMap::new();

        base.set(HtmlAttr::Role, "button");
        base.set(HtmlAttr::Class, "base");
        base.set_style(CssProperty::Width, "10px");

        let mut overlay = AttrMap::new();

        overlay.set(HtmlAttr::Role, "switch");
        overlay.set(HtmlAttr::Class, "overlay");
        overlay.set_style(CssProperty::Width, "20px");

        base.merge(overlay);

        assert_eq!(base.get(&HtmlAttr::Role), Some("switch"));
        assert_eq!(base.get(&HtmlAttr::Class), Some("base overlay"));
        assert_eq!(base.styles(), &[(CssProperty::Width, String::from("20px"))]);
    }

    #[test]
    fn user_attrs_reject_blocked_keys() {
        let mut user = UserAttrs::new();

        user.set(HtmlAttr::Id, "user-id");
        user.set(HtmlAttr::Role, "button");
        user.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        user.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        user.set_bool(HtmlAttr::TabIndex, true);
        user.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        user.set(HtmlAttr::Title, "allowed");
        user.set_style(CssProperty::Width, "12px");

        let mut merged = AttrMap::new();

        merged.merge_user(user);

        assert!(!merged.contains(&HtmlAttr::Id));
        assert!(!merged.contains(&HtmlAttr::Role));
        assert!(!merged.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
        assert!(!merged.contains(&HtmlAttr::Aria(AriaAttr::Modal)));
        assert!(!merged.contains(&HtmlAttr::TabIndex));
        assert!(!merged.contains(&HtmlAttr::Aria(AriaAttr::Live)));
        assert_eq!(merged.get(&HtmlAttr::Title), Some("allowed"));
        assert_eq!(
            merged.styles(),
            &[(CssProperty::Width, String::from("12px"))]
        );
    }

    #[test]
    fn attr_map_into_parts_exposes_raw_sorted_vectors() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, "root");
        attrs.set(HtmlAttr::Class, "alpha");
        attrs.set_style(CssProperty::Width, "10px");

        let parts = attrs.into_parts();

        assert_eq!(
            parts.attrs,
            vec![
                (HtmlAttr::Class, AttrValue::String(String::from("alpha"))),
                (HtmlAttr::Id, AttrValue::String(String::from("root"))),
            ]
        );
        assert_eq!(
            parts.styles,
            vec![(CssProperty::Width, String::from("10px"))]
        );
    }

    #[test]
    fn attr_map_iterators_expose_current_entries() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, "root");
        attrs.set(HtmlAttr::Title, "tooltip");

        let keys = attrs.keys().copied().collect::<Vec<_>>();

        let iter_pairs = attrs
            .iter()
            .map(|(key, value)| (*key, value.clone()))
            .collect::<Vec<_>>();

        assert_eq!(keys, vec![HtmlAttr::Id, HtmlAttr::Title]);
        assert_eq!(
            iter_pairs,
            vec![
                (HtmlAttr::Id, AttrValue::String(String::from("root"))),
                (HtmlAttr::Title, AttrValue::String(String::from("tooltip"))),
            ]
        );
        assert_eq!(attrs.iter_attrs().count(), 2);
        assert_eq!(attrs.iter_styles().count(), 0);
    }

    #[test]
    fn style_strategy_defaults_to_inline_and_supports_representative_variants() {
        assert_eq!(StyleStrategy::default(), StyleStrategy::Inline);
        assert_eq!(StyleStrategy::Cssom, StyleStrategy::Cssom);
        assert_eq!(
            StyleStrategy::Nonce(String::from("nonce-123")),
            StyleStrategy::Nonce(String::from("nonce-123"))
        );
    }

    #[test]
    fn attr_value_as_str_covers_all_variants() {
        assert_eq!(AttrValue::from("hello").as_str(), Some("hello"));
        assert_eq!(AttrValue::from(true).as_str(), Some("true"));
        assert_eq!(AttrValue::from(false).as_str(), Some("false"));
        assert_eq!(AttrValue::None.as_str(), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn attr_map_serializes_for_ssr() {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, "dialog-root");
        attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        attrs.set_style(CssProperty::Width, "1px");

        let json = serde_json::to_string(&attrs).expect("AttrMap must serialize");

        assert!(json.contains("dialog-root"));
        assert!(json.contains("ars-visually-hidden"));
        assert!(json.contains("width"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn style_strategy_serializes_nonce_variant() {
        let json = serde_json::to_string(&StyleStrategy::Nonce(String::from("nonce-123")))
            .expect("StyleStrategy must serialize");

        assert!(json.contains("nonce-123"));
    }
}
