//! Procedural derive macros for ars-ui component infrastructure.
//!
//! Provides `#[derive(HasId)]`, `#[derive(ComponentPart)]`,
//! `#[derive(TabKey)]`, and `#[derive(Translate)]` to generate boilerplate
//! implementations for component ID access, DOM part enums, typed tab
//! identifiers, and application translation enums.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod component_part;
mod has_id;
mod tab_key;
mod translate;

/// Derives the `HasId` trait for a struct with a `pub id: String` field.
///
/// Generates `id()`, `with_id()`, and `set_id()` methods for typed access
/// to the component's DOM identifier. Generated code uses hidden
/// `::ars_core::__private` re-exports so downstream crates do not need to
/// import `alloc` just to use the derive.
///
/// # Required input shape
///
/// - The input must be a struct.
/// - The struct must contain a public field named `id`.
/// - The `id` field must have type `String`.
/// - The derive has no helper attributes.
///
/// # Example
///
/// ```rust,ignore
/// use ars_core::HasId;
///
/// #[derive(Clone, Debug, HasId)]
/// pub struct ButtonIds {
///     pub id: String,
/// }
///
/// let ids = ButtonIds { id: "save".into() };
/// assert_eq!(ids.id(), "save");
/// ```
#[proc_macro_derive(HasId)]
pub fn derive_has_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match has_id::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives the `ars_core::ComponentPart` trait for a part enum.
///
/// Generates `ROOT`, `scope()`, `name()`, and `all()` methods. Use
/// `#[scope = "component-name"]` on the enum to set the component namespace
/// for data attribute generation. Generated code uses hidden
/// `::ars_core::__private` re-exports so downstream crates do not need to
/// import `alloc` just to use the derive.
///
/// # Required input shape
///
/// - The input must be an enum.
/// - The enum must have exactly one `#[scope = "..."]` helper attribute.
/// - The enum's first variant must be a unit variant. That variant becomes
///   `ars_core::ComponentPart::ROOT`.
/// - Unit, tuple, and struct variants are supported. Variants with fields
///   receive generated `Clone`, `Debug`, `PartialEq`, `Eq`, and `Hash`
///   implementations with the necessary field bounds.
/// - Fields may use `#[part(default = expr)]` to control the value used for
///   that field in `ComponentPart::all()`. Fields without that annotation use
///   `Default::default()` and therefore require a `Default` bound.
///
/// # Generated names
///
/// Variant names are converted to kebab-case for `name()`, so `CloseTrigger`
/// becomes `"close-trigger"`. `scope()` returns the exact string from
/// `#[scope = "..."]`.
///
/// # Example
///
/// ```rust,ignore
/// use ars_core::ComponentPart;
///
/// #[derive(ComponentPart)]
/// #[scope = "tabs"]
/// pub enum TabsPart {
///     Root,
///     List,
///     Tab,
///     CloseTrigger,
///     Panel {
///         #[part(default = String::from("example-panel"))]
///         id: String,
///     },
/// }
///
/// assert_eq!(TabsPart::scope(), "tabs");
/// assert_eq!(TabsPart::CloseTrigger.name(), "close-trigger");
/// ```
#[proc_macro_derive(ComponentPart, attributes(scope, part))]
pub fn derive_component_part(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match component_part::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives the `TabKey` trait for a fieldless enum.
///
/// The derive intentionally requires explicit keys, either through an
/// enum-level `#[tab_key(...)]` strategy or through per-variant
/// `#[tab_key(... = ...)]` attributes. Declaration order is never used
/// unless the enum opts into `#[tab_key(ordinal)]`.
///
/// Generated paths resolve through the public facade crate that the consuming
/// package depends on: `ars-collections`, `ars-leptos`, or `ars-dioxus`.
/// Normal adapter consumers do not need a direct `ars-collections`
/// dependency. Use `#[tab_key(crate = some_path)]` only when the dependency is
/// renamed or both adapters are direct dependencies and the intended facade is
/// ambiguous.
///
/// # Required input shape
///
/// - The input must be an enum.
/// - Every variant must be a unit variant with no fields.
/// - The enum must satisfy the `ars_collections::TabKey` supertrait
///   bounds: `Copy + Eq + Ord + Send + Sync + 'static`. In practice, derive
///   `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, and `Ord` on the enum.
/// - Either one enum-level `#[tab_key(ordinal)]` /
///   `#[tab_key(discriminant)]` strategy is required, or every variant
///   must have one per-variant `#[tab_key(int = ...)]`,
///   `#[tab_key(str = "...")]`, or `#[tab_key(uuid = "...")]` key.
///
/// # Enum-level strategy attributes
///
/// Use `#[tab_key(ordinal)]` when the keys are local UI identities and are
/// not persisted or shared across releases. Variants map to zero-based integer
/// keys in declaration order.
///
/// ```rust,ignore
/// use ars_collections::{Key, TabKey};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// #[tab_key(ordinal)]
/// enum SettingsTab {
///     General,
///     Privacy,
///     Billing,
/// }
///
/// assert_eq!(SettingsTab::General.into_key(), Key::int(0));
/// assert_eq!(SettingsTab::Privacy.into_key(), Key::int(1));
/// ```
///
/// Use `#[tab_key(discriminant)]` when the key values must remain stable
/// independently of declaration order. Every variant must have an explicit,
/// unique, non-negative integer literal discriminant that fits in `u64`.
///
/// ```rust,ignore
/// use ars_collections::{Key, TabKey};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// #[tab_key(discriminant)]
/// enum ProductTab {
///     Overview = 10,
///     Metrics = 20,
///     Settings = 30,
/// }
///
/// assert_eq!(ProductTab::Metrics.into_key(), Key::int(20));
/// ```
///
/// Add `crate = ...` to the enum-level attribute only when the generated
/// implementation should use a specific facade path:
///
/// ```rust,ignore
/// use ars_leptos::prelude::*;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// #[tab_key(ordinal, crate = ars_leptos)]
/// enum SettingsTab {
///     Profile,
///     Billing,
/// }
/// ```
///
/// # Per-variant key attributes
///
/// Use per-variant attributes when the enum should carry stable key values
/// directly. Every variant must be annotated, all variants in one enum must
/// use the same key kind, and all keys must be unique.
///
/// ```rust,ignore
/// use ars_collections::{Key, TabKey};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// enum SettingsTab {
///     #[tab_key(str = "profile")]
///     Profile,
///     #[tab_key(str = "billing")]
///     Billing,
/// }
///
/// assert_eq!(SettingsTab::Profile.into_key(), Key::str("profile"));
/// ```
///
/// Integer keys use non-negative integer literals that fit in `u64`:
///
/// ```rust,ignore
/// use ars_collections::{Key, TabKey};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// enum SettingsTab {
///     #[tab_key(int = 42)]
///     Profile,
///     #[tab_key(int = 77)]
///     Billing,
/// }
///
/// assert_eq!(SettingsTab::Profile.into_key(), Key::int(42));
/// ```
///
/// UUID keys use canonical UUID string literals and require the
/// `ars-collections` `uuid` feature in the consuming crate:
///
/// ```rust,ignore
/// use ars_collections::{Key, TabKey};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
/// enum SettingsTab {
///     #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000000")]
///     Profile,
///     #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000001")]
///     Billing,
/// }
/// ```
///
/// # Generated implementations
///
/// The derive implements:
///
/// - `TabKey` from the resolved facade crate for the enum.
/// - `From<YourEnum> for Key` from the resolved facade crate.
///
/// That means consumers can pass the enum directly to generic tab adapter
/// props, while adapter internals still use the framework-agnostic `Key`.
///
/// # Compile-time validation
///
/// These cases are rejected with targeted compile errors:
///
/// - deriving on a struct or union;
/// - missing enum-level strategy and missing per-variant keys;
/// - using an unknown strategy;
/// - specifying more than one `#[tab_key(...)]` attribute;
/// - variants with payload fields;
/// - `#[tab_key(discriminant)]` variants without explicit integer literals;
/// - negative, non-literal, duplicate, or out-of-range discriminants;
/// - mixing enum-level strategies with per-variant keys;
/// - ambiguous facade resolution when both adapters are direct dependencies;
/// - per-variant keys that are partial, duplicated, malformed, or mixed
///   across `int`, `str`, and `uuid` kinds.
#[proc_macro_derive(TabKey, attributes(tab_key))]
pub fn derive_tab_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match tab_key::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives the `Translate` trait for an application text enum.
///
/// The derive covers static unit variants and named-field interpolation. It is
/// intended for straightforward labels and messages; implement `Translate`
/// manually for plural rules, number/date/currency formatting, gender/select
/// rules, or other ICU-style message behavior.
///
/// Generated paths resolve through the public facade crate that the consuming
/// package depends on: `ars-i18n`, `ars-leptos`, or `ars-dioxus`. Normal
/// adapter consumers do not need a direct `ars-i18n` dependency. Use
/// `#[translate(crate = some_path)]` only when the dependency is renamed or
/// both adapters are direct dependencies and the intended facade is ambiguous.
///
/// # Required input shape
///
/// - The input must be an enum.
/// - The enum must declare `#[translate(fallback = "...")]`.
/// - Every variant must declare a message for the fallback locale.
/// - Unit variants and named-field variants are supported.
/// - Tuple variants are rejected so placeholders stay self-documenting.
///
/// # Locale attributes
///
/// Locale identifiers can use Rust identifiers with `_` in place of BCP 47
/// `-` separators. The derive normalizes `pt_BR` to `pt-BR` and `en_US` to
/// `en-US`.
///
/// ```rust,ignore
/// use ars_i18n::Translate;
///
/// #[derive(Clone, Debug, Translate)]
/// #[translate(fallback = "en")]
/// enum SettingsText {
///     #[translate(en = "Profile", pt_BR = "Perfil")]
///     Profile,
/// }
/// ```
///
/// For locale tags that are awkward as Rust identifiers, use the explicit
/// `locale = "...", text = "..."` form:
///
/// ```rust,ignore
/// use ars_i18n::Translate;
///
/// #[derive(Clone, Debug, Translate)]
/// #[translate(fallback = "en")]
/// enum SettingsText {
///     #[translate(locale = "sr-Latn-RS", text = "Profil")]
///     #[translate(en = "Profile")]
///     Profile,
/// }
/// ```
///
/// # Interpolation
///
/// Named-field variants may reference fields with `{field}` placeholders.
/// Placeholder values are formatted with `Display`.
///
/// ```rust,ignore
/// use ars_i18n::Translate;
///
/// #[derive(Clone, Debug, Translate)]
/// #[translate(fallback = "en")]
/// enum InventoryText {
///     #[translate(en = "{count} items", pt_BR = "{count} itens")]
///     ItemCount { count: usize },
/// }
/// ```
///
/// This is interpolation only: the generated code does not choose plural
/// categories or format numbers with locale-specific rules. Implement
/// `Translate` manually for those cases.
#[proc_macro_derive(Translate, attributes(translate))]
pub fn derive_translate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match translate::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
