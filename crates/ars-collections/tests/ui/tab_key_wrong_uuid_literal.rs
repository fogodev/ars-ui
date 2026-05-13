use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(uuid = 1)]
    Profile,
}

fn main() {}
