use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(ordinal)]
enum SettingsTab {
    Profile,
    Billing(u8),
}

fn main() {}
