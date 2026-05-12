//! Derive-contract coverage for typed tab identifiers.

use ars_collections::{Key, TabKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
#[tab_key(ordinal)]
enum SettingsTab {
    Profile,
    Billing,
    Security,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
#[tab_key(discriminant)]
enum StableTab {
    Overview = 10,
    Metrics = 20,
    Audit = 30,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum ExplicitIntTab {
    #[tab_key(int = 42)]
    Profile,
    #[tab_key(int = 77)]
    Billing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum ExplicitStrTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "billing")]
    Billing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
#[tab_key(crate = ars_collections)]
enum OverrideCrateTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "billing")]
    Billing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
#[tab_key(ordinal, crate = ars_collections)]
enum OverrideCrateOrdinalTab {
    Profile,
    Billing,
}

#[cfg(feature = "uuid")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum ExplicitUuidTab {
    #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000000")]
    Profile,
    #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000001")]
    Billing,
}

#[test]
fn tab_key_ordinal_strategy_uses_declaration_order_integer_keys() {
    assert_eq!(SettingsTab::Profile.into_key(), Key::int(0));
    assert_eq!(SettingsTab::Billing.into_key(), Key::int(1));
    assert_eq!(SettingsTab::Security.into_key(), Key::int(2));

    let key: Key = SettingsTab::Billing.into();

    assert_eq!(key, Key::int(1));
}

#[test]
fn tab_key_discriminant_strategy_uses_explicit_integer_keys() {
    assert_eq!(StableTab::Overview.into_key(), Key::int(10));
    assert_eq!(StableTab::Metrics.into_key(), Key::int(20));
    assert_eq!(StableTab::Audit.into_key(), Key::int(30));
}

#[test]
fn tab_key_variant_int_attributes_use_explicit_integer_keys() {
    assert_eq!(ExplicitIntTab::Profile.into_key(), Key::int(42));
    assert_eq!(ExplicitIntTab::Billing.into_key(), Key::int(77));
}

#[test]
fn tab_key_variant_str_attributes_use_explicit_string_keys() {
    assert_eq!(ExplicitStrTab::Profile.into_key(), Key::str("profile"));
    assert_eq!(ExplicitStrTab::Billing.into_key(), Key::str("billing"));
}

#[test]
fn tab_key_crate_override_uses_explicit_facade_for_variant_keys() {
    assert_eq!(OverrideCrateTab::Profile.into_key(), Key::str("profile"));
    assert_eq!(OverrideCrateTab::Billing.into_key(), Key::str("billing"));
}

#[test]
fn tab_key_crate_override_works_with_enum_level_strategy() {
    assert_eq!(OverrideCrateOrdinalTab::Profile.into_key(), Key::int(0));
    assert_eq!(OverrideCrateOrdinalTab::Billing.into_key(), Key::int(1));
}

#[cfg(feature = "uuid")]
#[test]
fn tab_key_variant_uuid_attributes_use_explicit_uuid_keys() {
    let profile = uuid::Uuid::parse_str("018f9b58-8f3d-7c8b-9d71-000000000000")
        .expect("test uuid should parse");
    let billing = uuid::Uuid::parse_str("018f9b58-8f3d-7c8b-9d71-000000000001")
        .expect("test uuid should parse");

    assert_eq!(ExplicitUuidTab::Profile.into_key(), Key::uuid(profile));
    assert_eq!(ExplicitUuidTab::Billing.into_key(), Key::uuid(billing));
}

#[test]
fn tab_key_derive_ui_tests() {
    let cases = trybuild::TestCases::new();

    cases.compile_fail("tests/ui/tab_key_*.rs");
}
