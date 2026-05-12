use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(slug = "profile")]
    Profile,
}

fn main() {}
