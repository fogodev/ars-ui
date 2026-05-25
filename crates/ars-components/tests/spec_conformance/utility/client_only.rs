#[test]
fn client_only_props_default_and_builder_match_spec() {
    let default_props = ars_components::utility::client_only::Props::<&str>::default();
    let new_props = ars_components::utility::client_only::Props::<&str>::new();
    let fallback_props = ars_components::utility::client_only::Props::new().fallback("Loading");

    assert_eq!(default_props.fallback, None);
    assert_eq!(new_props.fallback, None);
    assert_eq!(fallback_props.fallback, Some("Loading"));
}
