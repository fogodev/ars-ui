//! Adapter-only derive ergonomics for typed tabs.

use ars_leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
enum AdapterTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "billing")]
    Billing,
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
