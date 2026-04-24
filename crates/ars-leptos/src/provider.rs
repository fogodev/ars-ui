//! Reactive `ArsProvider` context helpers for the Leptos adapter.
//!
//! This module wraps framework-agnostic provider values in Leptos signals and
//! exposes adapter-level helpers for locale, ICU, message resolution, and
//! reactive application text rendering.

use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects,
    PlatformEffects, StyleStrategy, resolve_messages as core_resolve_messages,
};
use ars_i18n::{
    Direction, IntlBackend, Locale, NumberFormatOptions, NumberFormatter, StubIntlBackend,
    Translate, locales,
};
use leptos::{prelude::*, reactive::owner::LocalStorage};

/// Reactive environment context published by the Leptos `ArsProvider`.
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
        style_strategy: StyleStrategy,
    ) -> Self {
        Self {
            locale: Signal::stored(locale),
            direction: Memo::new(move |_| direction),
            color_mode: Signal::stored(color_mode),
            disabled: Signal::stored(disabled),
            read_only: Signal::stored(read_only),
            id_prefix: Signal::stored(id_prefix),
            portal_container_id: Signal::stored(portal_container_id),
            root_node_id: Signal::stored(root_node_id),
            platform,
            modality,
            intl_backend,
            i18n_registries,
            style_strategy,
        }
    }

    /// Returns the configured style strategy.
    #[must_use]
    pub const fn style_strategy(&self) -> &StyleStrategy {
        &self.style_strategy
    }
}

pub(crate) fn current_ars_context() -> Option<ArsContext> {
    use_context::<ArsContext>()
}

/// Publishes [`ArsContext`] into Leptos context.
pub fn provide_ars_context(context: ArsContext) {
    provide_context(context);
}

fn direction_from_locale(locale: &Locale) -> Direction {
    if locale.direction().is_rtl() {
        Direction::Rtl
    } else {
        Direction::Ltr
    }
}

/// Publishes adapter environment context to a Leptos subtree.
#[component]
#[expect(
    unreachable_pub,
    reason = "ArsProvider is re-exported at the adapter crate root."
)]
#[expect(
    clippy::too_many_arguments,
    reason = "ArsProvider intentionally mirrors the documented provider surface."
)]
pub fn ArsProvider(
    #[prop(optional, into)] locale: Option<Signal<Locale>>,
    #[prop(optional, into)] direction: Option<Signal<Direction>>,
    #[prop(optional, into)] color_mode: Option<Signal<ColorMode>>,
    #[prop(optional, into)] disabled: Option<Signal<bool>>,
    #[prop(optional, into)] read_only: Option<Signal<bool>>,
    #[prop(optional)] id_prefix: Option<String>,
    #[prop(optional)] portal_container_id: Option<String>,
    #[prop(optional)] root_node_id: Option<String>,
    #[prop(optional)] platform: Option<Arc<dyn PlatformEffects>>,
    #[prop(optional)] intl_backend: Option<Arc<dyn IntlBackend>>,
    #[prop(optional)] i18n_registries: Option<Arc<I18nRegistries>>,
    #[prop(optional)] style_strategy: Option<StyleStrategy>,
    children: Children,
) -> impl IntoView {
    let locale = locale.unwrap_or_else(|| Signal::stored(locales::en_us()));

    let direction =
        direction.unwrap_or_else(|| Signal::derive(move || direction_from_locale(&locale.get())));

    let color_mode = color_mode.unwrap_or_else(|| Signal::stored(ColorMode::System));

    let disabled = disabled.unwrap_or_else(|| Signal::stored(false));

    let read_only = read_only.unwrap_or_else(|| Signal::stored(false));

    let dir_attr = move || direction.get().as_html_attr();

    provide_ars_context(ArsContext {
        locale,
        direction: Memo::new(move |_| direction.get()),
        color_mode,
        disabled,
        read_only,
        id_prefix: Signal::stored(id_prefix),
        portal_container_id: Signal::stored(portal_container_id),
        root_node_id: Signal::stored(root_node_id),
        platform: platform.unwrap_or_else(|| Arc::new(NullPlatformEffects)),
        modality: Arc::new(DefaultModalityContext::new()),
        intl_backend: intl_backend.unwrap_or_else(|| Arc::new(StubIntlBackend)),
        i18n_registries: i18n_registries.unwrap_or_else(|| Arc::new(I18nRegistries::new())),
        style_strategy: style_strategy.unwrap_or(StyleStrategy::Inline),
    });

    view! {
        <div dir=dir_attr>{children()}</div>
    }
}

