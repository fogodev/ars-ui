use ars_components::overlay::toast::{manager as toast_manager, single as toast_single};
use ars_core::{AriaAttr, Env, HtmlAttr};

use crate::helper::assert_anatomy;

#[test]
fn toast_single_anatomy_matches_spec() {
    assert_anatomy(
        "toast",
        &[
            (toast_single::Part::Root, "root"),
            (toast_single::Part::Title, "title"),
            (toast_single::Part::Description, "description"),
            (
                toast_single::Part::ActionTrigger {
                    alt_text: String::new(),
                },
                "action-trigger",
            ),
            (toast_single::Part::CloseTrigger, "close-trigger"),
            (toast_single::Part::ProgressBar, "progress-bar"),
        ],
    );
}

#[test]
fn toast_manager_anatomy_matches_spec() {
    // Manager's enumerable Part is just `Root`. The polite/assertive
    // `aria-live` region shells stamp `data-ars-scope="toast"` (NOT
    // `toast-provider`) because they belong to the per-toast surface
    // conceptually. The region attrs are exercised by the manager snapshot
    // suite, not enumerated through `Part::all()`.
    assert_anatomy("toast-provider", &[(toast_manager::Part::Root, "root")]);
}

#[test]
fn toast_region_attrs_match_live_region_contract() {
    let messages = toast_manager::Messages::default();
    let env = Env::default();

    let polite =
        toast_manager::region_attrs(&messages, &env.locale, toast_manager::RegionPart::Polite);
    let assertive =
        toast_manager::region_attrs(&messages, &env.locale, toast_manager::RegionPart::Assertive);

    assert_eq!(polite.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
    assert_eq!(polite.get(&HtmlAttr::Role), Some("status"));
    assert_eq!(polite.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("false"));
    assert_eq!(
        assertive.get(&HtmlAttr::Aria(AriaAttr::Live)),
        Some("assertive")
    );
    assert_eq!(assertive.get(&HtmlAttr::Role), Some("alert"));
}
