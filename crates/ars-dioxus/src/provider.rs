//! Reactive `ArsProvider` context helpers for the Dioxus adapter.

use std::{any::Any, pin::Pin, sync::Arc};

use ars_core::{
    ColorMode, I18nRegistries, PlatformEffects, Rect, StyleStrategy,
    resolve_messages as core_resolve_messages,
};
use ars_forms::field::FileRef;
use ars_i18n::{
    Direction, IcuProvider, Locale, NumberFormatOptions, NumberFormatter, StubIcuProvider,
    Translate, locales,
};
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

    /// ICU-backed locale data provider.
    pub icu_provider: Arc<dyn IcuProvider>,

    /// Application-owned message registries.
    pub i18n_registries: Arc<I18nRegistries>,

    /// Dioxus-specific platform services.
    pub dioxus_platform: Arc<dyn DioxusPlatform>,

    /// CSS style injection strategy for all descendant ars components.
    style_strategy: StyleStrategy,
}

impl std::fmt::Debug for ArsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            .field("icu_provider", &"Arc(..)")
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
        icu_provider: Arc<dyn IcuProvider>,
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
            icu_provider,
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
#[must_use]
pub(crate) fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| use_locale().read().clone())
}

/// Resolves a memoized number formatter from the current provider locale.
#[must_use]
pub fn use_number_formatter<F>(options: F) -> Memo<NumberFormatter>
where
    F: Fn() -> NumberFormatOptions + 'static,
{
    use_resolved_number_formatter(None, options)
}

/// Resolves a memoized number formatter from an explicit locale or provider context.
#[must_use]
pub(crate) fn use_resolved_number_formatter<F>(
    adapter_props_locale: Option<&Locale>,
    options: F,
) -> Memo<NumberFormatter>
where
    F: Fn() -> NumberFormatOptions + 'static,
{
    let explicit_locale = adapter_props_locale.cloned();
    let locale = use_locale();
    let resolved_options = options();

    use_memo(use_reactive!(|explicit_locale, resolved_options| {
        let resolved_locale = explicit_locale
            .clone()
            .unwrap_or_else(|| locale.read().clone());

        NumberFormatter::new(&resolved_locale, resolved_options.clone())
    }))
}

