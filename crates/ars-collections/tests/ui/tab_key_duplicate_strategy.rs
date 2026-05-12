use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(ordinal, discriminant)]
enum BadTab {
    Profile = 1,
}

fn main() {}
