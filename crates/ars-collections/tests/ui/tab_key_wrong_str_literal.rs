use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(str = 1)]
    Profile,
}

fn main() {}
