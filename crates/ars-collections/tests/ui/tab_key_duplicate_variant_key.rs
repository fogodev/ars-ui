use ars_collections::TabKey;

#[derive(TabKey)]
enum SettingsTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(str = "profile")]
    Billing,
}

fn main() {}
