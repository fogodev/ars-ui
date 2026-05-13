use ars_collections::TabKey;

#[derive(TabKey)]
enum SettingsTab {
    #[tab_key(str = "profile")]
    Profile,
    #[tab_key(int = 42)]
    Billing,
}

fn main() {}
