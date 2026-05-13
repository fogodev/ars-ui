use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(crate = ars_collections, crate = ars_collections)]
#[tab_key(ordinal)]
enum BadTab {
    Profile,
}

fn main() {}
