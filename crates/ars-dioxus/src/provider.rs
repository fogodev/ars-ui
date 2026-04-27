//! Reactive `ArsProvider` context helpers for the Dioxus adapter.

use std::{
    any::Any,
    fmt::{self, Debug},
    pin::Pin,
    sync::Arc,
};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects,
    PlatformEffects, Rect, StyleStrategy, resolve_messages as core_resolve_messages,
};
use ars_forms::field::FileRef;
use ars_i18n::{Direction, IntlBackend, Locale, StubIntlBackend, Translate, locales, number};
use dioxus::prelude::*;

/// Options for opening a platform file picker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePickerOptions;

/// Adapter-local drag payload wrapper.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DragData;

/// Dioxus-specific platform services not covered by core [`PlatformEffects`].
pub trait DioxusPlatform: Send + Sync + 'static {
    /// Focus an element by its ID.
    fn focus_element(&self, id: &str);

    /// Returns the current bounding rect for an element, if available.
    fn get_bounding_rect(&self, id: &str) -> Option<Rect>;

    /// Scrolls an element into view.
    fn scroll_into_view(&self, id: &str);

    /// Writes text to the clipboard.
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>>;

    /// Opens a native file picker.
    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>>;

    /// Returns the current timestamp in milliseconds.
    fn now_ms(&self) -> f64;

    /// Generates a platform-scoped unique ID.
    fn new_id(&self) -> String;

    /// Creates drag data from a platform event, if supported.
    fn create_drag_data(&self, event: &dyn Any) -> Option<DragData>;
}

/// Web platform implementation placeholder.
#[cfg(feature = "web")]
#[derive(Clone, Copy, Debug, Default)]
pub struct WebPlatform;

/// Desktop platform implementation placeholder.
#[cfg(feature = "desktop")]
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopPlatform;

/// No-op platform for tests and non-interactive environments.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullPlatform;

#[cfg(feature = "web")]
impl DioxusPlatform for WebPlatform {
    fn focus_element(&self, id: &str) {
        NullPlatform.focus_element(id);
    }

    fn get_bounding_rect(&self, id: &str) -> Option<Rect> {
        NullPlatform.get_bounding_rect(id)
    }

    fn scroll_into_view(&self, id: &str) {
        NullPlatform.scroll_into_view(id);
    }

    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        NullPlatform.set_clipboard(text)
    }

    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        NullPlatform.open_file_picker(options)
    }

    fn now_ms(&self) -> f64 {
        NullPlatform.now_ms()
    }

    fn new_id(&self) -> String {
        NullPlatform.new_id()
    }

    fn create_drag_data(&self, event: &dyn Any) -> Option<DragData> {
        NullPlatform.create_drag_data(event)
    }
}

#[cfg(feature = "desktop")]
impl DioxusPlatform for DesktopPlatform {
    fn focus_element(&self, id: &str) {
        NullPlatform.focus_element(id);
    }

    fn get_bounding_rect(&self, id: &str) -> Option<Rect> {
        NullPlatform.get_bounding_rect(id)
    }

    fn scroll_into_view(&self, id: &str) {
        NullPlatform.scroll_into_view(id);
    }

    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        NullPlatform.set_clipboard(text)
    }

    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        NullPlatform.open_file_picker(options)
    }

    fn now_ms(&self) -> f64 {
        NullPlatform.now_ms()
    }

    fn new_id(&self) -> String {
        NullPlatform.new_id()
    }

    fn create_drag_data(&self, event: &dyn Any) -> Option<DragData> {
        NullPlatform.create_drag_data(event)
    }
}

impl DioxusPlatform for NullPlatform {
    fn focus_element(&self, _id: &str) {}

    fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
        None
    }

    fn scroll_into_view(&self, _id: &str) {}

    fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn open_file_picker(
        &self,
        _options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        Box::pin(async { Vec::new() })
    }

    fn now_ms(&self) -> f64 {
        0.0
    }

    fn new_id(&self) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        format!("null-id-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn create_drag_data(&self, _event: &dyn Any) -> Option<DragData> {
        None
    }
}

fn default_dioxus_platform() -> Arc<dyn DioxusPlatform> {
    #[cfg(feature = "web")]
    {
        Arc::new(WebPlatform)
    }

    #[cfg(all(feature = "desktop", not(feature = "web")))]
    {
        Arc::new(DesktopPlatform)
    }

    #[cfg(not(any(feature = "web", feature = "desktop")))]
    {
        Arc::new(NullPlatform)
    }
}

fn direction_from_locale(locale: &Locale) -> Direction {
    if locale.direction().is_rtl() {
        Direction::Rtl
    } else {
        Direction::Ltr
    }
}

/// Props for the Dioxus [`ArsProvider`] component.
#[derive(Props, Clone)]
pub struct ArsProviderProps {
    /// The active locale for this provider subtree.
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,

    /// The explicit reading direction override for this provider subtree.
    #[props(optional, into)]
    pub direction: Option<Signal<Direction>>,

    /// The active color mode for descendant components.
    #[props(optional, into)]
    pub color_mode: Option<Signal<ColorMode>>,

    /// Whether descendant interactive components should render as disabled.
    #[props(optional, into)]
    pub disabled: Option<Signal<bool>>,

    /// Whether descendant form fields should render as read-only.
    #[props(optional, into)]
    pub read_only: Option<Signal<bool>>,

    /// Optional generated-ID prefix for descendants.
    #[props(optional)]
    pub id_prefix: Option<String>,

    /// Optional portal container element ID.
    #[props(optional)]
    pub portal_container_id: Option<String>,

    /// Optional focus/portal root node ID.
    #[props(optional)]
    pub root_node_id: Option<String>,

    /// Platform side-effect capabilities for descendants.
    #[props(optional)]
    pub platform: Option<Arc<dyn PlatformEffects>>,

    /// ICU-backed locale data provider.
    #[props(optional)]
    pub intl_backend: Option<Arc<dyn IntlBackend>>,

    /// Application-owned translation registries.
    #[props(optional)]
    pub i18n_registries: Option<Arc<I18nRegistries>>,

    /// CSS style injection strategy.
    #[props(optional)]
    pub style_strategy: Option<StyleStrategy>,

    /// Dioxus-specific platform services for descendants.
    #[props(optional)]
    pub dioxus_platform: Option<Arc<dyn DioxusPlatform>>,

    /// Descendant content wrapped by the provider boundary.
    pub children: Element,
}

