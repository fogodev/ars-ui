use ars_components::overlay::dialog;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};

use crate::helper::assert_anatomy;

#[test]
fn dialog_anatomy_matches_spec() {
    assert_anatomy(
        "dialog",
        &[
            (dialog::Part::Root, "root"),
            (dialog::Part::Trigger, "trigger"),
            (dialog::Part::Backdrop, "backdrop"),
            (dialog::Part::Positioner, "positioner"),
            (dialog::Part::Content, "content"),
            (dialog::Part::Title, "title"),
            (dialog::Part::Description, "description"),
            (dialog::Part::CloseTrigger, "close-trigger"),
        ],
    );
}

#[test]
fn dialog_content_attrs_carry_modal_role_and_label_refs() {
    let mut service = Service::<dialog::Machine>::new(
        dialog::Props::new()
            .id("dialog")
            .default_open(true)
            .role(dialog::Role::AlertDialog),
        &Env::default(),
        &dialog::Messages::default(),
    );

    drop(service.send(dialog::Event::RegisterTitle));
    drop(service.send(dialog::Event::RegisterDescription));

    let attrs = service.connect(&|_| {}).content_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Role), Some("alertdialog"));
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
        Some("dialog-title")
    );
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
        Some("dialog-description")
    );
}