/// Emits a debug warning when a provider-dependent helper is used without context.
#[cfg(feature = "debug")]
pub fn warn_missing_provider(hook_name: &str) {
    log::warn!(
        "[ars-ui] {hook_name}: No ArsProvider found in the component tree. Falling back to defaults."
    );
}

/// No-op outside debug builds.
#[cfg(not(feature = "debug"))]
pub const fn warn_missing_provider(_hook_name: &str) {}

/// Returns the current locale signal from provider context.
#[must_use]
pub fn use_locale() -> Signal<Locale> {
    current_ars_context().map_or_else(
        || {
            warn_missing_provider("use_locale");

            Signal::stored(locales::en_us())
        },
        |ctx| ctx.locale,
    )
}

/// Resolves the current ICU provider from provider context.
#[must_use]
pub fn use_intl_backend() -> Arc<dyn IntlBackend> {
    current_ars_context().map_or_else(
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
    current_ars_context().map_or_else(
        || -> Arc<dyn ModalityContext> {
            warn_missing_provider("use_modality_context");

            Arc::new(DefaultModalityContext::new())
        },
        |ctx| -> Arc<dyn ModalityContext> { Arc::clone(&ctx.modality) },
    )
}

/// Resolves a provider-derived number formatter from the current provider locale.
///
/// Leptos 0.8 only exposes public `Memo` constructors for `Send + Sync` values.
/// `NumberFormatter` is intentionally not guaranteed to be thread-safe on every
/// backend, so the adapter uses a local derived signal plus component-local
/// cache to preserve provider-derived reuse semantics without inventing
/// unsupported thread-safe guarantees.
#[must_use]
pub fn use_number_formatter<F>(options: F) -> Signal<NumberFormatter, LocalStorage>
where
    F: Fn() -> NumberFormatOptions + 'static,
{
    use_resolved_number_formatter(None, options)
}

/// Resolves a provider-derived number formatter from an explicit locale or provider context.
#[must_use]
pub(crate) fn use_resolved_number_formatter<F>(
    adapter_props_locale: Option<&Locale>,
    options: F,
) -> Signal<NumberFormatter, LocalStorage>
where
    F: Fn() -> NumberFormatOptions + 'static,
{
    let explicit_locale = adapter_props_locale.cloned();

    let locale = use_locale();

    let cache =
        StoredValue::<Option<(Locale, NumberFormatOptions, NumberFormatter)>, LocalStorage>::new_local(
            None,
        );

    Signal::derive_local(move || {
        let resolved_locale = explicit_locale.clone().unwrap_or_else(|| locale.get());

        let resolved_options = options();

        let mut resolved_formatter = None;

        cache.update_value(|cached| {
            if let Some((cached_locale, cached_options, cached_formatter)) = cached
                && *cached_locale == resolved_locale
                && *cached_options == resolved_options
            {
                resolved_formatter = Some(cached_formatter.clone());

                return;
            }

            let next_formatter = NumberFormatter::new(&resolved_locale, resolved_options.clone());

            *cached = Some((resolved_locale, resolved_options, next_formatter.clone()));

            resolved_formatter = Some(next_formatter);
        });

        resolved_formatter.expect("formatter cache should always produce a formatter")
    })
}

/// Resolves the effective locale for an adapter component instance.
#[must_use]
pub fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| use_locale().get())
}

/// Resolves per-component messages from override, provider registry, or defaults.
#[must_use]
pub fn use_messages<M: ars_core::ComponentMessages + Send + Sync + 'static>(
    adapter_props_messages: Option<&M>,
    adapter_props_locale: Option<&Locale>,
) -> M {
    let locale = resolve_locale(adapter_props_locale);

    let registries = current_ars_context().map_or_else(
        || Arc::new(I18nRegistries::new()),
        |ctx| Arc::clone(&ctx.i18n_registries),
    );

    core_resolve_messages(adapter_props_messages, registries.as_ref(), &locale)
}