impl Debug for ArsProviderProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArsProviderProps")
            .field("locale", &self.locale)
            .field("direction", &self.direction)
            .field("color_mode", &self.color_mode)
            .field("disabled", &self.disabled)
            .field("read_only", &self.read_only)
            .field("id_prefix", &self.id_prefix)
            .field("portal_container_id", &self.portal_container_id)
            .field("root_node_id", &self.root_node_id)
            .field("platform", &self.platform.as_ref().map(|_| "Arc(..)"))
            .field(
                "intl_backend",
                &self.intl_backend.as_ref().map(|_| "Arc(..)"),
            )
            .field(
                "i18n_registries",
                &self.i18n_registries.as_ref().map(|_| "Arc(..)"),
            )
            .field("style_strategy", &self.style_strategy)
            .field(
                "dioxus_platform",
                &self.dioxus_platform.as_ref().map(|_| "Arc(..)"),
            )
            .field("children", &"<Element>")
            .finish()
    }
}

impl PartialEq for ArsProviderProps {
    fn eq(&self, other: &Self) -> bool {
        self.locale == other.locale
            && self.direction == other.direction
            && self.color_mode == other.color_mode
            && self.disabled == other.disabled
            && self.read_only == other.read_only
            && self.id_prefix == other.id_prefix
            && self.portal_container_id == other.portal_container_id
            && self.root_node_id == other.root_node_id
            && self.style_strategy == other.style_strategy
            && arc_option_ptr_eq(self.platform.as_ref(), other.platform.as_ref())
            && arc_option_ptr_eq(self.intl_backend.as_ref(), other.intl_backend.as_ref())
            && arc_option_ptr_eq(
                self.i18n_registries.as_ref(),
                other.i18n_registries.as_ref(),
            )
            && arc_option_ptr_eq(
                self.dioxus_platform.as_ref(),
                other.dioxus_platform.as_ref(),
            )
            && self.children == other.children
    }
}

fn arc_option_ptr_eq<T: ?Sized>(left: Option<&Arc<T>>, right: Option<&Arc<T>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

/// Publishes adapter environment context to a Dioxus subtree.
#[component]
pub fn ArsProvider(props: ArsProviderProps) -> Element {
    let ArsProviderProps {
        locale,
        direction,
        color_mode,
        disabled,
        read_only,
        id_prefix,
        portal_container_id,
        root_node_id,
        platform,
        intl_backend,
        i18n_registries,
        style_strategy,
        dioxus_platform,
        children,
    } = props;
    let fallback_locale = use_signal(locales::en_us);
    let locale = locale.unwrap_or(fallback_locale);

    let explicit_direction = direction;

    let direction = use_memo(move || {
        explicit_direction.map_or_else(
            || direction_from_locale(&locale.read()),
            |signal| *signal.read(),
        )
    });

    let fallback_color_mode = use_signal(|| ColorMode::System);
    let color_mode = color_mode.unwrap_or(fallback_color_mode);

    let fallback_disabled = use_signal(|| false);
    let disabled = disabled.unwrap_or(fallback_disabled);

    let fallback_read_only = use_signal(|| false);
    let read_only = read_only.unwrap_or(fallback_read_only);

    let direction_for_context = direction;

    let direction_for_render = direction;

    use_context_provider(move || ArsContext {
        locale,
        direction: direction_for_context,
        color_mode,
        disabled,
        read_only,
        id_prefix: Signal::new(id_prefix),
        portal_container_id: Signal::new(portal_container_id),
        root_node_id: Signal::new(root_node_id),
        platform: platform.unwrap_or_else(|| Arc::new(NullPlatformEffects)),
        modality: Arc::new(DefaultModalityContext::new()),
        intl_backend: intl_backend.unwrap_or_else(|| Arc::new(StubIntlBackend)),
        i18n_registries: i18n_registries.unwrap_or_else(|| Arc::new(I18nRegistries::new())),
        dioxus_platform: dioxus_platform.unwrap_or_else(default_dioxus_platform),
        style_strategy: style_strategy.unwrap_or(StyleStrategy::Inline),
    });

    let dir = direction_for_render.read().as_html_attr();

    rsx! {
        div { dir, {children} }
    }
}

/// Reactive environment context published by the Dioxus `ArsProvider`.
#[derive(Clone)]
pub struct ArsContext {
    /// Active locale for this subtree.
    pub locale: Signal<Locale>,

    /// Resolved reading direction for this subtree.
    pub direction: Memo<Direction>,

    /// Active color mode for descendants.
    pub color_mode: Signal<ColorMode>,

    /// Whether interactive descendants should render as disabled.
    pub disabled: Signal<bool>,

    /// Whether descendant form fields should render as read-only.
    pub read_only: Signal<bool>,

    /// Optional generated-ID prefix.
    pub id_prefix: Signal<Option<String>>,

    /// Optional portal container element ID.
    pub portal_container_id: Signal<Option<String>>,

    /// Optional focus/portal root node ID.
    pub root_node_id: Signal<Option<String>>,

    /// Platform side-effect capabilities.
    pub platform: Arc<dyn PlatformEffects>,

    /// Shared input-modality state for this provider root.
    pub modality: Arc<dyn ModalityContext>,

    /// ICU-backed locale data provider.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// Application-owned message registries.
    pub i18n_registries: Arc<I18nRegistries>,

    /// Dioxus-specific platform services.
    pub dioxus_platform: Arc<dyn DioxusPlatform>,

    /// CSS style injection strategy for all descendant ars components.
    style_strategy: StyleStrategy,
}

impl Debug for ArsContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArsContext")
            .field("locale", &self.locale)
            .field("direction", &self.direction)
            .field("color_mode", &self.color_mode)
            .field("disabled", &self.disabled)
            .field("read_only", &self.read_only)
            .field("id_prefix", &self.id_prefix)
            .field("portal_container_id", &self.portal_container_id)
            .field("root_node_id", &self.root_node_id)
            .field("platform", &"Arc(..)")
            .field("modality", &"Arc(..)")
            .field("intl_backend", &"Arc(..)")
            .field("i18n_registries", &"Arc(..)")
            .field("dioxus_platform", &"Arc(..)")
            .field("style_strategy", &self.style_strategy)
            .finish()
    }
}

impl ArsContext {
    /// Creates a reactive provider context from fixed initial values.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "ArsContext intentionally mirrors the provider surface."
    )]
    pub fn new(
        locale: Locale,
        direction: Direction,
        color_mode: ColorMode,
        disabled: bool,
        read_only: bool,
        id_prefix: Option<String>,
        portal_container_id: Option<String>,
        root_node_id: Option<String>,
        platform: Arc<dyn PlatformEffects>,
        modality: Arc<dyn ModalityContext>,
        intl_backend: Arc<dyn IntlBackend>,
        i18n_registries: Arc<I18nRegistries>,
        dioxus_platform: Arc<dyn DioxusPlatform>,
        style_strategy: StyleStrategy,
    ) -> Self {
        Self {
            locale: Signal::new(locale),
            direction: Memo::new(move || direction),
            color_mode: Signal::new(color_mode),
            disabled: Signal::new(disabled),
            read_only: Signal::new(read_only),
            id_prefix: Signal::new(id_prefix),
            portal_container_id: Signal::new(portal_container_id),
            root_node_id: Signal::new(root_node_id),
            platform,
            modality,
            intl_backend,
            i18n_registries,
            dioxus_platform,
            style_strategy,
        }
    }

    /// Returns the configured style strategy.
    #[must_use]
    pub const fn style_strategy(&self) -> &StyleStrategy {
        &self.style_strategy
    }
}