/// Resolves the current ICU provider from provider context.
#[must_use]
pub fn use_icu_provider() -> Arc<dyn IcuProvider> {
    try_use_context::<ArsContext>().map_or_else(
        || -> Arc<dyn IcuProvider> {
            warn_missing_provider("use_icu_provider");

            Arc::new(StubIcuProvider)
        },
        |ctx| -> Arc<dyn IcuProvider> { Arc::clone(&ctx.icu_provider) },
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

            msg.translate(&fallback, &StubIcuProvider)
        },
        |ctx| msg.translate(&ctx.locale.read(), &*ctx.icu_provider),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        pin::Pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Poll, Waker},
    };

    use ars_core::{ColorMode, I18nRegistries, NullPlatformEffects, StyleStrategy};
    use ars_i18n::{
        Direction, IcuProvider, Locale, NumberFormatOptions, StubIcuProvider, Translate, locales,
    };
    use dioxus::dioxus_core::{NoOpMutations, ScopeId};

    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum AppText {
        Greeting,
    }

    impl Translate for AppText {
        fn translate(&self, locale: &Locale, _icu: &dyn IcuProvider) -> String {
            match locale.language() {
                "es" => String::from("Hola"),
                _ => String::from("Hello"),
            }
        }
    }

    struct TestIcuProvider;

    impl IcuProvider for TestIcuProvider {}

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

    fn test_context(locale: Locale, icu_provider: Arc<dyn IcuProvider>) -> ArsContext {
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
            icu_provider,
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
    fn use_icu_provider_falls_back_without_provider() {
        fn app() -> Element {
            let provider = use_icu_provider();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), provider.as_ref()),
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
    fn use_icu_provider_reads_context_value() {
        fn app() -> Element {
            let expected: Arc<dyn IcuProvider> = Arc::new(TestIcuProvider);

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
                Arc::new(I18nRegistries::new()),
                Arc::new(NullPlatform),
                StyleStrategy::Inline,
            );

            use_context_provider(|| ctx);

            assert!(Arc::ptr_eq(&use_icu_provider(), &expected));

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
                Arc::new(StubIcuProvider),
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
            let formatter = use_number_formatter(NumberFormatOptions::default);

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
            let ctx = test_context(locales::de_de(), Arc::new(StubIcuProvider));

            use_context_provider(|| ctx);

            let formatter = use_number_formatter(NumberFormatOptions::default);

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
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIcuProvider)));

            let mut phase = use_signal(|| 0u8);

            let formatter = use_number_formatter(NumberFormatOptions::default);

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
            let ctx = test_context(locales::fr(), Arc::new(StubIcuProvider));

            use_context_provider(|| ctx);

            let explicit = locales::de_de();

            let formatter =
                use_resolved_number_formatter(Some(&explicit), NumberFormatOptions::default);

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
            let ctx = test_context(locales::fr(), Arc::new(StubIcuProvider));

            use_context_provider(|| ctx);

            let mut phase = use_signal(|| 0u8);
            let mut use_german_locale = use_signal(|| false);

            let explicit = if use_german_locale() {
                locales::de_de()
            } else {
                locales::en_us()
            };

            let formatter =
                use_resolved_number_formatter(Some(&explicit), NumberFormatOptions::default);

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
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIcuProvider)));

            let mut phase = use_signal(|| 0u8);
            let mut use_percent = use_signal(|| false);

            let options = if use_percent() {
                NumberFormatOptions {
                    style: ars_i18n::NumberStyle::Percent,
                    ..NumberFormatOptions::default()
                }
            } else {
                NumberFormatOptions::default()
            };

            let formatter = use_number_formatter({
                let options = options.clone();
                move || options.clone()
            });

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
                    Arc::new(StubIcuProvider),
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

            let ctx = test_context(locales::en_us(), Arc::new(StubIcuProvider));

            use_context_provider(|| ctx);

            drop(t(AppText::Greeting));

            let _ = prelude_use_number_formatter(NumberFormatOptions::default);

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
                Arc::new(StubIcuProvider),
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
                Arc::new(StubIcuProvider),
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
            let ctx = test_context(locales::en_us(), Arc::new(StubIcuProvider));

            assert!(format!("{ctx:?}").contains("ArsContext"));

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
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    use ars_core::{ColorMode, I18nRegistries, NullPlatformEffects, StyleStrategy};
    use ars_i18n::{
        Direction, IcuProvider, Locale, NumberFormatOptions, StubIcuProvider, Translate, locales,
    };
    use dioxus::dioxus_core::{NoOpMutations, ScopeId};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum AppText {
        Greeting,
    }

    impl Translate for AppText {
        fn translate(&self, locale: &Locale, _icu: &dyn IcuProvider) -> String {
            match locale.language() {
                "es" => String::from("Hola"),
                _ => String::from("Hello"),
            }
        }
    }

    struct TestIcuProvider;

    impl IcuProvider for TestIcuProvider {}

    fn test_context(locale: Locale, icu_provider: Arc<dyn IcuProvider>) -> ArsContext {
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
            icu_provider,
            Arc::new(I18nRegistries::new()),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
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
    fn use_locale_and_icu_provider_fall_back_without_provider_on_wasm() {
        fn app() -> Element {
            assert_eq!(use_locale()().to_bcp47(), "en-US");

            let provider = use_icu_provider();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), &*provider),
                "Hello"
            );

            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.read().format(1234.56), "1,234.56");

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
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIcuProvider)));

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
            let expected: Arc<dyn IcuProvider> = Arc::new(TestIcuProvider);

            let ctx = test_context(locales::en_us(), Arc::clone(&expected));

            use_context_provider(|| ctx);

            let explicit = Locale::parse("pt-BR").expect("locale should parse");

            assert_eq!(resolve_locale(Some(&explicit)).to_bcp47(), "pt-BR");
            assert_eq!(resolve_locale(None).to_bcp47(), "en-US");
            assert!(Arc::ptr_eq(&use_icu_provider(), &expected));

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
    fn use_number_formatter_recomputes_when_locale_changes_on_wasm() {
        let outputs = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
            let mut ctx =
                use_context_provider(|| test_context(locales::en_us(), Arc::new(StubIcuProvider)));

            let mut phase = use_signal(|| 0u8);

            let formatter = use_number_formatter(NumberFormatOptions::default);

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
}
