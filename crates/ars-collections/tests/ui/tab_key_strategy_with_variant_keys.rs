use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(ordinal)]
enum SettingsTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "billing")]
    Billing,
}

fn main() {}