/// Emits a debug warning when a provider-dependent helper is used without context.
#[cfg(feature = "debug")]
pub fn warn_missing_provider(hook: &str) {
    log::warn!(
        "[ars-ui] {hook}() called without ArsProvider. Returning default value. Wrap your app root in <ArsProvider>."
    );
}

/// No-op outside debug builds.
#[cfg(not(feature = "debug"))]
pub const fn warn_missing_provider(_hook: &str) {}

/// Returns the current locale signal from provider context.
#[must_use]
pub fn use_locale() -> Signal<Locale> {
    let fallback = use_signal(locales::en_us);

    try_use_context::<ArsContext>().map_or_else(
        || {
            warn_missing_provider("use_locale");

            fallback
        },
        |ctx| ctx.locale,
    )
}

/// Resolves the effective locale for an adapter component instance.
///
/// Returns `adapter_props_locale.cloned()` when set, otherwise reads the
/// surrounding [`ArsProvider`](ArsContext) locale via [`use_locale`] and
/// clones it. Subscribes the calling component to locale changes (via
/// `Signal::read`), so locale-dependent output stays reactive.
///
/// This mirrors the Leptos
/// [`resolve_locale`](ars_leptos::resolve_locale) utility — adapter
/// component authors should reach for this helper instead of
/// hand-rolling the `Option` + provider fallback chain.
#[must_use]
pub fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| use_locale().read().clone())
}

/// Resolves a memoized number formatter from the current provider locale.
#[must_use]
pub fn use_number_formatter<F>(options: F) -> Memo<number::Formatter>
where
    F: Fn() -> number::FormatOptions + 'static,
{
    use_resolved_number_formatter(None, options)
}

/// Resolves a memoized number formatter from an explicit locale or provider context.
#[must_use]
pub(crate) fn use_resolved_number_formatter<F>(
    adapter_props_locale: Option<&Locale>,
    options: F,
) -> Memo<number::Formatter>
where
    F: Fn() -> number::FormatOptions + 'static,
{
    let explicit_locale = adapter_props_locale.cloned();

    let locale = use_locale();

    let resolved_options = options();

    use_memo(use_reactive!(|explicit_locale, resolved_options| {
        let resolved_locale = explicit_locale
            .clone()
            .unwrap_or_else(|| locale.read().clone());

        number::Formatter::new(&resolved_locale, resolved_options.clone())
    }))
}

/// Resolves the current ICU provider from provider context.
#[must_use]
pub fn use_intl_backend() -> Arc<dyn IntlBackend> {
    try_use_context::<ArsContext>().map_or_else(
        || -> Arc<dyn IntlBackend> {
            warn_missing_provider("use_intl_backend");

            Arc::new(StubIntlBackend)
        },
        |ctx| -> Arc<dyn IntlBackend> { Arc::clone(&ctx.intl_backend) },
    )
}

/// Resolves the shared input-modality context from provider context.
#[must_use]
pub fn use_modality_context() -> Arc<dyn ModalityContext> {
    try_use_context::<ArsContext>().map_or_else(
        || -> Arc<dyn ModalityContext> {
            warn_missing_provider("use_modality_context");

            Arc::new(DefaultModalityContext::new())
        },
        |ctx| -> Arc<dyn ModalityContext> { Arc::clone(&ctx.modality) },
    )
}

/// Resolves per-component messages from override, provider registry, or defaults.
#[must_use]
pub fn use_messages<M: ars_core::ComponentMessages + Send + Sync + 'static>(
    adapter_props_messages: Option<&M>,
    adapter_props_locale: Option<&Locale>,
) -> M {
    let locale = resolve_locale(adapter_props_locale);

    let registries = try_use_context::<ArsContext>().map_or_else(
        || Arc::new(I18nRegistries::new()),
        |ctx| Arc::clone(&ctx.i18n_registries),
    );

    core_resolve_messages(adapter_props_messages, registries.as_ref(), &locale)
}

/// Resolves the Dioxus-specific platform handle from provider context.
#[must_use]
pub fn use_platform() -> Arc<dyn DioxusPlatform> {
    try_use_context::<ArsContext>().map_or_else(
        || {
            warn_missing_provider("use_platform");

            default_dioxus_platform()
        },
        |ctx| Arc::clone(&ctx.dioxus_platform),
    )
}

