use ars_collections::TabKey;

#[derive(TabKey)]
#[tab_key(discriminant)]
#[repr(u8)]
enum BadTab {
    Profile = b'p',
}

fn main() {}
