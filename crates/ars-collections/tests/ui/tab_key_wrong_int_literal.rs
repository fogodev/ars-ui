use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(int = "profile")]
    Profile,
}

fn main() {}
