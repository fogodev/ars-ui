use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(ordinal)]
struct SettingsTab {
    profile: bool,
}

fn main() {}
