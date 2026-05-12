use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(stable)]
enum BadTab {
    Profile,
}

fn main() {}
