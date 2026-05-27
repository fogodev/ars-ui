use ars_components::input::search_input;

use crate::helper::assert_anatomy;

#[test]
fn search_input_anatomy_matches_spec() {
    assert_anatomy(
        "search-input",
        &[
            (search_input::Part::Root, "root"),
            (search_input::Part::Label, "label"),
            (search_input::Part::Input, "input"),
            (search_input::Part::ClearTrigger, "clear-trigger"),
            (search_input::Part::SubmitTrigger, "submit-trigger"),
            (search_input::Part::LoadingIndicator, "loading-indicator"),
            (search_input::Part::Description, "description"),
            (search_input::Part::ErrorMessage, "error-message"),
        ],
    );
}