fn translated_text<T: Translate + Send + Sync + 'static>(msg: T) -> impl Fn() -> String {
    let locale = use_locale();

    let intl_backend = use_intl_backend();

    move || msg.translate(&locale.get(), &*intl_backend)
}

/// Resolves application-owned translatable text into a reactive text node.
#[inline]
#[must_use]
pub fn t<T: Translate + Send + Sync + 'static>(msg: T) -> impl IntoView {
    translated_text(msg)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ars_core::{
        ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects,
        StyleStrategy,
    };
    use ars_i18n::{
        Direction, IntlBackend, Locale, NumberFormatOptions, StubIntlBackend, Translate, locales,
    };
    use leptos::prelude::{Get, GetUntracked, Memo, Owner, RwSignal, Set, Signal};

    use super::{
        ArsContext, current_ars_context, resolve_locale, t, translated_text, use_intl_backend,
        use_locale, use_messages, use_modality_context, use_number_formatter,
        use_resolved_number_formatter,
    };

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

    fn reactive_test_context(
        locale: Locale,
        intl_backend: Arc<dyn IntlBackend>,
    ) -> (ArsContext, RwSignal<Locale>) {
        let locale_signal = RwSignal::new(locale);

        let context = ArsContext {
            locale: locale_signal.into(),
            direction: Memo::new(move |_| direction_from_locale(&locale_signal.get())),
            color_mode: Signal::stored(ColorMode::System),
            disabled: Signal::stored(false),
            read_only: Signal::stored(false),
            id_prefix: Signal::stored(None),
            portal_container_id: Signal::stored(None),
            root_node_id: Signal::stored(None),
            platform: Arc::new(NullPlatformEffects),
            modality: Arc::new(DefaultModalityContext::new()),
            intl_backend,
            i18n_registries: Arc::new(I18nRegistries::new()),
            style_strategy: StyleStrategy::Inline,
        };

        (context, locale_signal)
    }

    #[test]
    fn ars_context_debug_redacts_arc_backed_fields() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, _) = reactive_test_context(locales::en_us(), Arc::new(TestIntlBackend));

            let debug = format!("{context:?}");

            assert!(debug.contains("ArsContext"));
            assert!(debug.contains("platform: \"Arc(..)\""));
            assert!(debug.contains("modality: \"Arc(..)\""));
            assert!(debug.contains("intl_backend: \"Arc(..)\""));
            assert!(debug.contains("i18n_registries: \"Arc(..)\""));
        });
    }

    #[test]
    fn test_intl_backend_delegates_required_methods_to_stub() {
        let backend = TestIntlBackend;

        let locale = locales::en_us();

        assert_eq!(
            backend.weekday_short_label(ars_i18n::Weekday::Monday, &locale),
            StubIntlBackend.weekday_short_label(ars_i18n::Weekday::Monday, &locale)
        );
        assert_eq!(
            backend.weekday_long_label(ars_i18n::Weekday::Monday, &locale),
            StubIntlBackend.weekday_long_label(ars_i18n::Weekday::Monday, &locale)
        );
        assert_eq!(
            backend.month_long_name(5, &locale),
            StubIntlBackend.month_long_name(5, &locale)
        );
        assert_eq!(
            backend.day_period_label(false, &locale),
            StubIntlBackend.day_period_label(false, &locale)
        );
        assert_eq!(backend.day_period_from_char('p', &locale), Some(true));
        assert_eq!(
            backend.format_segment_digits(
                7,
                core::num::NonZero::new(2).expect("minimum digits should be non-zero"),
                &locale,
            ),
            "07"
        );
        assert_eq!(
            backend.hour_cycle(&locale),
            StubIntlBackend.hour_cycle(&locale)
        );
        assert_eq!(
            backend.week_info(&locale),
            StubIntlBackend.week_info(&locale)
        );
    }

    fn direction_from_locale(locale: &Locale) -> Direction {
        if locale.direction().is_rtl() {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    fn test_context_with_defaults(
        locale: Locale,
        intl_backend: Arc<dyn IntlBackend>,
    ) -> ArsContext {
        let (context, _) = reactive_test_context(locale, intl_backend);

        context
    }

    #[test]
    fn use_locale_falls_back_to_en_us_without_provider() {
        let owner = Owner::new();
        owner.with(|| {
            assert_eq!(use_locale().get_untracked().to_bcp47(), "en-US");
        });
    }

    #[test]
    fn resolve_locale_prefers_explicit_override() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(test_context_with_defaults(
                Locale::parse("fr-FR").expect("locale should parse"),
                Arc::new(StubIntlBackend),
            ));

            let override_locale = Locale::parse("pt-BR").expect("locale should parse");

            assert_eq!(resolve_locale(Some(&override_locale)).to_bcp47(), "pt-BR");
            assert_eq!(resolve_locale(None).to_bcp47(), "fr-FR");
        });
    }

    #[test]
    fn current_ars_context_round_trips_through_leptos_context() {
        let owner = Owner::new();
        owner.with(|| {
            assert!(current_ars_context().is_none());

            crate::provide_ars_context(test_context_with_defaults(
                Locale::parse("fr-FR").expect("locale should parse"),
                Arc::new(StubIntlBackend),
            ));

            let current = current_ars_context().expect("provider context should exist");

            assert_eq!(current.locale.get_untracked().to_bcp47(), "fr-FR");
        });
    }

    #[test]
    fn use_intl_backend_reads_context_value() {
        let owner = Owner::new();
        owner.with(|| {
            let expected: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

            crate::provide_ars_context(test_context_with_defaults(
                locales::en_us(),
                Arc::clone(&expected),
            ));

            assert!(Arc::ptr_eq(&use_intl_backend(), &expected));
        });
    }

    #[test]
    fn use_modality_context_reads_context_value() {
        let owner = Owner::new();
        owner.with(|| {
            let expected: Arc<dyn ModalityContext> = Arc::new(DefaultModalityContext::new());

            let (mut context, _) =
                reactive_test_context(locales::en_us(), Arc::new(TestIntlBackend));

            context.modality = Arc::clone(&expected);

            crate::provide_ars_context(context);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected));
        });
    }

    #[test]
    fn use_modality_context_falls_back_without_provider() {
        let owner = Owner::new();
        owner.with(|| {
            let first = use_modality_context();
            let second = use_modality_context();

            assert_eq!(first.snapshot(), ars_core::ModalitySnapshot::default());
            assert_eq!(second.snapshot(), ars_core::ModalitySnapshot::default());
            assert!(!Arc::ptr_eq(&first, &second));
        });
    }

    #[test]
    fn use_intl_backend_falls_back_without_provider() {
        let owner = Owner::new();
        owner.with(|| {
            let backend = use_intl_backend();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), &*backend),
                "Hello"
            );
        });
    }

    #[test]
    fn use_messages_uses_provider_registry_bundle() {
        let owner = Owner::new();
        owner.with(|| {
            let mut registries = I18nRegistries::new();

            registries.register(
                ars_core::MessagesRegistry::new(TestMessages::default()).register(
                    "es",
                    TestMessages {
                        label: ars_core::MessageFn::static_str("Etiqueta"),
                    },
                ),
            );

            crate::provide_ars_context(ArsContext::new(
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
                StyleStrategy::Inline,
            ));

            let resolved = use_messages::<TestMessages>(None, None);

            let locale = Locale::parse("es-MX").expect("locale should parse");

            assert_eq!((resolved.label)(&locale), "Etiqueta");
        });
    }

    #[test]
    fn use_messages_falls_back_without_provider() {
        let owner = Owner::new();
        owner.with(|| {
            let locale = Locale::parse("pt-BR").expect("locale should parse");

            let resolved = use_messages::<TestMessages>(None, Some(&locale));

            assert_eq!((resolved.label)(&locale), "Default");
        });
    }

    #[test]
    fn translated_text_reacts_to_locale_changes() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, locale_signal) =
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let text = translated_text(AppText::Greeting);

            assert_eq!(text(), "Hello");

            locale_signal.set(Locale::parse("es-ES").expect("locale should parse"));

            assert_eq!(text(), "Hola");
        });
    }

    #[test]
    fn use_number_formatter_falls_back_without_provider() {
        let owner = Owner::new();
        owner.with(|| {
            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1,234.56");
        });
    }

    #[test]
    fn use_number_formatter_reads_context_locale() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(test_context_with_defaults(
                locales::de_de(),
                Arc::new(StubIntlBackend),
            ));

            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[test]
    fn use_number_formatter_recomputes_when_locale_changes() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, locale_signal) =
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1,234.56");

            locale_signal.set(locales::de_de());

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[test]
    fn use_number_formatter_reuses_cached_formatter_for_identical_inputs() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(test_context_with_defaults(
                locales::en_us(),
                Arc::new(StubIntlBackend),
            ));

            let formatter = use_number_formatter(NumberFormatOptions::default);

            let first = formatter.get_untracked();
            let second = formatter.get_untracked();

            assert_eq!(first, second);
            assert_eq!(second.format(1234.56), "1,234.56");
        });
    }

    #[test]
    fn use_resolved_number_formatter_prefers_explicit_locale_override() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(test_context_with_defaults(
                locales::fr(),
                Arc::new(StubIntlBackend),
            ));

            let explicit = locales::de_de();

            let formatter =
                use_resolved_number_formatter(Some(&explicit), NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[test]
    fn prelude_t_reexport_compiles() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(test_context_with_defaults(
                locales::en_us(),
                Arc::new(StubIntlBackend),
            ));

            drop(t(AppText::Greeting));

            use crate::prelude::use_number_formatter as prelude_use_number_formatter;

            let _ = prelude_use_number_formatter(NumberFormatOptions::default);
        });
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::sync::Arc;

    use ars_core::{
        ColorMode, I18nRegistries, ModalityContext, NullPlatformEffects, StyleStrategy,
    };
    use ars_i18n::{
        Direction, IntlBackend, Locale, NumberFormatOptions, StubIntlBackend, Translate, locales,
    };
    use leptos::prelude::*;
    #[cfg(feature = "csr")]
    use leptos::{mount::mount_to, wasm_bindgen::JsCast};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        ArsContext, ArsProvider, current_ars_context, resolve_locale, t, translated_text,
        use_intl_backend, use_locale, use_messages, use_modality_context, use_number_formatter,
        use_resolved_number_formatter,
    };

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

    fn direction_from_locale(locale: &Locale) -> Direction {
        if locale.direction().is_rtl() {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    fn reactive_test_context(
        locale: Locale,
        intl_backend: Arc<dyn IntlBackend>,
    ) -> (ArsContext, RwSignal<Locale>) {
        let locale_signal = RwSignal::new(locale);

        let context = ArsContext {
            locale: locale_signal.into(),
            direction: Memo::new(move |_| direction_from_locale(&locale_signal.get())),
            color_mode: Signal::stored(ColorMode::System),
            disabled: Signal::stored(false),
            read_only: Signal::stored(false),
            id_prefix: Signal::stored(None),
            portal_container_id: Signal::stored(None),
            root_node_id: Signal::stored(None),
            platform: Arc::new(NullPlatformEffects),
            modality: Arc::new(ars_core::DefaultModalityContext::new()),
            intl_backend,
            i18n_registries: Arc::new(I18nRegistries::new()),
            style_strategy: StyleStrategy::Inline,
        };

        (context, locale_signal)
    }

    #[wasm_bindgen_test]
    fn ars_context_new_and_debug_cover_constructor_fields_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
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
                Arc::new(ars_core::DefaultModalityContext::new()),
                Arc::new(StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                StyleStrategy::Cssom,
            );

            assert_eq!(context.locale.get_untracked().to_bcp47(), "ar-SA");
            assert_eq!(context.direction.get_untracked(), Direction::Rtl);
            assert_eq!(context.color_mode.get_untracked(), ColorMode::Dark);
            assert!(context.disabled.get_untracked());
            assert!(context.read_only.get_untracked());
            assert_eq!(context.id_prefix.get_untracked().as_deref(), Some("app"));
            assert_eq!(
                context.portal_container_id.get_untracked().as_deref(),
                Some("portal-root")
            );
            assert_eq!(
                context.root_node_id.get_untracked().as_deref(),
                Some("focus-root")
            );
            assert_eq!(context.style_strategy(), &StyleStrategy::Cssom);

            let debug_output = format!("{context:?}");

            assert!(debug_output.contains("ArsContext"));
            assert!(debug_output.contains("portal_container_id"));
            assert!(debug_output.contains("style_strategy: Cssom"));
            assert!(debug_output.contains("platform: \"Arc(..)\""));
        });
    }

    #[wasm_bindgen_test]
    fn translated_text_reacts_to_locale_changes_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, locale_signal) =
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let text = translated_text(AppText::Greeting);

            assert_eq!(text(), "Hello");

            locale_signal.set(Locale::parse("es-ES").expect("locale should parse"));

            assert_eq!(text(), "Hola");

            drop(t(AppText::Greeting));
        });
    }

    #[wasm_bindgen_test]
    fn current_ars_context_round_trips_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            assert!(current_ars_context().is_none());

            let (context, _) = reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let current = current_ars_context().expect("provider context should exist");

            assert_eq!(current.locale.get_untracked().to_bcp47(), "en-US");
        });
    }

    #[wasm_bindgen_test]
    fn locale_and_intl_backend_are_readable_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let expected_backend: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

            let (context, _) =
                reactive_test_context(locales::en_us(), Arc::clone(&expected_backend));

            crate::provide_ars_context(context);

            assert_eq!(use_locale().get_untracked().to_bcp47(), "en-US");
            assert!(Arc::ptr_eq(&use_intl_backend(), &expected_backend));
        });
    }

    #[wasm_bindgen_test]
    fn use_modality_context_reads_context_value_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let expected: Arc<dyn ModalityContext> =
                Arc::new(ars_core::DefaultModalityContext::new());

            let (mut context, _) =
                reactive_test_context(locales::en_us(), Arc::new(TestIntlBackend));

            context.modality = Arc::clone(&expected);

            crate::provide_ars_context(context);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected));
        });
    }

    #[wasm_bindgen_test]
    fn use_locale_falls_back_without_provider_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            assert_eq!(use_locale().get_untracked().to_bcp47(), "en-US");
        });
    }

    #[wasm_bindgen_test]
    fn use_intl_backend_falls_back_without_provider_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let backend = use_intl_backend();

            assert_eq!(
                AppText::Greeting.translate(&locales::en_us(), &*backend),
                "Hello"
            );
        });
    }

    #[wasm_bindgen_test]
    fn provider_fallbacks_cover_modality_messages_and_text_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let locale = Locale::parse("pt-BR").expect("locale should parse");

            let modality = use_modality_context();

            let resolved = use_messages::<TestMessages>(None, Some(&locale));

            assert_eq!(modality.snapshot(), ars_core::ModalitySnapshot::default());
            assert_eq!((resolved.label)(&locale), "Default");
            assert_eq!(translated_text(AppText::Greeting)(), "Hello");

            drop(t(AppText::Greeting));
        });
    }

    #[wasm_bindgen_test]
    fn provider_context_helpers_read_registered_messages_and_modality_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let expected: Arc<dyn ModalityContext> =
                Arc::new(ars_core::DefaultModalityContext::new());

            let mut registries = I18nRegistries::new();

            registries.register(
                ars_core::MessagesRegistry::new(TestMessages::default()).register(
                    "es",
                    TestMessages {
                        label: ars_core::MessageFn::static_str("Etiqueta"),
                    },
                ),
            );

            let (mut context, _) = reactive_test_context(
                Locale::parse("es-MX").expect("locale should parse"),
                Arc::new(StubIntlBackend),
            );

            context.modality = Arc::clone(&expected);
            context.i18n_registries = Arc::new(registries);

            crate::provide_ars_context(context);

            let locale = Locale::parse("es-MX").expect("locale should parse");

            let resolved = use_messages::<TestMessages>(None, None);

            assert!(Arc::ptr_eq(&use_modality_context(), &expected));
            assert_eq!((resolved.label)(&locale), "Etiqueta");
        });
    }

    #[wasm_bindgen_test]
    fn resolve_locale_prefers_override_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, _) = reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let override_locale = Locale::parse("pt-BR").expect("locale should parse");

            assert_eq!(resolve_locale(Some(&override_locale)).to_bcp47(), "pt-BR");
            assert_eq!(resolve_locale(None).to_bcp47(), "en-US");
        });
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_falls_back_without_provider_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1,234.56");
        });
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_reads_context_locale_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(
                reactive_test_context(locales::de_de(), Arc::new(StubIntlBackend)).0,
            );

            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_reacts_to_locale_changes_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (context, locale_signal) =
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend));

            crate::provide_ars_context(context);

            let formatter = use_number_formatter(NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1,234.56");

            locale_signal.set(locales::de_de());

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_reuses_cached_formatter_for_identical_inputs_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend)).0,
            );

            let formatter = use_number_formatter(NumberFormatOptions::default);

            let first = formatter.get_untracked();
            let second = formatter.get_untracked();

            assert_eq!(first, second);
            assert_eq!(second.format(1234.56), "1,234.56");
        });
    }

    #[wasm_bindgen_test]
    fn use_number_formatter_recomputes_when_non_reactive_options_change_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(
                reactive_test_context(locales::en_us(), Arc::new(StubIntlBackend)).0,
            );

            let (use_percent, set_use_percent) = signal(false);

            let formatter = use_number_formatter(move || {
                if use_percent.get() {
                    NumberFormatOptions {
                        style: ars_i18n::NumberStyle::Percent,
                        ..NumberFormatOptions::default()
                    }
                } else {
                    NumberFormatOptions::default()
                }
            });

            assert_eq!(formatter.get().format(0.47), "0.47");

            set_use_percent.set(true);

            assert_eq!(formatter.get().format(0.47), "47%");
        });
    }

    #[wasm_bindgen_test]
    fn use_resolved_number_formatter_prefers_explicit_locale_override_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            crate::provide_ars_context(
                reactive_test_context(locales::fr(), Arc::new(StubIntlBackend)).0,
            );

            let explicit = locales::de_de();
            let formatter =
                use_resolved_number_formatter(Some(&explicit), NumberFormatOptions::default);

            assert_eq!(formatter.get().format(1234.56), "1.234,56");
        });
    }

    #[cfg(feature = "csr")]
    fn document() -> leptos::web_sys::Document {
        leptos::web_sys::window()
            .expect("window should exist")
            .document()
            .expect("document should exist")
    }

    #[cfg(feature = "csr")]
    fn append_container() -> leptos::web_sys::HtmlElement {
        let container = document()
            .create_element("div")
            .expect("container creation should succeed")
            .dyn_into::<leptos::web_sys::HtmlElement>()
            .expect("container should be an HtmlElement");

        document()
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("container append should succeed");

        container
    }

    #[cfg(feature = "csr")]
    #[leptos::component]
    fn ProviderProbe() -> impl IntoView {
        let context = current_ars_context().expect("ArsProvider should publish context");

        let locale = use_locale();

        leptos::view! {
            <div data-testid="probe">
                <span data-testid="locale">{move || locale.get().to_bcp47()}</span>
                <span data-testid="direction">
                    {move || context.direction.get().as_html_attr()}
                </span>
            </div>
        }
    }

    #[cfg(feature = "csr")]
    #[leptos::component]
    fn ConfiguredProviderProbe() -> impl IntoView {
        let context = current_ars_context().expect("ArsProvider should publish context");

        let locale = use_locale();

        leptos::view! {
            <div data-testid="configured-probe">
                <span data-testid="locale">{move || locale.get().to_bcp47()}</span>
                <span data-testid="direction">
                    {move || context.direction.get().as_html_attr()}
                </span>
                <span data-testid="color-mode">
                    {move || format!("{:?}", context.color_mode.get())}
                </span>
                <span data-testid="disabled">{move || context.disabled.get().to_string()}</span>
                <span data-testid="read-only">{move || context.read_only.get().to_string()}</span>
                <span data-testid="id-prefix">
                    {move || context.id_prefix.get().unwrap_or_default()}
                </span>
                <span data-testid="portal-container-id">
                    {move || context.portal_container_id.get().unwrap_or_default()}
                </span>
                <span data-testid="root-node-id">
                    {move || context.root_node_id.get().unwrap_or_default()}
                </span>
                <span data-testid="style-strategy">
                    {move || format!("{:?}", context.style_strategy())}
                </span>
            </div>
        }
    }

    #[cfg(feature = "csr")]
    #[wasm_bindgen_test]
    async fn ars_provider_renders_default_locale_and_dir_wrapper_on_wasm() {
        let container = append_container();

        let mount_handle = mount_to(container.clone(), move || {
            leptos::view! {
                <ArsProvider>
                    <ProviderProbe />
                </ArsProvider>
            }
        });

        leptos::task::tick().await;

        let wrapper = container
            .query_selector("[dir='ltr']")
            .expect("selector should be valid")
            .expect("provider wrapper should exist");

        let locale = container
            .query_selector("[data-testid='locale']")
            .expect("selector should be valid")
            .expect("locale node should exist");

        assert_eq!(wrapper.get_attribute("dir").as_deref(), Some("ltr"));
        assert_eq!(locale.text_content().as_deref(), Some("en-US"));

        drop(mount_handle);

        container.remove();
    }

    #[cfg(feature = "csr")]
    #[wasm_bindgen_test]
    async fn ars_provider_reacts_to_locale_changes_on_wasm() {
        let container = append_container();

        let locale = RwSignal::new(locales::en_us());

        let mount_handle = mount_to(container.clone(), move || {
            leptos::view! {
                <ArsProvider locale=locale>
                    <ProviderProbe />
                </ArsProvider>
            }
        });

        leptos::task::tick().await;

        locale.set(Locale::parse("ar-SA").expect("locale should parse"));

        leptos::task::tick().await;

        let wrapper = container
            .query_selector("[dir='rtl']")
            .expect("selector should be valid")
            .expect("provider wrapper should exist");

        let locale_node = container
            .query_selector("[data-testid='locale']")
            .expect("selector should be valid")
            .expect("locale node should exist");

        let direction_node = container
            .query_selector("[data-testid='direction']")
            .expect("selector should be valid")
            .expect("direction node should exist");

        assert_eq!(wrapper.get_attribute("dir").as_deref(), Some("rtl"));
        assert_eq!(locale_node.text_content().as_deref(), Some("ar-SA"));
        assert_eq!(direction_node.text_content().as_deref(), Some("rtl"));

        drop(mount_handle);

        container.remove();
    }

    #[cfg(feature = "csr")]
    #[wasm_bindgen_test]
    async fn ars_provider_respects_explicit_direction_and_optional_context_values_on_wasm() {
        let container = append_container();

        let locale = Signal::stored(Locale::parse("ar-SA").expect("locale should parse"));

        let direction = Signal::stored(Direction::Ltr);

        let color_mode = Signal::stored(ColorMode::Dark);

        let disabled = Signal::stored(true);

        let read_only = Signal::stored(true);

        let mount_handle = mount_to(container.clone(), move || {
            leptos::view! {
                <ArsProvider
                    locale
                    direction
                    color_mode
                    disabled
                    read_only
                    id_prefix=String::from("app")
                    portal_container_id=String::from("portal-root")
                    root_node_id=String::from("focus-root")
                    style_strategy=StyleStrategy::Cssom
                >
                    <ConfiguredProviderProbe />
                </ArsProvider>
            }
        });

        leptos::task::tick().await;

        let wrapper = container
            .query_selector("[dir='ltr']")
            .expect("selector should be valid")
            .expect("provider wrapper should exist");

        assert_eq!(wrapper.get_attribute("dir").as_deref(), Some("ltr"));
        assert_eq!(
            container
                .query_selector("[data-testid='locale']")
                .expect("selector should be valid")
                .expect("locale node should exist")
                .text_content()
                .as_deref(),
            Some("ar-SA")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='direction']")
                .expect("selector should be valid")
                .expect("direction node should exist")
                .text_content()
                .as_deref(),
            Some("ltr")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='color-mode']")
                .expect("selector should be valid")
                .expect("color mode node should exist")
                .text_content()
                .as_deref(),
            Some("Dark")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='disabled']")
                .expect("selector should be valid")
                .expect("disabled node should exist")
                .text_content()
                .as_deref(),
            Some("true")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='read-only']")
                .expect("selector should be valid")
                .expect("read-only node should exist")
                .text_content()
                .as_deref(),
            Some("true")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='id-prefix']")
                .expect("selector should be valid")
                .expect("id-prefix node should exist")
                .text_content()
                .as_deref(),
            Some("app")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='portal-container-id']")
                .expect("selector should be valid")
                .expect("portal-container-id node should exist")
                .text_content()
                .as_deref(),
            Some("portal-root")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='root-node-id']")
                .expect("selector should be valid")
                .expect("root-node-id node should exist")
                .text_content()
                .as_deref(),
            Some("focus-root")
        );
        assert_eq!(
            container
                .query_selector("[data-testid='style-strategy']")
                .expect("selector should be valid")
                .expect("style-strategy node should exist")
                .text_content()
                .as_deref(),
            Some("Cssom")
        );

        drop(mount_handle);

        container.remove();
    }
}
