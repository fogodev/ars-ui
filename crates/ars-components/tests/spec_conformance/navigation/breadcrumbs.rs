use ars_components::navigation::breadcrumbs;
use ars_core::SafeUrl;

use crate::helper::assert_anatomy;

#[test]
fn breadcrumbs_anatomy_matches_spec() {
    assert_anatomy(
        "breadcrumbs",
        &[
            (breadcrumbs::Part::Root, "root"),
            (breadcrumbs::Part::List, "list"),
            (breadcrumbs::Part::Item, "item"),
            (
                breadcrumbs::Part::Link {
                    href: SafeUrl::from_static("/"),
                },
                "link",
            ),
            (breadcrumbs::Part::CurrentPage, "current-page"),
            (breadcrumbs::Part::Separator, "separator"),
        ],
    );
}
