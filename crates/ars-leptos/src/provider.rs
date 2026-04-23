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
    ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, PlatformEffects,
    StyleStrategy, resolve_messages as core_resolve_messages,
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

    use ars_core::{ColorMode, I18nRegistries, NullPlatformEffects, StyleStrategy};
    use ars_i18n::{
        Direction, IntlBackend, Locale, NumberFormatOptions, StubIntlBackend, Translate, locales,
    };
    use leptos::prelude::{Get, GetUntracked, Memo, Owner, RwSignal, Set, Signal};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        ArsContext, current_ars_context, resolve_locale, t, translated_text, use_intl_backend,
        use_locale, use_number_formatter,
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
}
