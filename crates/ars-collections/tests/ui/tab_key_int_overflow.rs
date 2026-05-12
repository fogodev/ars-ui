use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(int = 18446744073709551616)]
    Profile,
}

fn main() {}
