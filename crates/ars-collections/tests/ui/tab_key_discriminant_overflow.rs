use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(discriminant)]
enum BadTab {
    Profile = 18446744073709551616,
}

fn main() {}
