use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(discriminant)]
enum BadTab {
    Profile = 1,
    Billing = 1,
}

fn main() {}
