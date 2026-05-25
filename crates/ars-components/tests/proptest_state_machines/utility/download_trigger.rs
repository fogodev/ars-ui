use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    // ── DownloadTrigger ────────────────────────────────────────────

    /// `Api::part_attrs(Part::Root)` always equals `Api::root_attrs()`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_part_root_dispatch_equals_root_attrs(
        props in arb_download_trigger_props(),
    ) {
        let api = utility_core::download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            utility_core::download_trigger::Messages::default(),
        );

        prop_assert_eq!(
            api.part_attrs(utility_core::download_trigger::Part::Root),
            api.root_attrs()
        );
    }

    /// Root attrs always carry canonical scope/part data attributes.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_root_attrs_always_have_scope_and_part(
        props in arb_download_trigger_props(),
    ) {
        let attrs = utility_core::download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            utility_core::download_trigger::Messages::default(),
        )
        .root_attrs();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("download-trigger")
        );
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// Relative hrefs remain native-download eligible without `document_origin`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_relative_href_is_always_native_eligible(
        props in arb_download_trigger_props()
            .prop_filter("relative href", |p| {
                p.href.starts_with('/') && !p.href.starts_with("//")
            }),
    ) {
        let api = utility_core::download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            utility_core::download_trigger::Messages::default(),
        );

        prop_assert!(api.native_download_eligible());
        prop_assert!(!api.needs_blob_fallback());
        prop_assert!(api.root_attrs().contains(&HtmlAttr::Download));
    }

    /// Cross-origin HTTP(S) emits the fallback hook instead of `download`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_cross_origin_signals_blob_fallback(
        props in arb_download_trigger_cross_origin_props(),
    ) {
        let api = utility_core::download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            utility_core::download_trigger::Messages::default(),
        );

        prop_assert!(!api.native_download_eligible());
        prop_assert!(api.needs_blob_fallback());

        let attrs = api.root_attrs();

        prop_assert_eq!(attrs.get(&HtmlAttr::Download), None);
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-download-fallback")),
            Some(utility_core::download_trigger::DOWNLOAD_FALLBACK_REQUIRED)
        );
    }

    /// `Api::props()` round-trip for DownloadTrigger.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_api_props_round_trip(props in arb_download_trigger_props()) {
        let original = props.clone();

        let api = utility_core::download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            utility_core::download_trigger::Messages::default(),
        );

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.is_disabled(), original.disabled);
        prop_assert_eq!(api.filename(), original.filename.as_deref());
    }
}