/// Resolves application-owned translatable text into a Dioxus string.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "t() consumes the translatable enum into the render call."
)]
pub fn t<T: Translate>(msg: T) -> String {
    try_use_context::<ArsContext>().map_or_else(
        || {
            warn_missing_provider("t");

            let fallback = locales::en_us();

            msg.translate(&fallback, &StubIntlBackend)
        },
        |ctx| msg.translate(&ctx.locale.read(), &*ctx.intl_backend),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        num::NonZero,
        pin::Pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Waker},
    };

    use ars_core::{
        ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects,
        StyleStrategy,
    };
    use ars_i18n::{Direction, IntlBackend, Locale, StubIntlBackend, Translate, locales, number};
    use dioxus::dioxus_core::{NoOpMutations, ScopeId};

    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum AppText {
        Greeting,
    }

    impl Translate for AppText {
        fn translate(&self, locale: &Locale, _intl: &dyn IntlBackend) -> String {
            match locale.language() {
                "es" => String::from("Hola"),
                _ => String::from("Hello"),
            }
        }
    }

    struct TestIntlBackend;

    impl IntlBackend for TestIntlBackend {
        fn weekday_short_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_short_label(weekday, locale)
        }

        fn weekday_long_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_long_label(weekday, locale)
        }

        fn month_long_name(&self, month: u8, locale: &Locale) -> String {
            StubIntlBackend.month_long_name(month, locale)
        }

        fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
            StubIntlBackend.day_period_label(is_pm, locale)
        }

        fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
            StubIntlBackend.day_period_from_char(ch, locale)
        }

        fn format_segment_digits(
            &self,
            value: u32,
            min_digits: NonZero<u8>,
            locale: &Locale,
        ) -> String {
            StubIntlBackend.format_segment_digits(value, min_digits, locale)
        }

        fn hour_cycle(&self, locale: &Locale) -> ars_i18n::HourCycle {
            StubIntlBackend.hour_cycle(locale)
        }

        fn week_info(&self, locale: &Locale) -> ars_i18n::WeekInfo {
            StubIntlBackend.week_info(locale)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TestMessages {
        label: ars_core::MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for TestMessages {
        fn default() -> Self {
            Self {
                label: ars_core::MessageFn::static_str("Default"),
            }
        }
    }

    impl ars_core::ComponentMessages for TestMessages {}

    #[derive(Debug)]
    struct TestPlatform;

    impl DioxusPlatform for TestPlatform {
        fn focus_element(&self, _id: &str) {}

        fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
            None
        }

        fn scroll_into_view(&self, _id: &str) {}

        fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
            Box::pin(async { Ok(()) })
        }

        fn open_file_picker(
            &self,
            _options: FilePickerOptions,
        ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
            Box::pin(async { Vec::new() })
        }

        fn now_ms(&self) -> f64 {
            1.0
        }

        fn new_id(&self) -> String {
            String::from("test-platform-id")
        }

        fn create_drag_data(&self, _event: &dyn Any) -> Option<DragData> {
            None
        }
    }

    fn test_context(locale: Locale, intl_backend: Arc<dyn IntlBackend>) -> ArsContext {
        ArsContext::new(
            locale,
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            intl_backend,
            Arc::new(I18nRegistries::new()),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
    }

    fn block_on_ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        let mut context = Context::from_waker(Waker::noop());

        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test future unexpectedly returned Pending"),
        }
    }

    #[test]
    fn use_locale_falls_back_without_provider() {
        fn app() -> Element {
            assert_eq!(use_locale()().to_bcp47(), "en-US");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_intl_backend_falls_back_without_provider() {
        fn app() -> Element {
            let backend = use_intl_backend();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), backend.as_ref()),
                "Hello"
            );

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_intl_backend_reads_context_value() {
        fn app() -> Element {
            let expected: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::clone(&expected),
                Arc::new(I18nRegistries::new()),
                Arc::new(NullPlatform),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            assert!(Arc::ptr_eq(&use_intl_backend(), &expected));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_modality_context_reads_context_value() {
        fn app() -> Element {
            let expected: Arc<dyn ModalityContext> = Arc::new(DefaultModalityContext::new());

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::clone(&expected),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::new(NullPlatform),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_modality_context_falls_back_without_provider() {
        fn app() -> Element {
            let first = use_modality_context();
            let second = use_modality_context();

            assert_eq!(first.snapshot(), ars_core::ModalitySnapshot::default());
            assert_eq!(second.snapshot(), ars_core::ModalitySnapshot::default());
            assert!(!Arc::ptr_eq(&first, &second));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_messages_reads_provider_registry_bundle() {
        fn app() -> Element {
            let mut registries = I18nRegistries::new();

            registries.register(
                ars_core::MessagesRegistry::new(TestMessages::default()).register(
                    "es",
                    TestMessages {
                        label: ars_core::MessageFn::static_str("Etiqueta"),
                    },
                ),
            );

            let ctx = ArsContext::new(
                Locale::parse("es-MX").expect("locale should parse"),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(registries),
                Arc::new(NullPlatform),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            let locale = Locale::parse("es-MX").expect("locale should parse");

            let resolved = use_messages::<TestMessages>(None, None);

            assert_eq!((resolved.label)(&locale), "Etiqueta");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_messages_falls_back_without_provider() {
        fn app() -> Element {
            let locale = Locale::parse("pt-BR").expect("locale should parse");

            let resolved = use_messages::<TestMessages>(None, Some(&locale));

            assert_eq!((resolved.label)(&locale), "Default");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_number_formatter_falls_back_without_provider() {
        fn app() -> Element {
            let formatter = use_number_formatter(number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1,234.56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_number_formatter_reads_context_locale() {
        fn app() -> Element {
            let ctx = test_context(locales::de_de(), Arc::new(StubIntlBackend));

            use_context_provider(|| ctx);

            let formatter = use_number_formatter(number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1.234,56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_number_formatter_recomputes_when_locale_changes() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let mut ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let formatter = use_number_formatter(number::FormatOptions::default);

            outputs.borrow_mut().push(formatter.read().format(1234.56));

            if phase() == 0 {
                phase.set(1);
                ctx.locale.set(locales::de_de());
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["1,234.56", "1.234,56"]);
    }

    #[test]
    fn use_resolved_number_formatter_prefers_explicit_override() {
        fn app() -> Element {
            let ctx = test_context(locales::fr(), Arc::new(StubIntlBackend));

            use_context_provider(|| ctx);

            let explicit = locales::de_de();

            let formatter =
                use_resolved_number_formatter(Some(&explicit), number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1.234,56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_resolved_number_formatter_recomputes_when_explicit_locale_changes() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let ctx = test_context(locales::fr(), Arc::new(StubIntlBackend));

            use_context_provider(|| ctx);

            let mut phase = use_signal(|| 0u8);

            let mut use_german_locale = use_signal(|| false);

            let explicit = if use_german_locale() {
                locales::de_de()
            } else {
                locales::en_us()
            };

            let formatter =
                use_resolved_number_formatter(Some(&explicit), number::FormatOptions::default);

            outputs.borrow_mut().push(formatter.read().format(1234.56));

            if phase() == 0 {
                phase.set(1);

                use_german_locale.set(true);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["1,234.56", "1.234,56"]);
    }

    #[test]
    fn use_number_formatter_recomputes_when_non_reactive_options_change() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let _ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let mut use_percent = use_signal(|| false);

            let options = if use_percent() {
                number::FormatOptions {
                    style: ars_i18n::number::Style::Percent,
                    ..number::FormatOptions::default()
                }
            } else {
                number::FormatOptions::default()
            };

            let formatter = use_number_formatter(move || options.clone());

            outputs.borrow_mut().push(formatter.read().format(0.47));

            if phase() == 0 {
                phase.set(1);

                use_percent.set(true);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["0.47", "47%"]);
    }

    #[test]
    fn t_reads_provider_locale_and_reacts_on_rerender() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let mut ctx = use_context_provider(|| {
                ArsContext::new(
                    locales::en_us(),
                    Direction::Ltr,
                    ColorMode::System,
                    false,
                    false,
                    None,
                    None,
                    None,
                    Arc::new(NullPlatformEffects),
                    Arc::new(DefaultModalityContext::new()),
                    Arc::new(StubIntlBackend),
                    Arc::new(I18nRegistries::new()),
                    Arc::new(NullPlatform),
                    StyleStrategy::Inline,
                )
            });

            let mut phase = use_signal(|| 0u8);

            let text = t(AppText::Greeting);

            outputs.borrow_mut().push(text.clone());

            if phase() == 0 {
                phase.set(1);

                ctx.locale
                    .set(Locale::parse("es-ES").expect("locale should parse"));
            }

            rsx! {
                div { "{text}" }
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["Hello", "Hola"]);
    }

    #[test]
    fn t_falls_back_without_provider() {
        fn app() -> Element {
            assert_eq!(t(AppText::Greeting), "Hello");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn prelude_exports_compile() {
        fn app() -> Element {
            use crate::prelude::use_number_formatter as prelude_use_number_formatter;

            let ctx = test_context(locales::en_us(), Arc::new(StubIntlBackend));

            use_context_provider(|| ctx);

            drop(t(AppText::Greeting));

            let _ = prelude_use_number_formatter(number::FormatOptions::default);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn resolve_locale_prefers_explicit_override() {
        fn app() -> Element {
            let ctx = test_context(
                Locale::parse("fr-FR").expect("locale should parse"),
                Arc::new(StubIntlBackend),
            );

            use_context_provider(|| ctx);

            let explicit = Locale::parse("es-ES").expect("locale should parse");

            assert_eq!(resolve_locale(Some(&explicit)).to_bcp47(), "es-ES");
            assert_eq!(resolve_locale(None).to_bcp47(), "fr-FR");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_platform_reads_context_value() {
        fn app() -> Element {
            let expected: Arc<dyn DioxusPlatform> = Arc::new(TestPlatform);

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::clone(&expected),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            let platform = use_platform();

            assert!(Arc::ptr_eq(&platform, &expected));

            platform.focus_element("target");

            assert!(platform.get_bounding_rect("target").is_none());

            platform.scroll_into_view("target");

            assert_eq!(block_on_ready(platform.set_clipboard("hello")), Ok(()));
            assert!(block_on_ready(platform.open_file_picker(FilePickerOptions)).is_empty());
            assert_eq!(platform.new_id(), "test-platform-id");
            assert!(platform.create_drag_data(&()).is_none());

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_platform_falls_back_without_provider() {
        fn app() -> Element {
            let platform = use_platform();

            let first = platform.new_id();

            let second = platform.new_id();

            assert!(first.starts_with("null-id-"));
            assert!(second.starts_with("null-id-"));
            assert_ne!(first, second);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn null_platform_is_noop_and_returns_default_values() {
        let platform = NullPlatform;

        platform.focus_element("missing");

        assert!(platform.get_bounding_rect("missing").is_none());

        platform.scroll_into_view("missing");

        assert_eq!(block_on_ready(platform.set_clipboard("text")), Ok(()));
        assert!(block_on_ready(platform.open_file_picker(FilePickerOptions)).is_empty());
        assert_eq!(platform.now_ms(), 0.0);
        assert!(platform.new_id().starts_with("null-id-"));
        assert!(platform.create_drag_data(&()).is_none());
    }

    #[test]
    fn ars_context_debug_includes_struct_name() {
        fn app() -> Element {
            let ctx = test_context(locales::en_us(), Arc::new(StubIntlBackend));

            assert!(format!("{ctx:?}").contains("ArsContext"));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn ars_provider_props_debug_and_equality_follow_semantic_fields() {
        fn app() -> Element {
            let shared_platform: Arc<dyn PlatformEffects> = Arc::new(NullPlatformEffects);
            let shared_intl_backend: Arc<dyn IntlBackend> = Arc::new(StubIntlBackend);
            let shared_registries = Arc::new(I18nRegistries::new());
            let shared_dioxus_platform: Arc<dyn DioxusPlatform> = Arc::new(NullPlatform);

            let baseline = ArsProviderProps {
                locale: None,
                direction: None,
                color_mode: None,
                disabled: None,
                read_only: None,
                id_prefix: Some(String::from("app")),
                portal_container_id: Some(String::from("portal-root")),
                root_node_id: Some(String::from("focus-root")),
                platform: Some(Arc::clone(&shared_platform)),
                intl_backend: Some(Arc::clone(&shared_intl_backend)),
                i18n_registries: Some(Arc::clone(&shared_registries)),
                style_strategy: Some(StyleStrategy::Cssom),
                dioxus_platform: Some(Arc::clone(&shared_dioxus_platform)),
                children: rsx! {
                    div {}
                },
            };

            let same_semantics = baseline.clone();

            let different_children = ArsProviderProps {
                children: rsx! {
                    span {}
                },
                ..baseline.clone()
            };
            let different_platform = ArsProviderProps {
                platform: Some(Arc::new(NullPlatformEffects)),
                ..baseline.clone()
            };

            assert_eq!(baseline, same_semantics);
            assert_ne!(baseline, different_children);
            assert_ne!(baseline, different_platform);

            let debug_output = format!("{baseline:?}");

            assert!(debug_output.contains("ArsProviderProps"));
            assert!(debug_output.contains("portal-root"));
            assert!(debug_output.contains("style_strategy: Some(Cssom)"));
            assert!(debug_output.contains("platform: Some(\"Arc(..)\")"));
            assert!(debug_output.contains("children: \"<Element>\""));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{
        any::Any,
        cell::RefCell,
        future::Future,
        pin::Pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Waker},
    };

    use ars_core::{
        ColorMode, I18nRegistries, ModalityContext, NullPlatformEffects, PlatformEffects, Rect,
        StyleStrategy,
    };
    use ars_forms::field::FileRef;
    use ars_i18n::{Direction, IntlBackend, Locale, StubIntlBackend, Translate, locales, number};
    use dioxus::{
        dioxus_core::{AttributeValue, Mutation, Mutations, NoOpMutations, ScopeId, VirtualDom},
        prelude::*,
    };
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum AppText {
        Greeting,
    }

    impl Translate for AppText {
        fn translate(&self, locale: &Locale, _intl: &dyn IntlBackend) -> String {
            match locale.language() {
                "es" => String::from("Hola"),
                _ => String::from("Hello"),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TestMessages {
        label: ars_core::MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for TestMessages {
        fn default() -> Self {
            Self {
                label: ars_core::MessageFn::static_str("Default"),
            }
        }
    }

    impl ars_core::ComponentMessages for TestMessages {}

    struct TestIntlBackend;

    impl IntlBackend for TestIntlBackend {
        fn weekday_short_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_short_label(weekday, locale)
        }

        fn weekday_long_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_long_label(weekday, locale)
        }

        fn month_long_name(&self, month: u8, locale: &Locale) -> String {
            StubIntlBackend.month_long_name(month, locale)
        }

        fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
            StubIntlBackend.day_period_label(is_pm, locale)
        }

        fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
            StubIntlBackend.day_period_from_char(ch, locale)
        }

        fn format_segment_digits(
            &self,
            value: u32,
            min_digits: core::num::NonZero<u8>,
            locale: &Locale,
        ) -> String {
            StubIntlBackend.format_segment_digits(value, min_digits, locale)
        }

        fn hour_cycle(&self, locale: &Locale) -> ars_i18n::HourCycle {
            StubIntlBackend.hour_cycle(locale)
        }

        fn week_info(&self, locale: &Locale) -> ars_i18n::WeekInfo {
            StubIntlBackend.week_info(locale)
        }
    }

    fn test_context(locale: Locale, intl_backend: Arc<dyn IntlBackend>) -> ArsContext {
        ArsContext::new(
            locale,
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            intl_backend,
            Arc::new(I18nRegistries::new()),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
    }

    fn test_context_with_registries(
        locale: Locale,
        intl_backend: Arc<dyn IntlBackend>,
        i18n_registries: Arc<I18nRegistries>,
    ) -> ArsContext {
        ArsContext::new(
            locale,
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            intl_backend,
            i18n_registries,
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
    }

    fn block_on_ready<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        let mut context = Context::from_waker(Waker::noop());

        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test future unexpectedly returned Pending"),
        }
    }

    #[derive(Debug)]
    struct TestPlatform;

    impl DioxusPlatform for TestPlatform {
        fn focus_element(&self, _id: &str) {}

        fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
            None
        }

        fn scroll_into_view(&self, _id: &str) {}

        fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
            Box::pin(async { Ok(()) })
        }

        fn open_file_picker(
            &self,
            _options: FilePickerOptions,
        ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
            Box::pin(async { Vec::new() })
        }

        fn now_ms(&self) -> f64 {
            1.0
        }

        fn new_id(&self) -> String {
            String::from("test-platform-id")
        }

        fn create_drag_data(&self, _event: &dyn Any) -> Option<super::DragData> {
            None
        }
    }

    #[wasm_bindgen_test]
    fn ars_context_new_and_provider_props_semantics_work_on_wasm() {
        fn app() -> Element {
            let context = ArsContext::new(
                Locale::parse("ar-SA").expect("locale should parse"),
                Direction::Rtl,
                ColorMode::Dark,
                true,
                true,
                Some(String::from("app")),
                Some(String::from("portal-root")),
                Some(String::from("focus-root")),
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::new(NullPlatform),
                StyleStrategy::Cssom,
            );

            assert_eq!(context.locale.read().to_bcp47(), "ar-SA");
            assert_eq!(*context.direction.read(), Direction::Rtl);
            assert_eq!(*context.color_mode.read(), ColorMode::Dark);
            assert!(*context.disabled.read());
            assert!(*context.read_only.read());
            assert_eq!(context.id_prefix.read().as_deref(), Some("app"));
            assert_eq!(
                context.portal_container_id.read().as_deref(),
                Some("portal-root")
            );
            assert_eq!(context.root_node_id.read().as_deref(), Some("focus-root"));
            assert_eq!(context.style_strategy(), &StyleStrategy::Cssom);

            let context_debug = format!("{context:?}");

            assert!(context_debug.contains("ArsContext"));
            assert!(context_debug.contains("style_strategy: Cssom"));

            let shared_platform: Arc<dyn PlatformEffects> = Arc::new(NullPlatformEffects);
            let shared_intl_backend: Arc<dyn IntlBackend> = Arc::new(StubIntlBackend);
            let shared_registries = Arc::new(I18nRegistries::new());
            let shared_dioxus_platform: Arc<dyn DioxusPlatform> = Arc::new(NullPlatform);

            let baseline = ArsProviderProps {
                locale: None,
                direction: None,
                color_mode: None,
                disabled: None,
                read_only: None,
                id_prefix: Some(String::from("app")),
                portal_container_id: Some(String::from("portal-root")),
                root_node_id: Some(String::from("focus-root")),
                platform: Some(Arc::clone(&shared_platform)),
                intl_backend: Some(Arc::clone(&shared_intl_backend)),
                i18n_registries: Some(Arc::clone(&shared_registries)),
                style_strategy: Some(StyleStrategy::Cssom),
                dioxus_platform: Some(Arc::clone(&shared_dioxus_platform)),
                children: rsx! {
                    div {}
                },
            };

            let same_semantics = baseline.clone();

            let different_children = ArsProviderProps {
                children: rsx! {
                    span {}
                },
                ..baseline.clone()
            };
            let different_platform = ArsProviderProps {
                platform: Some(Arc::new(NullPlatformEffects)),
                ..baseline.clone()
            };

            assert_eq!(baseline, same_semantics);
            assert_ne!(baseline, different_children);
            assert_ne!(baseline, different_platform);

            let debug_output = format!("{baseline:?}");

            assert!(debug_output.contains("ArsProviderProps"));
            assert!(debug_output.contains("portal-root"));
            assert!(debug_output.contains("style_strategy: Some(Cssom)"));
            assert!(debug_output.contains("platform: Some(\"Arc(..)\")"));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn ars_provider_props_semantics_cover_early_field_mismatches_on_wasm() {
        fn app() -> Element {
            let locale = use_signal(locales::en_us);
            let direction = use_signal(|| Direction::Ltr);
            let color_mode = use_signal(|| ColorMode::System);
            let disabled = use_signal(|| false);
            let read_only = use_signal(|| false);

            let shared_platform: Arc<dyn PlatformEffects> = Arc::new(NullPlatformEffects);
            let shared_intl_backend: Arc<dyn IntlBackend> = Arc::new(StubIntlBackend);
            let shared_registries = Arc::new(I18nRegistries::new());
            let shared_dioxus_platform: Arc<dyn DioxusPlatform> = Arc::new(NullPlatform);

            let baseline = ArsProviderProps {
                locale: Some(locale),
                direction: Some(direction),
                color_mode: Some(color_mode),
                disabled: Some(disabled),
                read_only: Some(read_only),
                id_prefix: Some(String::from("app")),
                portal_container_id: Some(String::from("portal-root")),
                root_node_id: Some(String::from("focus-root")),
                platform: Some(Arc::clone(&shared_platform)),
                intl_backend: Some(Arc::clone(&shared_intl_backend)),
                i18n_registries: Some(Arc::clone(&shared_registries)),
                style_strategy: Some(StyleStrategy::Cssom),
                dioxus_platform: Some(Arc::clone(&shared_dioxus_platform)),
                children: rsx! {
                    div {}
                },
            };

            assert_ne!(
                baseline,
                ArsProviderProps {
                    locale: Some(use_signal(locales::de_de)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    direction: Some(use_signal(|| Direction::Rtl)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    color_mode: Some(use_signal(|| ColorMode::Dark)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    disabled: Some(use_signal(|| true)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    read_only: Some(use_signal(|| true)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    id_prefix: Some(String::from("other")),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    portal_container_id: Some(String::from("other-portal")),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    root_node_id: Some(String::from("other-root")),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    style_strategy: Some(StyleStrategy::Inline),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    intl_backend: Some(Arc::new(TestIntlBackend)),
                    ..baseline.clone()
                }
            );
            assert_ne!(
                baseline,
                ArsProviderProps {
                    i18n_registries: Some(Arc::new(I18nRegistries::new())),
                    ..baseline.clone()
                }
            );

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn default_dioxus_platform_uses_web_feature_path() {
        let platform = default_dioxus_platform();

        platform.focus_element("missing");
        platform.scroll_into_view("missing");

        assert!(platform.get_bounding_rect("missing").is_none());
        assert!(platform.new_id().starts_with("null-id-"));
        assert!(platform.create_drag_data(&()).is_none());
    }

    #[wasm_bindgen_test]
    fn use_locale_and_intl_backend_fall_back_without_provider_on_wasm() {
        fn app() -> Element {
            assert_eq!(use_locale()().to_bcp47(), "en-US");

            let backend = use_intl_backend();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), &*backend),
                "Hello"
            );

            let formatter = use_number_formatter(number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1,234.56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_intl_backend_reads_context_value_on_wasm() {
        fn app() -> Element {
            let expected: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

            use_context_provider(|| test_context(locales::en_us(), Arc::clone(&expected)));

            assert!(Arc::ptr_eq(&use_intl_backend(), &expected));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_modality_context_reads_context_value_on_wasm() {
        fn app() -> Element {
            let expected: Arc<dyn ModalityContext> = Arc::new(DefaultModalityContext::new());

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::clone(&expected),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::new(NullPlatform),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn provider_fallbacks_cover_modality_messages_platform_and_text_on_wasm() {
        fn app() -> Element {
            let modality = use_modality_context();
            let platform = use_platform();
            let locale = Locale::parse("pt-BR").expect("locale should parse");
            let resolved = use_messages::<TestMessages>(None, Some(&locale));

            assert_eq!(modality.snapshot(), ars_core::ModalitySnapshot::default());
            assert_eq!((resolved.label)(&locale), "Default");
            assert_eq!(t(AppText::Greeting), "Hello");

            let generated_id = platform.new_id();

            assert!(generated_id.starts_with("null-id-"));
            assert!(platform.get_bounding_rect("missing").is_none());
            assert!(platform.create_drag_data(&()).is_none());

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn provider_context_helpers_read_registered_messages_and_modality_on_wasm() {
        fn app() -> Element {
            let expected_modality: Arc<dyn ModalityContext> =
                Arc::new(DefaultModalityContext::new());

            let mut registries = I18nRegistries::new();

            registries.register(
                ars_core::MessagesRegistry::new(TestMessages::default()).register(
                    "es",
                    TestMessages {
                        label: ars_core::MessageFn::static_str("Etiqueta"),
                    },
                ),
            );

            let mut ctx = test_context_with_registries(
                Locale::parse("es-MX").expect("locale should parse"),
                Arc::new(StubIntlBackend),
                Arc::new(registries),
            );

            ctx.modality = Arc::clone(&expected_modality);

            use_context_provider(|| ctx);

            let locale = Locale::parse("es-MX").expect("locale should parse");

            let resolved = use_messages::<TestMessages>(None, None);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected_modality));
            assert_eq!((resolved.label)(&locale), "Etiqueta");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn t_falls_back_without_provider_on_wasm() {
        fn app() -> Element {
            assert_eq!(t(AppText::Greeting), "Hello");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn t_reads_provider_locale_and_reacts_on_rerender_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let mut ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let text = t(AppText::Greeting);

            outputs.borrow_mut().push(text.clone());

            if phase() == 0 {
                phase.set(1);

                ctx.locale
                    .set(Locale::parse("es-ES").expect("locale should parse"));
            }

            rsx! {
                div { "{text}" }
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["Hello", "Hola"]);
    }

    #[wasm_bindgen_test]
    fn use_platform_and_explicit_locale_work_on_wasm() {
        fn app() -> Element {
            let expected: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

            let ctx = test_context(locales::en_us(), Arc::clone(&expected));

            use_context_provider(|| ctx);

            let explicit = Locale::parse("pt-BR").expect("locale should parse");

            assert_eq!(resolve_locale(Some(&explicit)).to_bcp47(), "pt-BR");
            assert_eq!(resolve_locale(None).to_bcp47(), "en-US");
            assert!(Arc::ptr_eq(&use_intl_backend(), &expected));

            let platform = use_platform();

            let first = platform.new_id();
            let second = platform.new_id();

            assert!(first.starts_with("null-id-"));
            assert!(second.starts_with("null-id-"));
            assert_ne!(first, second);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_platform_reads_context_value_on_wasm() {
        fn app() -> Element {
            let expected: Arc<dyn DioxusPlatform> = Arc::new(TestPlatform);

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::clone(&expected),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            let platform = use_platform();

            assert!(Arc::ptr_eq(&platform, &expected));

            platform.focus_element("target");
            platform.scroll_into_view("target");

            assert!(platform.get_bounding_rect("target").is_none());
            assert_eq!(block_on_ready(platform.set_clipboard("hello")), Ok(()));
            assert!(block_on_ready(platform.open_file_picker(FilePickerOptions)).is_empty());
            assert_eq!(platform.new_id(), "test-platform-id");
            assert!(platform.create_drag_data(&()).is_none());

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_reads_context_locale_on_wasm() {
        fn app() -> Element {
            use_context_provider(|| test_context(locales::de_de(), Arc::new(StubIntlBackend)));

            let formatter = use_number_formatter(number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1.234,56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_recomputes_when_locale_changes_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let mut ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let formatter = use_number_formatter(number::FormatOptions::default);

            outputs.borrow_mut().push(formatter.read().format(1234.56));

            if phase() == 0 {
                phase.set(1);

                ctx.locale.set(locales::de_de());
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["1,234.56", "1.234,56"]);
    }

    #[wasm_bindgen_test]
    fn use_resolved_number_formatter_prefers_explicit_locale_override_on_wasm() {
        fn app() -> Element {
            let ctx = test_context(locales::fr(), Arc::new(StubIntlBackend));

            use_context_provider(|| ctx);

            let explicit = locales::de_de();

            let formatter =
                use_resolved_number_formatter(Some(&explicit), number::FormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1.234,56");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn use_resolved_number_formatter_recomputes_when_explicit_locale_changes_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            use_context_provider(|| test_context(locales::fr(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let mut use_german_locale = use_signal(|| false);

            let explicit = if use_german_locale() {
                locales::de_de()
            } else {
                locales::en_us()
            };

            let formatter =
                use_resolved_number_formatter(Some(&explicit), number::FormatOptions::default);

            outputs.borrow_mut().push(formatter.read().format(1234.56));

            if phase() == 0 {
                phase.set(1);

                use_german_locale.set(true);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["1,234.56", "1.234,56"]);
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_recomputes_when_non_reactive_options_change_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let _ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIntlBackend)));

            let mut phase = use_signal(|| 0u8);

            let mut use_percent = use_signal(|| false);

            let options = if use_percent() {
                number::FormatOptions {
                    style: ars_i18n::number::Style::Percent,
                    ..number::FormatOptions::default()
                }
            } else {
                number::FormatOptions::default()
            };

            let formatter = use_number_formatter(move || options.clone());

            outputs.borrow_mut().push(formatter.read().format(0.47));

            if phase() == 0 {
                phase.set(1);
                use_percent.set(true);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(outputs.borrow().as_slice(), ["0.47", "47%"]);
    }

    #[component]
    fn ProviderProbe() -> Element {
        let context = try_use_context::<ArsContext>().expect("ArsProvider should publish context");

        let locale = use_locale();

        let direction = context.direction.read().as_html_attr();

        rsx! {
            div { "data-testid": "probe",
                span { "data-testid": "locale", "{locale().to_bcp47()}" }
                span { "data-testid": "direction", "{direction}" }
            }
        }
    }

    #[component]
    fn DefaultProviderApp() -> Element {
        rsx! {
            ArsProvider { ProviderProbe {} }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ObservedProviderConfig {
        locale: String,
        direction: String,
        color_mode: String,
        disabled: bool,
        read_only: bool,
        id_prefix: Option<String>,
        portal_container_id: Option<String>,
        root_node_id: Option<String>,
        style_strategy: String,
    }

    #[derive(Clone, Props)]
    struct ConfiguredProviderProbeProps {
        outputs: Rc<RefCell<Vec<ObservedProviderConfig>>>,
    }

    impl PartialEq for ConfiguredProviderProbeProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.outputs, &other.outputs)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus component props are passed by value"
    )]
    #[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
    fn ConfiguredProviderProbe(props: ConfiguredProviderProbeProps) -> Element {
        let context = try_use_context::<ArsContext>().expect("ArsProvider should publish context");

        let locale = use_locale();

        props.outputs.borrow_mut().push(ObservedProviderConfig {
            locale: locale().to_bcp47(),
            direction: String::from(context.direction.read().as_html_attr()),
            color_mode: format!("{:?}", *context.color_mode.read()),
            disabled: *context.disabled.read(),
            read_only: *context.read_only.read(),
            id_prefix: context.id_prefix.read().clone(),
            portal_container_id: context.portal_container_id.read().clone(),
            root_node_id: context.root_node_id.read().clone(),
            style_strategy: format!("{:?}", context.style_strategy()),
        });

        rsx! {
            div {}
        }
    }

    #[derive(Clone, Props)]
    struct ObservedProbeProps {
        outputs: Rc<RefCell<Vec<(String, String)>>>,
        locale: Signal<Locale>,
        phase: Signal<u8>,
    }

    impl PartialEq for ObservedProbeProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.outputs, &other.outputs)
                && self.locale == other.locale
                && self.phase == other.phase
        }
    }

    #[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
    fn ObservedProbe(mut props: ObservedProbeProps) -> Element {
        let context = try_use_context::<ArsContext>().expect("ArsProvider should publish context");

        let locale = use_locale();

        let direction = context.direction.read().as_html_attr();

        props
            .outputs
            .borrow_mut()
            .push((locale().to_bcp47(), String::from(direction)));

        if (props.phase)() == 0 {
            props.phase.set(1);

            props
                .locale
                .set(Locale::parse("ar-SA").expect("locale should parse"));
        }

        rsx! {
            div {}
        }
    }

    #[wasm_bindgen_test]
    fn ars_provider_renders_default_locale_and_dir_wrapper_on_wasm() {
        let mut dom = VirtualDom::new(DefaultProviderApp);

        let mut mutations = Mutations::default();

        dom.rebuild(&mut mutations);

        assert!(mutations.edits.iter().any(|edit| {
            matches!(
                edit,
                Mutation::SetAttribute {
                    name: "dir",
                    value: AttributeValue::Text(value),
                    ..
                } if value == "ltr"
            )
        }));
        assert!(mutations.edits.iter().any(|edit| {
            matches!(edit, Mutation::CreateTextNode { value, .. } if value == "en-US")
        }));
    }

    #[wasm_bindgen_test]
    fn ars_provider_reacts_to_locale_changes_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        fn app(outputs: Rc<RefCell<Vec<(String, String)>>>) -> Element {
            let locale = use_signal(locales::en_us);

            let phase = use_signal(|| 0u8);

            rsx! {
                ArsProvider { locale,
                    ObservedProbe { outputs, locale, phase }
                }
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            outputs.borrow().as_slice(),
            [
                ("en-US".into(), "ltr".into()),
                ("ar-SA".into(), "rtl".into())
            ]
        );
    }

    #[wasm_bindgen_test]
    fn ars_provider_respects_explicit_direction_and_optional_context_values_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        fn app(outputs: Rc<RefCell<Vec<ObservedProviderConfig>>>) -> Element {
            let locale = use_signal(|| Locale::parse("ar-SA").expect("locale should parse"));

            let direction = use_signal(|| Direction::Ltr);

            let color_mode = use_signal(|| ColorMode::Dark);

            let disabled = use_signal(|| true);

            let read_only = use_signal(|| true);

            rsx! {
                ArsProvider {
                    locale,
                    direction,
                    color_mode,
                    disabled,
                    read_only,
                    id_prefix: "app".to_string(),
                    portal_container_id: "portal-root".to_string(),
                    root_node_id: "focus-root".to_string(),
                    style_strategy: StyleStrategy::Cssom,
                    ConfiguredProviderProbe { outputs }
                }
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

        dom.rebuild_in_place();

        assert_eq!(
            outputs.borrow().as_slice(),
            [ObservedProviderConfig {
                locale: String::from("ar-SA"),
                direction: String::from("ltr"),
                color_mode: String::from("Dark"),
                disabled: true,
                read_only: true,
                id_prefix: Some(String::from("app")),
                portal_container_id: Some(String::from("portal-root")),
                root_node_id: Some(String::from("focus-root")),
                style_strategy: String::from("Cssom"),
            }]
        );
    }
}
