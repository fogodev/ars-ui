//! Adapter-only derive ergonomics for typed tabs.

use ars_leptos::prelude::*;
#[cfg(feature = "uuid")]
use ars_leptos::uuid as adapter_uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum AdapterTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "billing")]
    Billing,
}

#[cfg(feature = "uuid")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum AdapterUuidTab {
    #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000001")]
    Profile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Translate)]
#[translate(fallback = "en")]
enum AdapterText {
    #[translate(en = "Profile", pt_BR = "Perfil")]
    Profile,
    #[translate(en = "{count} items", pt_BR = "{count} itens")]
    Count { count: usize },
}

#[test]
fn tab_key_trait_and_derive_are_available_from_leptos_prelude() {
    assert_eq!(AdapterTab::Profile.into_key(), Key::str("profile"));
    assert_eq!(AdapterTab::Billing.into_key(), Key::str("billing"));
}

#[cfg(feature = "uuid")]
#[test]
fn uuid_tab_key_derive_is_available_through_leptos_facade() {
    assert_eq!(
        AdapterUuidTab::Profile.into_key(),
        Key::uuid(
            <adapter_uuid::Uuid as core::str::FromStr>::from_str(
                "018f9b58-8f3d-7c8b-9d71-000000000001"
            )
            .expect("uuid literal should parse")
        )
    );
}

#[test]
fn translate_trait_and_derive_are_available_from_leptos_prelude() {
    fn assert_translate<T: Translate>() {}

    assert_translate::<AdapterText>();

    assert_eq!(
        AdapterText::Count { count: 2 },
        AdapterText::Count { count: 2 }
    );

    let _ = AdapterText::Profile;
}
