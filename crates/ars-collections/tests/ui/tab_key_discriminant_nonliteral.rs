use ars_collections::TabKey;

const PROFILE: u64 = 1;

#[derive(TabKey)]
#[tab_key(discriminant)]
#[repr(u64)]
enum BadTab {
    Profile = PROFILE,
}

fn main() {}
