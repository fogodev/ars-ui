//! Component message registry infrastructure.
//!
//! Adapters resolve component-localized message bundles by combining optional
//! per-instance overrides with application-wide registries published from
//! `ArsProvider`.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
};
use core::{
    any::{Any, TypeId},
    fmt,
};

use ars_i18n::Locale;

use crate::ComponentMessages;

type RegistryValue = Box<dyn Any + Send + Sync>;

/// A registry of pre-built message sets for common locales.
#[derive(Clone, Debug, PartialEq)]
pub struct MessagesRegistry<M: ComponentMessages> {
    messages: BTreeMap<String, M>,
    default: M,
}

impl<M: ComponentMessages> MessagesRegistry<M> {
    /// Creates an empty registry with a built-in fallback bundle.
    #[must_use]
    pub const fn new(default: M) -> Self {
        Self {
            messages: BTreeMap::new(),
            default,
        }
    }

    /// Registers a message bundle for `locale_tag`.
    #[must_use]
    pub fn register(mut self, locale_tag: &str, messages: M) -> Self {
        self.messages.insert(locale_tag.to_string(), messages);
        self
    }

    /// Retrieves messages for `locale`, following the locale-tag fallback chain.
    #[must_use]
    pub fn get(&self, locale: &Locale) -> &M {
        if let Some(messages) = self.messages.get(&locale.to_bcp47()) {
            return messages;
        }

        if let Some(script) = locale.script() {
            let lang_script = alloc::format!("{}-{script}", locale.language());
            if let Some(messages) = self.messages.get(&lang_script) {
                return messages;
            }
        }

        if let Some(messages) = self.messages.get(locale.language()) {
            return messages;
        }

        &self.default
    }
}

/// Type-erased storage for per-component message registries.
///
/// Each registry is keyed by the concrete `TypeId` of a component's `Messages`
/// type so adapters can resolve localized message bundles without coupling
/// components to one another.
#[derive(Default)]
pub struct I18nRegistries {
    map: BTreeMap<TypeId, RegistryValue>,
}

impl I18nRegistries {
    /// Creates an empty registry collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Registers a `MessagesRegistry` for the given component messages type.
    pub fn register<M: ComponentMessages + Send + Sync + 'static>(
        &mut self,
        registry: MessagesRegistry<M>,
    ) {
        self.map.insert(TypeId::of::<M>(), Box::new(registry));
    }

    /// Looks up the `MessagesRegistry` for `M`, if one is registered.
    #[must_use]
    pub fn get<M: ComponentMessages + Send + Sync + 'static>(
        &self,
    ) -> Option<&MessagesRegistry<M>> {
        self.map
            .get(&TypeId::of::<M>())
            .and_then(|value| value.downcast_ref::<MessagesRegistry<M>>())
    }
}

impl fmt::Debug for I18nRegistries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I18nRegistries")
            .field("registries", &self.map.len())
            .finish()
    }
}

/// Resolves component messages using prop override, provider registries, then defaults.
#[must_use]
pub fn resolve_messages<M: ComponentMessages + Send + Sync + 'static>(
    adapter_props_messages: Option<&M>,
    registries: &I18nRegistries,
    locale: &Locale,
) -> M {
    if let Some(messages) = adapter_props_messages {
        return messages.clone();
    }

    if let Some(registry) = registries.get::<M>() {
        return registry.get(locale).clone();
    }

    M::default()
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use ars_i18n::Locale;

    use super::{I18nRegistries, MessagesRegistry, resolve_messages};
    use crate::{ComponentMessages, MessageFn};

    #[derive(Clone, Debug, PartialEq)]
    struct DialogMessages {
        close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for DialogMessages {
        fn default() -> Self {
            Self {
                close_label: MessageFn::static_str("Close"),
            }
        }
    }

    impl ComponentMessages for DialogMessages {}

    #[derive(Clone, Debug, PartialEq)]
    struct SelectMessages {
        trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for SelectMessages {
        fn default() -> Self {
            Self {
                trigger_label: MessageFn::static_str("Open"),
            }
        }
    }

    impl ComponentMessages for SelectMessages {}

    #[test]
    fn messages_registry_resolves_script_level_fallback() {
        let registry = MessagesRegistry::new(DialogMessages::default()).register(
            "zh-Hant",
            DialogMessages {
                close_label: MessageFn::static_str("關閉"),
            },
        );

        let locale = Locale::parse("zh-Hant-TW").expect("locale should parse");
        assert_eq!((registry.get(&locale).close_label)(&locale), "關閉");
    }

    #[test]
    fn messages_registry_falls_back_to_default_without_match() {
        let registry = MessagesRegistry::new(DialogMessages::default()).register(
            "fr",
            DialogMessages {
                close_label: MessageFn::static_str("Fermer"),
            },
        );

        let locale = Locale::parse("ja-JP").expect("locale should parse");
        assert_eq!((registry.get(&locale).close_label)(&locale), "Close");
    }

    #[test]
    fn i18n_registries_store_multiple_message_types() {
        let mut registries = I18nRegistries::new();
        registries.register(MessagesRegistry::new(DialogMessages::default()).register(
            "es",
            DialogMessages {
                close_label: MessageFn::static_str("Cerrar"),
            },
        ));
        registries.register(MessagesRegistry::new(SelectMessages::default()).register(
            "es",
            SelectMessages {
                trigger_label: MessageFn::static_str("Abrir"),
            },
        ));

        let locale = Locale::parse("es-ES").expect("locale should parse");
        let dialog = registries
            .get::<DialogMessages>()
            .expect("dialog registry should exist");
        let select = registries
            .get::<SelectMessages>()
            .expect("select registry should exist");

        assert_eq!((dialog.get(&locale).close_label)(&locale), "Cerrar");
        assert_eq!((select.get(&locale).trigger_label)(&locale), "Abrir");
    }

    #[test]
    fn resolve_messages_prefers_prop_override() {
        let mut registries = I18nRegistries::new();
        registries.register(MessagesRegistry::new(DialogMessages::default()).register(
            "es",
            DialogMessages {
                close_label: MessageFn::static_str("Cerrar"),
            },
        ));

        let locale = Locale::parse("es-ES").expect("locale should parse");
        let override_messages = DialogMessages {
            close_label: MessageFn::static_str("Override"),
        };

        let resolved = resolve_messages(Some(&override_messages), &registries, &locale);
        assert_eq!((resolved.close_label)(&locale), "Override");
    }

    #[test]
    fn resolve_messages_uses_registry_when_present() {
        let mut registries = I18nRegistries::new();
        registries.register(MessagesRegistry::new(DialogMessages::default()).register(
            "es",
            DialogMessages {
                close_label: MessageFn::static_str("Cerrar"),
            },
        ));

        let locale = Locale::parse("es-MX").expect("locale should parse");
        let resolved = resolve_messages::<DialogMessages>(None, &registries, &locale);

        assert_eq!((resolved.close_label)(&locale), "Cerrar");
    }

    #[test]
    fn resolve_messages_falls_back_to_default() {
        let registries = I18nRegistries::new();
        let locale = Locale::parse("de-DE").expect("locale should parse");

        let resolved = resolve_messages::<DialogMessages>(None, &registries, &locale);
        assert_eq!((resolved.close_label)(&locale), "Close");
    }
}
