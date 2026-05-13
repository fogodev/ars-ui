use ars_collections::TabKey;

const PROFILE: u64 = 1;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(int = PROFILE)]
    Profile,
}

fn main() {}
