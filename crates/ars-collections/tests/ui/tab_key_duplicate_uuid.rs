use ars_collections::TabKey;

#[derive(TabKey)]
enum BadTab {
    #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000000")]
    Profile,
    #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000000")]
    Billing,
}

fn main() {}
