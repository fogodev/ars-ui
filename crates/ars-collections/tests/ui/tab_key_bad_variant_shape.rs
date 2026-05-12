use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(int)]
    Profile,
}

fn main() {}
