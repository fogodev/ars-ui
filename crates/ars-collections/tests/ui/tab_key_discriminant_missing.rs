use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(discriminant)]
enum SettingsTab {
    Profile = 1,
    Billing,
}

fn main() {}
