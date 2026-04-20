//! Locale fallback-chain utilities for message lookup.
//!
//! [`LocaleStack`] preserves a primary locale plus progressively less-specific
//! BCP 47 fallbacks, allowing adapters and registries to resolve locale-aware
//! resources without duplicating truncation logic.

use alloc::vec::Vec;
use core::iter;

use super::Locale;

/// A locale with a fallback chain.
///
/// The first entry is always the primary locale. Additional entries are derived
/// by truncating script and region subtags according to the canonical BCP 47
/// fallback order used by the spec.
#[derive(Clone, Debug)]
pub struct LocaleStack {
    locales: Vec<Locale>,
}

impl LocaleStack {
    /// Creates a locale stack from a primary locale.
    ///
    /// The generated chain follows this order:
    /// - full tag
    /// - language+script (when both script and region are present)
    /// - language-only (when script or region is present)
    #[must_use]
    pub fn new(primary: Locale) -> Self {
        let mut secondary_locale = None;

        if primary.region().is_some()
            && let Some(script) = primary.script()
        {
            let lang_script = alloc::format!("{}-{script}", primary.language());

            if let Ok(locale) = Locale::parse(&lang_script)
                && locale != primary
            {
                secondary_locale = Some(locale);
            }
        }

        let mut tertiary_locale = None;

        if (primary.region().is_some() || primary.script().is_some())
            && let Ok(locale) = Locale::parse(primary.language())
            && locale != primary
            && (secondary_locale.as_ref() != Some(&locale))
        {
            tertiary_locale = Some(locale);
        }

        Self {
            locales: iter::once(primary)
                .chain(secondary_locale)
                .chain(tertiary_locale)
                .collect(),
        }
    }

    /// Appends an explicit fallback locale to the chain.
    #[must_use]
    pub fn with_fallback(mut self, fallback: Locale) -> Self {
        self.locales.push(fallback);
        self
    }

    /// Returns the highest-priority locale in the chain.
    #[must_use]
    pub fn primary(&self) -> &Locale {
        &self.locales[0]
    }

    /// Iterates over locales from most specific to least specific.
    pub fn iter(&self) -> impl Iterator<Item = &Locale> {
        self.locales.iter()
    }

    /// Finds the first locale in the chain matching `predicate`.
    #[must_use]
    pub fn find<F>(&self, predicate: F) -> Option<&Locale>
    where
        F: Fn(&Locale) -> bool,
    {
        self.locales.iter().find(|locale| predicate(locale))
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use super::LocaleStack;
    use crate::Locale;

    #[test]
    fn locale_stack_builds_cjk_script_fallback_chain() {
        let stack = LocaleStack::new(Locale::parse("zh-Hant-TW").expect("locale should parse"));

        let tags = stack.iter().map(Locale::to_bcp47).collect::<Vec<_>>();

        assert_eq!(tags, vec!["zh-Hant-TW", "zh-Hant", "zh"]);
    }

    #[test]
    fn locale_stack_builds_region_fallback_chain() {
        let stack = LocaleStack::new(Locale::parse("pt-BR").expect("locale should parse"));

        let tags = stack.iter().map(Locale::to_bcp47).collect::<Vec<_>>();

        assert_eq!(tags, vec!["pt-BR", "pt"]);
    }

    #[test]
    fn locale_stack_builds_script_only_fallback_chain() {
        let stack = LocaleStack::new(Locale::parse("zh-Hant").expect("locale should parse"));

        let tags = stack.iter().map(Locale::to_bcp47).collect::<Vec<_>>();

        assert_eq!(tags, vec!["zh-Hant", "zh"]);
    }

    #[test]
    fn locale_stack_keeps_single_language_locale() {
        let stack = LocaleStack::new(Locale::parse("en").expect("locale should parse"));

        let tags = stack.iter().map(Locale::to_bcp47).collect::<Vec<_>>();

        assert_eq!(tags, vec!["en"]);
    }

    #[test]
    fn locale_stack_appends_explicit_fallback() {
        let stack = LocaleStack::new(Locale::parse("pt-BR").expect("locale should parse"))
            .with_fallback(Locale::parse("en").expect("locale should parse"));

        let tags = stack.iter().map(Locale::to_bcp47).collect::<Vec<_>>();

        assert_eq!(tags, vec!["pt-BR", "pt", "en"]);
    }

    #[test]
    fn locale_stack_primary_returns_first_locale() {
        let stack = LocaleStack::new(Locale::parse("de-DE").expect("locale should parse"));

        assert_eq!(stack.primary().to_bcp47(), "de-DE");
    }

    #[test]
    fn locale_stack_find_returns_first_matching_locale() {
        let stack = LocaleStack::new(Locale::parse("pt-BR").expect("locale should parse"))
            .with_fallback(Locale::parse("en-US").expect("locale should parse"));

        let matched = stack
            .find(|locale| locale.to_bcp47() == "pt")
            .expect("matching locale should be found");

        assert_eq!(matched.to_bcp47(), "pt");
    }
}
