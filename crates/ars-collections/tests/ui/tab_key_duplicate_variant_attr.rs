use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(int = 1)]
    #[tab_key(int = 2)]
    Profile,
}

fn main() {}
