use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(uuid = "not-a-uuid")]
    Profile,
}

fn main() {}
